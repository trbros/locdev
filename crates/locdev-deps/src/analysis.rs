//! Turns what the project declares, what the registry publishes and known
//! advisories into a report per package, and judges an update. The verdict
//! is never "guaranteed": "compatible" only means no check found a conflict
//! (engines, peers, PHP), and a new major is at best "possibly".

use std::collections::HashMap;

use locdev_core::deps::{
    Advisory, Compatibility, DepStatus, Ecosystem, PackageLinks, PackageReport, UpdateKind,
};
use locdev_core::semver::{self, Range, Version};

use crate::local::Declared;
use crate::registry::{PackageInfo, VersionInfo};

/// What the project runs with, for compatibility checks.
#[derive(Debug, Default)]
pub struct Context {
    /// The site's resolved Node.js version.
    pub node: Option<String>,
    /// The site's resolved PHP version.
    pub php: Option<String>,
    /// Current version of every declared package (for peer checks).
    pub current: HashMap<String, String>,
}

fn parse_version(ecosystem: Ecosystem, raw: &str) -> Option<Version> {
    match ecosystem {
        Ecosystem::Npm => Version::parse(raw),
        Ecosystem::Composer => Version::parse_composer(raw),
    }
}

fn parse_range(ecosystem: Ecosystem, raw: &str) -> Option<Range> {
    match ecosystem {
        Ecosystem::Npm => semver::parse_npm(raw).ok(),
        Ecosystem::Composer => semver::parse_composer(raw).ok(),
    }
}

/// Stable releases only: betas, RCs and nightlies are never proposed.
fn stable(info: &PackageInfo) -> Vec<(Version, &VersionInfo)> {
    let mut out: Vec<_> = info
        .versions
        .iter()
        .filter_map(|v| Some((parse_version(info.ecosystem, &v.version)?, v)))
        .filter(|(v, _)| !v.is_prerelease())
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// The registry's latest when it is stable, else the newest stable.
fn latest(info: &PackageInfo) -> Option<(Version, &VersionInfo)> {
    let all = stable(info);
    if let Some(tag) = &info.latest_tag
        && let Some(found) = all.iter().find(|(_, v)| &v.version == tag)
    {
        return Some(found.clone());
    }
    all.last().cloned()
}

fn wanted<'a>(dep: &Declared, info: &'a PackageInfo) -> Option<(Version, &'a VersionInfo)> {
    let range = parse_range(dep.ecosystem, &dep.constraint)?;
    stable(info)
        .into_iter()
        .rev()
        .find(|(v, _)| range.matches(v))
}

pub fn update_kind(from: &Version, to: &Version) -> UpdateKind {
    if to <= from {
        UpdateKind::None
    } else if to.major() != from.major() {
        UpdateKind::Major
    } else if to.minor() != from.minor() {
        UpdateKind::Minor
    } else {
        UpdateKind::Patch
    }
}

/// The version a project uses now: installed, else locked.
pub fn current(dep: &Declared) -> Option<&str> {
    dep.installed.as_deref().or(dep.locked.as_deref())
}

