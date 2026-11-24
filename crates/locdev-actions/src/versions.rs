//! The actions templates use, and the version each one is at.
//!
//! The snapshot below was taken from each repository's latest release on
//! 2026-11-02; `refresh` replaces it with what GitHub reports now.

use std::collections::BTreeMap;

use locdev_core::actions::ActionVersion;

/// `(repo, version, purpose)`.
pub const SNAPSHOT: &[(&str, &str, &str)] = &[
    ("actions/checkout", "v7", "Check out the repository"),
    (
        "actions/setup-node",
        "v7",
        "Install Node.js (with dependency caching)",
    ),
    ("actions/cache", "v6", "Cache files between runs"),
    (
        "actions/upload-artifact",
        "v7",
        "Keep build output from a job",
    ),
    (
        "actions/download-artifact",
        "v8",
        "Use another job's build output",
    ),
    ("actions/setup-python", "v7", "Install Python"),
    (
        "actions/configure-pages",
        "v6",
        "Prepare a GitHub Pages deployment",
    ),
    (
        "actions/upload-pages-artifact",
        "v5",
        "Upload the site for GitHub Pages",
    ),
    ("actions/deploy-pages", "v5", "Publish to GitHub Pages"),
    (
        "actions/dependency-review-action",
        "v5",
        "Block pull requests adding vulnerable dependencies",
    ),
    (
        "actions/attest-build-provenance",
        "v4",
        "Sign build provenance",
    ),
    ("github/codeql-action", "v4", "CodeQL code scanning"),
    (
        "shivammathur/setup-php",
        "v2",
        "Install PHP, extensions and Composer",
    ),
    ("pnpm/action-setup", "v6", "Install pnpm"),
    ("oven-sh/setup-bun", "v2", "Install Bun"),
    (
        "softprops/action-gh-release",
        "v3",
        "Create a GitHub release",
    ),
    ("docker/setup-buildx-action", "v4", "Set up Docker Buildx"),
    (
        "docker/login-action",
        "v4",
        "Log in to a container registry",
    ),
    (
        "docker/build-push-action",
        "v7",
        "Build and push a Docker image",
    ),
];

/// Known versions: the snapshot, overridden by a refresh.
#[derive(Debug, Clone)]
pub struct Versions(BTreeMap<String, String>);

impl Default for Versions {
    fn default() -> Self {
        Self(
            SNAPSHOT
                .iter()
                .map(|(repo, version, _)| (repo.to_string(), version.to_string()))
                .collect(),
        )
    }
}

impl Versions {
    /// The snapshot with `refreshed` (repo → major tag) applied on top.
    pub fn with(refreshed: &BTreeMap<String, String>) -> Self {
        let mut versions = Self::default();
        for (repo, version) in refreshed {
            if versions.0.contains_key(repo) {
                versions.0.insert(repo.clone(), version.clone());
            }
        }
        versions
    }

    /// `owner/repo@version` for a template.
    pub fn uses(&self, repo: &str) -> String {
        let version = self.0.get(repo).map(String::as_str).unwrap_or("main");
        format!("{repo}@{version}")
    }

    pub fn get(&self, repo: &str) -> Option<&str> {
        self.0.get(repo).map(String::as_str)
    }

    pub fn list(&self) -> Vec<ActionVersion> {
        SNAPSHOT
            .iter()
            .map(|(repo, _, purpose)| ActionVersion {
                repo: repo.to_string(),
                version: self.0[*repo].clone(),
                purpose: purpose.to_string(),
            })
            .collect()
    }
}

/// `v7.0.1` → `v7`, `2.40.0` → `v2`. `None` for tags that aren't versions.
pub fn major_tag(tag: &str) -> Option<String> {
    let digits = tag.trim_start_matches(['v', 'V']);
    let major = digits.split('.').next()?;
    (!major.is_empty() && major.chars().all(|c| c.is_ascii_digit())).then(|| format!("v{major}"))
}

/// The major of a `uses:` ref (`v4`, `v4.1.0`), for comparing.
pub fn major_number(reference: &str) -> Option<u32> {
    major_tag(reference)?.trim_start_matches('v').parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn majors() {
        assert_eq!(major_tag("v7.0.1").as_deref(), Some("v7"));
        assert_eq!(major_tag("2.40.0").as_deref(), Some("v2"));
        assert_eq!(major_tag("codeql-bundle-v2.27.3"), None);
        assert_eq!(major_number("v4"), Some(4));
        assert_eq!(major_number("main"), None);
    }

    #[test]
    fn refresh_only_touches_known_actions() {
        let refreshed = BTreeMap::from([
            ("actions/checkout".to_string(), "v8".to_string()),
            ("evil/action".to_string(), "v1".to_string()),
        ]);
        let versions = Versions::with(&refreshed);
        assert_eq!(versions.uses("actions/checkout"), "actions/checkout@v8");
        assert_eq!(versions.get("evil/action"), None);
    }
}
