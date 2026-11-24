//! Local certificate authority and on-demand site certificates.
//!
//! The CA is *name-constrained* to `.test` and `.localhost`: even if its key
//! leaked, certificates it signs for any real domain would be rejected by
//! browsers. Site certificates are minted in memory on the first TLS
//! handshake for a name (via SNI) and cached for the daemon's lifetime.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use locdev_core::host::{self, Target};
use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, ExtendedKeyUsagePurpose,
    GeneralSubtree, IsCa, Issuer, KeyPair, KeyUsagePurpose, NameConstraints,
};
use rustls::ServerConfig;
use rustls::crypto::ring as provider;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use time::{Duration, OffsetDateTime};
use tracing::{debug, info, warn};

/// Common-name prefix of every LocDev CA. Uninstall removes certificates by
/// this name, so it must stay stable.
pub const CA_NAME: &str = "LocDev Local CA";

const CA_YEARS: i64 = 10;
/// Under Apple's 398-day limit for TLS server certificates.
const LEAF_DAYS: i64 = 397;
const CACHE_LIMIT: usize = 256;

#[derive(Debug)]
pub struct TlsError(pub String);

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TlsError {}

impl From<rcgen::Error> for TlsError {
    fn from(e: rcgen::Error) -> Self {
        Self(format!("certificate error: {e}"))
    }
}

impl From<std::io::Error> for TlsError {
    fn from(e: std::io::Error) -> Self {
        Self(format!("i/o error: {e}"))
    }
}

pub struct Authority {
    issuer: Issuer<'static, KeyPair>,
    der: CertificateDer<'static>,
    dir: PathBuf,
}

impl Authority {
    /// Loads the CA from `dir`, creating it on first use.
    pub fn load_or_create(dir: &Path) -> Result<Self, TlsError> {
        let cert_path = dir.join("ca.pem");
        let key_path = dir.join("ca-key.pem");

        if cert_path.exists() && key_path.exists() {
            let cert_pem = std::fs::read_to_string(&cert_path)?;
            let key = KeyPair::from_pem(&std::fs::read_to_string(&key_path)?)?;
            let der = pem_to_der(&cert_pem)?;
            let issuer = Issuer::from_ca_cert_pem(&cert_pem, key)?;
            return Ok(Self {
                issuer,
                der,
                dir: dir.to_path_buf(),
            });
        }

        create_private_dir(dir)?;
        let key = KeyPair::generate()?;
        let params = ca_params();
        let cert = params.self_signed(&key)?;

        write_private(&key_path, key.serialize_pem().as_bytes())?;
        std::fs::write(&cert_path, cert.pem())?;
        // DER copy for tools that want it (Windows Import-Certificate).
        std::fs::write(dir.join("ca.cer"), cert.der())?;
        info!(path = %cert_path.display(), "created local certificate authority");

        Ok(Self {
            der: cert.der().clone(),
            issuer: Issuer::new(params, key),
            dir: dir.to_path_buf(),
        })
    }

    pub fn cert_path(&self) -> PathBuf {
        self.dir.join("ca.pem")
    }

    pub fn der_path(&self) -> PathBuf {
        self.dir.join("ca.cer")
    }

    pub fn der(&self) -> &[u8] {
        self.der.as_ref()
    }

    /// SHA-256 of the CA certificate, as `AB:CD:…`.
    pub fn fingerprint(&self) -> String {
        let digest = ring::digest::digest(&ring::digest::SHA256, self.der());
        let mut out = String::with_capacity(95);
        for (i, byte) in digest.as_ref().iter().enumerate() {
            if i > 0 {
                out.push(':');
            }
            let _ = write!(out, "{byte:02X}");
        }
        out
    }

    /// Whether the OS trust store (what curl, Safari, Edge, … use) contains
    /// this CA.
    pub fn trusted_by_system(&self) -> bool {
        let found = rustls_native_certs::load_native_certs();
        found.certs.iter().any(|c| c.as_ref() == self.der())
    }

    /// Issues a certificate for one site: `name.test`, `*.name.test` and the
    /// `.localhost` equivalents. `None` issues one for plain `localhost`.
    pub fn issue(&self, site: Option<&str>) -> Result<CertifiedKey, TlsError> {
        self.issue_names(Self::site_names(site))
    }