pub fn report(
    dep: &Declared,
    info: Option<&PackageInfo>,
    advisories: Vec<Advisory>,
    ctx: &Context,
) -> PackageReport {
    let links = info
        .map(|i| i.links.clone())
        .unwrap_or_else(|| PackageLinks {
            registry: registry_page(dep.ecosystem, &dep.name),
            ..Default::default()
        });
    let mut report = PackageReport {
        name: dep.name.clone(),
        ecosystem: dep.ecosystem,
        kind: dep.kind,
        declared: dep.constraint.clone(),
        locked: dep.locked.clone(),
        installed: dep.installed.clone(),
        wanted: None,
        latest: None,
        update: UpdateKind::None,
        status: DepStatus::Unknown,
        compatibility: Compatibility::Unknown,
        reasons: Vec::new(),
        advisories,
        links,
        deprecated: None,
    };

    if parse_range(dep.ecosystem, &dep.constraint).is_none() {
        report.reasons.push(format!(
            "'{}' isn't a registry version range (git, path or workspace)",
            dep.constraint
        ));
    }
    let now = current(dep).and_then(|c| parse_version(dep.ecosystem, c));
    if now.is_none() {
        report.reasons.push("not installed or locked yet".into());
    }

    if let Some(info) = info {
        report.wanted = wanted(dep, info).map(|(v, _)| v.to_string());
        if let Some((latest_version, latest_info)) = latest(info) {
            report.latest = Some(latest_info.version.clone());
            report.deprecated = info
                .versions
                .iter()
                .find(|v| Some(v.version.as_str()) == current(dep))
                .and_then(|v| v.deprecated.clone());
            if let Some(now) = &now {
                report.update = update_kind(now, &latest_version);
                report.status = if report.update == UpdateKind::None {
                    DepStatus::UpToDate
                } else {
                    DepStatus::Outdated
                };
                if report.update != UpdateKind::None {
                    let (compatibility, reasons) =
                        judge(dep, now, &latest_version, latest_info, ctx);
                    report.compatibility = compatibility;
                    report.reasons.extend(reasons);
                } else {
                    report.compatibility = Compatibility::Compatible;
                }
            }
        }
    } else {
        report
            .reasons
            .push("not checked against the registry yet".into());
    }

    if !report.advisories.is_empty() {
        report.status = DepStatus::Vulnerable;
        for advisory in &report.advisories {
            if let Some(fixed) = &advisory.fixed {
                report
                    .reasons
                    .push(format!("{} is fixed in {fixed}", advisory.id));
            }
        }
    }
    report
}

/// The verdict on moving from `from` to `to` (published as `candidate`).
pub fn judge(
    dep: &Declared,
    from: &Version,
    to: &Version,
    candidate: &VersionInfo,
    ctx: &Context,
) -> (Compatibility, Vec<String>) {
    let mut verdict = Compatibility::Compatible;
    let mut reasons = Vec::new();
    let mut lower = |level: Compatibility| {
        verdict = match (verdict, level) {
            (Compatibility::Incompatible, _) | (_, Compatibility::Incompatible) => {
                Compatibility::Incompatible
            }
            (Compatibility::Possibly, _) | (_, Compatibility::Possibly) => Compatibility::Possibly,
            (Compatibility::Unknown, _) | (_, Compatibility::Unknown) => Compatibility::Unknown,
            _ => Compatibility::Compatible,
        }
    };

    match parse_range(dep.ecosystem, &dep.constraint) {
        Some(range) if range.matches(to) => {
            reasons.push(format!("within your constraint {}", dep.constraint))
        }
        Some(_) => reasons.push(format!(
            "outside your constraint {}; the manifest would change",
            dep.constraint
        )),
        None => {}
    }

    let breaking = to.major() != from.major() || (from.major() == 0 && to.minor() != from.minor());
    if breaking {
        lower(Compatibility::Possibly);
        reasons.push(if from.major() == 0 {
            "0.x minor release: breaking changes are allowed; read the release notes".into()
        } else {
            format!(
                "major release ({} → {}): breaking changes are possible; read the release notes",
                from.major(),
                to.major()
            )
        });
    }

    if let Some(node_range) = &candidate.node {
        match (&ctx.node, semver::parse_npm(node_range)) {
            (Some(node), Ok(range)) => {
                if Version::parse(node).is_some_and(|v| !range.matches(&v)) {
                    lower(Compatibility::Incompatible);
                    reasons.push(format!("needs Node.js {node_range}; the site runs {node}"));
                }
            }
            (None, _) => {
                lower(Compatibility::Possibly);
                reasons.push(format!(
                    "needs Node.js {node_range}; the site's Node.js version is unknown"
                ));
            }
            (_, Err(_)) => {}
        }
    }
    if let Some(php_range) = &candidate.php {
        match (&ctx.php, semver::parse_composer(php_range)) {
            (Some(php), Ok(range)) => {
                if Version::parse_composer(php).is_some_and(|v| !range.matches(&v)) {
                    lower(Compatibility::Incompatible);
                    reasons.push(format!("needs PHP {php_range}; the site runs {php}"));
                }
            }
            (None, _) => {
                lower(Compatibility::Possibly);
                reasons.push(format!(
                    "needs PHP {php_range}; the site's PHP version is unknown"
                ));
            }
            (_, Err(_)) => {}
        }
    }
    for (peer, range, optional) in &candidate.peers {
        let Ok(parsed) = semver::parse_npm(range) else {
            continue;
        };
        match ctx.current.get(peer).and_then(|v| Version::parse(v)) {
            Some(have) if !parsed.matches(&have) => {
                lower(Compatibility::Incompatible);
                reasons.push(format!("needs {peer} {range}; the project has {have}"));
            }
            Some(_) => {}
            None if !optional => {
                lower(Compatibility::Possibly);
                reasons.push(format!(
                    "expects {peer} {range}, which the project doesn't declare"
                ));
            }
            None => {}
        }
    }
    if let Some(note) = &candidate.deprecated {
        lower(Compatibility::Possibly);
        reasons.push(format!("{} is deprecated: {note}", candidate.version));
    }
    (verdict, reasons)
}

