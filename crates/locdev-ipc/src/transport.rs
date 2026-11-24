use std::fmt;
use std::io;

use locdev_core::paths::Paths;

pub use imp::{ClientStream, Endpoint, Listener, ServerStream};

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.address())
    }
}

/// Stable across builds (unlike `DefaultHasher`), so every client derives the
/// same endpoint as the daemon.
fn fnv1a(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0x811c_9dc5, |hash, b| {
        (hash ^ u32::from(*b)).wrapping_mul(0x0100_0193)
    })
}

fn in_use() -> io::Error {
    io::Error::new(
        io::ErrorKind::AddrInUse,
        "another LocDev daemon is already listening",
    )
}

#[cfg(unix)]
mod imp {
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;

    use tokio::net::{UnixListener, UnixStream};

    use super::*;

    pub type ClientStream = UnixStream;
    pub type ServerStream = UnixStream;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Endpoint {
        path: PathBuf,
    }

    /// `sun_path` is 104 bytes on macOS and 108 on Linux, NUL included.
    const MAX_SOCKET_PATH: usize = 100;

    impl Endpoint {
        pub fn for_paths(paths: &Paths) -> Self {
            let path = paths.runtime_dir.join("locdevd.sock");
            if path.as_os_str().len() < MAX_SOCKET_PATH {
                return Self { path };
            }

            // Deep homes (e.g. a long LOCDEV_HOME) would overflow sun_path;
            // fall back to a short private directory in the temp dir.
            let home = fnv1a(paths.home.as_os_str().as_encoded_bytes());
            Self {
                path: std::env::temp_dir()
                    .join(format!("locdev-{home:08x}"))
                    .join("locdevd.sock"),
            }
        }

        pub fn address(&self) -> String {
            self.path.display().to_string()
        }

        pub async fn connect(&self) -> io::Result<ClientStream> {
            UnixStream::connect(&self.path).await
        }
    }

    pub struct Listener {
        inner: UnixListener,
        path: PathBuf,
    }

    impl Listener {
        pub async fn bind(endpoint: &Endpoint) -> io::Result<Self> {
            let path = endpoint.path.clone();
            if let Some(dir) = path.parent() {
                ensure_private_dir(dir)?;
            }

            if path.exists() {
                if UnixStream::connect(&path).await.is_ok() {
                    return Err(in_use());
                }
                // Left behind by a daemon that did not shut down cleanly.
                std::fs::remove_file(&path)?;
            }

            let inner = UnixListener::bind(&path)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;

            Ok(Self { inner, path })
        }

        pub async fn accept(&mut self) -> io::Result<ServerStream> {
            Ok(self.inner.accept().await?.0)
        }
    }

    /// The socket's directory must belong to us and be closed to everyone
    /// else; otherwise another user could plant a socket and impersonate the
    /// daemon.
    fn ensure_private_dir(dir: &std::path::Path) -> io::Result<()> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};

        match std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)
        {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }

        let meta = std::fs::metadata(dir)?;
        let ours = nix::unistd::getuid().as_raw();

        if meta.uid() != ours || meta.mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("{} must be owned by you and have mode 0700", dir.display()),
            ));
        }
        Ok(())
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(windows)]
mod imp {
    use std::time::Duration;

    use tokio::net::windows::named_pipe::{
        ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions,
    };

    use super::*;

    const ERROR_ACCESS_DENIED: i32 = 5;
    const ERROR_PIPE_BUSY: i32 = 231;

    pub type ClientStream = NamedPipeClient;
    pub type ServerStream = NamedPipeServer;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Endpoint {
        name: String,
    }

    impl Endpoint {
        /// The pipe name is scoped to the user and to the LocDev home, so
        /// isolated homes (`LOCDEV_HOME`) get isolated daemons.
        pub fn for_paths(paths: &Paths) -> Self {
            let user: String = std::env::var("USERNAME")
                .unwrap_or_default()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            let home = fnv1a(paths.home.to_string_lossy().as_bytes());

            Self {
                name: format!(r"\\.\pipe\locdev-{user}-{home:08x}"),
            }
        }

        pub fn address(&self) -> String {
            self.name.clone()
        }

        pub async fn connect(&self) -> io::Result<ClientStream> {
            loop {
                match ClientOptions::new().open(&self.name) {
                    Ok(client) => return Ok(client),
                    Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                        tokio::time::sleep(Duration::from_millis(20)).await;
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }

    pub struct Listener {
        name: String,
        next: NamedPipeServer,
    }

    impl Listener {
        pub async fn bind(endpoint: &Endpoint) -> io::Result<Self> {
            let next = ServerOptions::new()
                .first_pipe_instance(true)
                .reject_remote_clients(true)
                .create(&endpoint.name)
                .map_err(|e| {
                    if e.raw_os_error() == Some(ERROR_ACCESS_DENIED) {
                        in_use()
                    } else {
                        e
                    }
                })?;

            Ok(Self {
                name: endpoint.name.clone(),
                next,
            })
        }

        pub async fn accept(&mut self) -> io::Result<ServerStream> {
            self.next.connect().await?;
            // Create the next instance before handing this one out, so there
            // is always a pipe for the next client to open.
            let next = ServerOptions::new()
                .reject_remote_clients(true)
                .create(&self.name)?;
            Ok(std::mem::replace(&mut self.next, next))
        }
    }
}