    /// The site's names plus `*.<parent>` for a deeper SNI: a wildcard covers
    /// one label only, so `a.tenants.shop.test` needs `*.tenants.shop.test`.
    pub fn issue_for(&self, site: &str, deeper: Option<&str>) -> Result<CertifiedKey, TlsError> {
        let mut names = Self::site_names(Some(site));
        if let Some(parent) = deeper {
            names.push(format!("*.{parent}"));
        }
        self.issue_names(names)
    }

    /// Writes a certificate and key for `site` into `dir` (`<site>.pem`,
    /// `<site>-key.pem`, the key readable only by this user) for dev tools
    /// that serve HTTPS themselves. Files younger than `REUSE_DAYS` are reused.
    pub fn issue_files(&self, site: &str, dir: &Path) -> Result<(PathBuf, PathBuf), TlsError> {
        const REUSE_DAYS: u64 = 30;
        let cert_path = dir.join(format!("{site}.pem"));
        let key_path = dir.join(format!("{site}-key.pem"));
        let fresh = |path: &Path| {
            std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age.as_secs() < REUSE_DAYS * 86_400)
        };
        if fresh(&cert_path) && fresh(&key_path) {
            return Ok((cert_path, key_path));
        }

        std::fs::create_dir_all(dir)?;
        let (cert, key) = self.sign(Self::site_names(Some(site)))?;
        write_private(&key_path, key.serialize_pem().as_bytes())?;
        std::fs::write(&cert_path, cert.pem())?;
        Ok((cert_path, key_path))
    }

    fn site_names(site: Option<&str>) -> Vec<String> {
        match site {
            Some(name) => vec![
                format!("{name}.{}", locdev_core::TLD),
                format!("*.{name}.{}", locdev_core::TLD),
                format!("{name}.localhost"),
                format!("*.{name}.localhost"),
            ],
            None => vec!["localhost".to_string()],
        }
    }

    fn issue_names(&self, names: Vec<String>) -> Result<CertifiedKey, TlsError> {
        let (cert, key) = self.sign(names.clone())?;
        let private = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
        let signing_key = provider::sign::any_supported_type(&private)
            .map_err(|e| TlsError(format!("unusable key: {e}")))?;
        debug!(?names, "issued site certificate");
        Ok(CertifiedKey::new(vec![cert.der().clone()], signing_key))
    }

    fn sign(&self, names: Vec<String>) -> Result<(rcgen::Certificate, KeyPair), TlsError> {
        let mut params = CertificateParams::new(names.clone())?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, names[0].clone());
        params.distinguished_name = dn;
        let now = OffsetDateTime::now_utc();
        params.not_before = now - Duration::days(1);
        params.not_after = now + Duration::days(LEAF_DAYS);
        params.is_ca = IsCa::ExplicitNoCa;
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        params.use_authority_key_identifier_extension = true;

        let key = KeyPair::generate()?;
        let cert = params.signed_by(&key, &self.issuer)?;
        Ok((cert, key))
    }
}

fn ca_params() -> CertificateParams {
    let mut params = CertificateParams::default();

    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, CA_NAME);
    dn.push(DnType::OrganizationName, "LocDev");
    // Tells several machines' CAs apart in a trust store.
    dn.push(DnType::OrganizationalUnitName, owner());
    params.distinguished_name = dn;

    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::days(1);
    params.not_after = now + Duration::days(365 * CA_YEARS);
    params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    params.name_constraints = Some(NameConstraints {
        permitted_subtrees: vec![
            GeneralSubtree::DnsName(locdev_core::TLD.to_string()),
            GeneralSubtree::DnsName("localhost".to_string()),
        ],
        excluded_subtrees: Vec::new(),
    });
    params
}

fn owner() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "user".into());
    let host = std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "localhost".into());
    format!("{user}@{host}")
}