/// The version to move to: the newest in the declared range, or (with
/// `allow_major`) the newest stable. `None` when there is nothing newer.
pub fn target<'a>(
    dep: &Declared,
    info: &'a PackageInfo,
    allow_major: bool,
) -> Option<(Version, &'a VersionInfo)> {
    let current = current(dep).and_then(|c| parse_version(dep.ecosystem, c))?;
    let candidate = if allow_major {
        latest(info)
    } else {
        wanted(dep, info)
    }?;
    (candidate.0 > current).then_some(candidate)
}

pub fn registry_page(ecosystem: Ecosystem, name: &str) -> String {
    match ecosystem {
        Ecosystem::Npm => format!("https://www.npmjs.com/package/{name}"),
        Ecosystem::Composer => format!("https://packagist.org/packages/{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use locdev_core::deps::DepKind;

    fn dep(name: &str, constraint: &str, installed: &str) -> Declared {
        Declared {
            name: name.into(),
            ecosystem: Ecosystem::Npm,
            kind: DepKind::Prod,
            constraint: constraint.into(),
            locked: None,
            installed: Some(installed.into()),
        }
    }

    /// (version, engines.node, peers)
    type Published<'a> = (&'a str, Option<&'a str>, &'a [(&'a str, &'a str)]);

    fn info(versions: &[Published]) -> PackageInfo {
        PackageInfo {
            name: "lib".into(),
            ecosystem: Ecosystem::Npm,
            latest_tag: None,
            links: PackageLinks::default(),
            versions: versions
                .iter()
                .map(|(v, node, peers)| VersionInfo {
                    version: v.to_string(),
                    node: node.map(str::to_string),
                    peers: peers
                        .iter()
                        .map(|(p, r)| (p.to_string(), r.to_string(), false))
                        .collect(),
                    ..Default::default()
                })
                .collect(),
        }
    }

    fn ctx(node: Option<&str>, current: &[(&str, &str)]) -> Context {
        Context {
            node: node.map(str::to_string),
            php: None,
            current: current
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        }
    }

    #[test]
    fn never_proposes_prereleases() {
        let info = info(&[
            ("1.0.0", None, &[]),
            ("1.1.0", None, &[]),
            ("2.0.0-beta.1", None, &[]),
        ]);
        let report = report(
            &dep("lib", "^1.0.0", "1.0.0"),
            Some(&info),
            vec![],
            &ctx(Some("22.0.0"), &[]),
        );
        assert_eq!(report.latest.as_deref(), Some("1.1.0"));
        assert_eq!(report.wanted.as_deref(), Some("1.1.0"));
        assert_eq!(
            (report.status, report.update),
            (DepStatus::Outdated, UpdateKind::Minor)
        );
        assert_eq!(report.compatibility, Compatibility::Compatible);
        assert!(
            report
                .reasons
                .iter()
                .any(|r| r.contains("within your constraint"))
        );
    }

    #[test]
    fn a_major_is_never_better_than_possibly() {
        let info = info(&[("1.0.0", None, &[]), ("2.0.0", None, &[])]);
        let report = report(
            &dep("lib", "^1.0.0", "1.0.0"),
            Some(&info),
            vec![],
            &ctx(Some("22.0.0"), &[]),
        );
        assert_eq!(report.update, UpdateKind::Major);
        assert_eq!(report.compatibility, Compatibility::Possibly);
        assert_eq!(report.wanted.as_deref(), Some("1.0.0"));
    }

    #[test]
    fn engines_and_peers_can_rule_out_an_update() {
        let info = info(&[
            ("1.0.0", None, &[]),
            ("1.2.0", Some(">=24"), &[("react", "^19.0.0")]),
        ]);
        let report = report(
            &dep("lib", "^1.0.0", "1.0.0"),
            Some(&info),
            vec![],
            &ctx(Some("22.11.0"), &[("react", "18.3.1")]),
        );
        assert_eq!(report.compatibility, Compatibility::Incompatible);
        assert!(
            report
                .reasons
                .iter()
                .any(|r| r.contains("needs Node.js >=24; the site runs 22.11.0"))
        );
        assert!(
            report
                .reasons
                .iter()
                .any(|r| r.contains("needs react ^19.0.0; the project has 18.3.1"))
        );
    }

    #[test]
    fn unknown_runtime_lowers_confidence() {
        let info = info(&[("1.0.0", None, &[]), ("1.0.1", Some(">=18"), &[])]);
        let report = report(
            &dep("lib", "^1.0.0", "1.0.0"),
            Some(&info),
            vec![],
            &ctx(None, &[]),
        );
        assert_eq!(report.compatibility, Compatibility::Possibly);
    }

    #[test]
    fn zero_minor_counts_as_breaking() {
        let info = info(&[("0.3.0", None, &[]), ("0.4.0", None, &[])]);
        let report = report(
            &dep("lib", "^0.3.0", "0.3.0"),
            Some(&info),
            vec![],
            &ctx(Some("22.0.0"), &[]),
        );
        assert_eq!(report.compatibility, Compatibility::Possibly);
    }

    #[test]
    fn advisories_mark_vulnerable() {
        let info = info(&[("1.0.0", None, &[])]);
        let advisory = Advisory {
            id: "GHSA-x".into(),
            summary: "bad".into(),
            severity: Some("HIGH".into()),
            url: String::new(),
            fixed: Some("1.0.1".into()),
        };
        let report = report(
            &dep("lib", "^1.0.0", "1.0.0"),
            Some(&info),
            vec![advisory],
            &ctx(None, &[]),
        );
        assert_eq!(report.status, DepStatus::Vulnerable);
        assert!(
            report
                .reasons
                .iter()
                .any(|r| r == "GHSA-x is fixed in 1.0.1")
        );
    }

    #[test]
    fn picks_targets_by_policy() {
        let info = info(&[
            ("1.0.0", None, &[]),
            ("1.4.0", None, &[]),
            ("2.1.0", None, &[]),
        ]);
        let d = dep("lib", "^1.0.0", "1.0.0");
        assert_eq!(target(&d, &info, false).unwrap().1.version, "1.4.0");
        assert_eq!(target(&d, &info, true).unwrap().1.version, "2.1.0");
        let up_to_date = dep("lib", "^2.0.0", "2.1.0");
        assert!(target(&up_to_date, &info, true).is_none());
    }

    #[test]
    fn non_registry_constraints_stay_unknown() {
        let report = report(
            &dep("lib", "workspace:*", "1.0.0"),
            None,
            vec![],
            &ctx(None, &[]),
        );
        assert_eq!(report.status, DepStatus::Unknown);
        assert!(
            report
                .reasons
                .iter()
                .any(|r| r.contains("isn't a registry version range"))
        );
    }
}
