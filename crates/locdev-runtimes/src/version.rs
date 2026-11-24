//! Comparing and matching runtime versions ("22.11.0", "v22.11.0", "8.3.14").

use std::cmp::Ordering;

/// Numeric components, ignoring a leading `v` and any pre-release suffix.
pub fn parts(version: &str) -> Vec<u64> {
    let version = version.trim().trim_start_matches('v');
    let core = version.split(['-', '+']).next().unwrap_or_default();
    core.split('.')
        .map_while(|part| part.parse().ok())
        .collect()
}

pub fn compare(a: &str, b: &str) -> Ordering {
    parts(a).cmp(&parts(b))
}

/// Whether `version` satisfies `pin`, where the pin may be partial:
/// "22" matches every 22.x.y, "22.11" every 22.11.y, "22.11.0" only itself.
pub fn matches(pin: &str, version: &str) -> bool {
    locdev_core::model::RuntimeVersion::matches(pin, version)
}

/// The newest of `versions` that satisfies `pin`.
pub fn best<'a, T>(
    pin: &str,
    versions: impl IntoIterator<Item = &'a T>,
    version_of: impl Fn(&T) -> &str,
) -> Option<&'a T>
where
    T: 'a,
{
    versions
        .into_iter()
        .filter(|v| matches(pin, version_of(v)))
        .max_by(|a, b| compare(version_of(a), version_of(b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_pins() {
        assert!(matches("22", "22.11.0"));
        assert!(matches("v22.11", "22.11.3"));
        assert!(matches("8.3", "8.3.14"));
        assert!(!matches("8.3", "8.30.1"));
        assert!(!matches("22.11.0", "22.11"));
        assert!(!matches("", "22.1.0"));
        assert!(!matches("lts", "22.1.0"));
    }

    #[test]
    fn ordering_is_numeric() {
        assert_eq!(compare("8.10.0", "8.9.9"), Ordering::Greater);
        assert_eq!(compare("v22.2.0", "22.10.0"), Ordering::Less);
        assert_eq!(compare("8.4.27-1", "8.4.27"), Ordering::Equal);
    }

    #[test]
    fn picks_newest_match() {
        let all = ["20.1.0", "22.3.0", "22.11.0", "22.9.4"];
        assert_eq!(best("22", &all, |v| v), Some(&"22.11.0"));
        assert_eq!(best("21", &all, |v| v), None);
    }
}