fn pem_to_der(pem: &str) -> Result<CertificateDer<'static>, TlsError> {
    let body: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .map(str::trim)
        .collect();
    decode_base64(&body)
        .map(CertificateDer::from)
        .ok_or_else(|| TlsError("ca.pem is not a valid certificate".into()))
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let bytes = input.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n = 0u32;
        for (i, b) in chunk.iter().enumerate() {
            n |= value(*b)? << (18 - 6 * i);
        }
        let take = chunk.len().saturating_sub(1);
        out.extend_from_slice(&n.to_be_bytes()[1..1 + take]);
    }
    Some(out)
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_private(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(contents)
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, contents)
    }
}

/// Picks (or mints) the certificate for each handshake from its SNI.
pub struct SiteCerts {
    authority: Arc<Authority>,
    cache: Mutex<HashMap<String, Arc<CertifiedKey>>>,
}

impl std::fmt::Debug for SiteCerts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SiteCerts").finish_non_exhaustive()
    }
}

impl SiteCerts {
    pub fn new(authority: Arc<Authority>) -> Arc<Self> {
        Arc::new(Self {
            authority,
            cache: Mutex::new(HashMap::new()),
        })
    }

    /// `parent` is set for SNIs two or more labels below the site.
    fn for_site(&self, site: Option<&str>, parent: Option<&str>) -> Option<Arc<CertifiedKey>> {
        let key = parent.or(site).unwrap_or("").to_string();
        if let Some(found) = self.cache.lock().unwrap().get(&key) {
            return Some(found.clone());
        }

        let issued = match site {
            Some(name) => self.authority.issue_for(name, parent),
            None => self.authority.issue(None),
        };
        match issued {
            Ok(certified) => {
                let certified = Arc::new(certified);
                let mut cache = self.cache.lock().unwrap();
                if cache.len() >= CACHE_LIMIT {
                    cache.clear();
                }
                cache.insert(key, certified.clone());
                Some(certified)
            }
            Err(error) => {
                warn!(%error, ?site, "could not issue certificate");
                None
            }
        }
    }
}

/// The names the certificate for `sni` covers (what `SiteCerts` issues).
pub fn names_for(sni: &str) -> Vec<String> {
    let sni = host::strip_port(sni);
    match host::target(sni) {
        Target::Site { name, sub, .. } => {
            let mut names = Authority::site_names(Some(name));
            if sub.contains('.')
                && let Some((_, parent)) = sni.split_once('.')
            {
                names.push(format!("*.{parent}"));
            }
            names
        }
        _ => Authority::site_names(None),
    }
}

impl ResolvesServerCert for SiteCerts {
    fn resolve(&self, hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let sni = hello.server_name().map(str::to_ascii_lowercase);
        let sni = sni.as_deref().map(host::strip_port);
        match sni.map(|s| (s, host::target(s))) {
            Some((full, Target::Site { name, sub, .. })) => {
                // `a.tenants.shop.test` → parent `tenants.shop.test`.
                let parent = sub
                    .contains('.')
                    .then(|| full.split_once('.').map(|(_, p)| p))
                    .flatten();
                self.for_site(Some(name), parent)
            }
            // No SNI (e.g. https://127.0.0.1) or plain localhost.
            _ => self.for_site(None, None),
        }
    }
}

/// A TLS server config that answers every local name. Only HTTP/1.1 is
/// offered so WebSocket upgrades (hot reload) keep working through the proxy.
pub fn server_config(certs: Arc<SiteCerts>) -> Result<Arc<ServerConfig>, TlsError> {
    let mut config = ServerConfig::builder_with_provider(Arc::new(provider::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| TlsError(e.to_string()))?
        .with_no_client_auth()
        .with_cert_resolver(certs);
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(Arc::new(config))
}

#[cfg(test)]
mod tests {
    #[test]
    fn writes_site_files_with_a_private_key() {
        let dir = tempfile::tempdir().unwrap();
        let ca = Authority::load_or_create(&dir.path().join("ca")).unwrap();
        let out = dir.path().join("sites");
        let (cert, key) = ca.issue_files("blog", &out).unwrap();
        let pem = std::fs::read_to_string(&cert).unwrap();
        assert!(pem.starts_with("-----BEGIN CERTIFICATE-----"));
        assert!(
            std::fs::read_to_string(&key)
                .unwrap()
                .contains("PRIVATE KEY")
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&key).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        // Reused while fresh.
        assert_eq!(ca.issue_files("blog", &out).unwrap(), (cert.clone(), key));
        assert_eq!(std::fs::read_to_string(&cert).unwrap(), pem);
    }

    use super::*;

    #[test]
    fn creates_then_reloads_the_same_ca() {
        let dir = tempfile::tempdir().unwrap();
        let ca_dir = dir.path().join("tls");

        let first = Authority::load_or_create(&ca_dir).unwrap();
        let second = Authority::load_or_create(&ca_dir).unwrap();
        assert_eq!(first.fingerprint(), second.fingerprint());
        assert_eq!(first.fingerprint().len(), 95);
        assert_eq!(std::fs::read(first.der_path()).unwrap(), first.der());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(ca_dir.join("ca-key.pem"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        // A reloaded CA still signs certificates.
        assert!(second.issue(Some("blog")).is_ok());
    }

    fn verify(ca: &Authority, leaf: &CertifiedKey, name: &str) -> Result<(), rustls::Error> {
        use rustls::client::WebPkiServerVerifier;
        use rustls::client::danger::ServerCertVerifier;
        use rustls::pki_types::{ServerName, UnixTime};

        let mut roots = rustls::RootCertStore::empty();
        roots.add(CertificateDer::from(ca.der().to_vec())).unwrap();
        let verifier = WebPkiServerVerifier::builder_with_provider(
            Arc::new(roots),
            Arc::new(provider::default_provider()),
        )
        .build()
        .unwrap();
        verifier
            .verify_server_cert(
                &leaf.cert[0],
                &[],
                &ServerName::try_from(name.to_string()).unwrap(),
                &[],
                UnixTime::now(),
            )
            .map(|_| ())
    }

    #[test]
    fn deeper_subdomains_get_their_own_wildcard() {
        let dir = tempfile::tempdir().unwrap();
        let ca = Arc::new(Authority::load_or_create(dir.path()).unwrap());
        let plain = ca.issue(Some("shop")).unwrap();
        // One label only: the site's wildcard doesn't cover two levels.
        assert!(verify(&ca, &plain, "acme.tenants.shop.test").is_err());

        let certs = SiteCerts::new(ca.clone());
        let deep = certs
            .for_site(Some("shop"), Some("tenants.shop.test"))
            .unwrap();
        for name in [
            "acme.tenants.shop.test",
            "other.tenants.shop.test",
            "shop.test",
            "api.shop.test",
        ] {
            verify(&ca, &deep, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        assert!(verify(&ca, &deep, "a.b.tenants.shop.test").is_err());
        // Cached per parent.
        assert!(Arc::ptr_eq(
            &deep,
            &certs
                .for_site(Some("shop"), Some("tenants.shop.test"))
                .unwrap()
        ));
    }

    #[test]
    fn site_certificates_verify_for_their_names() {
        let dir = tempfile::tempdir().unwrap();
        let ca = Authority::load_or_create(dir.path()).unwrap();
        let leaf = ca.issue(Some("blog")).unwrap();

        for name in [
            "blog.test",
            "api.blog.test",
            "blog.localhost",
            "api.blog.localhost",
        ] {
            verify(&ca, &leaf, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        }
        assert!(verify(&ca, &leaf, "shop.test").is_err());
        assert!(verify(&ca, &ca.issue(None).unwrap(), "localhost").is_ok());
    }

    #[test]
    fn name_constraints_stop_real_domains() {
        // Even a certificate the CA itself signs for a public domain must be
        // rejected: this is what makes trusting the CA low-risk.
        let dir = tempfile::tempdir().unwrap();
        let ca = Authority::load_or_create(dir.path()).unwrap();
        let rogue = ca.issue_names(vec!["example.com".into()]).unwrap();

        let error = verify(&ca, &rogue, "example.com").unwrap_err();
        assert!(format!("{error:?}").contains("NameConstraint"), "{error:?}");
    }

    #[test]
    fn base64_round_trip() {
        assert_eq!(decode_base64("Zm9vYmFy").unwrap(), b"foobar");
        assert_eq!(decode_base64("Zm9vYg==").unwrap(), b"foob");
        assert!(decode_base64("Zm9v!").is_none());
    }
}
