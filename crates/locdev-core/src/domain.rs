use std::fmt;

use crate::TLD;

#[derive(Debug, PartialEq, Eq)]
pub enum DomainError {
    Empty,
    Invalid,
    TooLong,
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "project name is empty"),
            Self::Invalid => write!(
                f,
                "project name may only contain letters, digits and '-', and must start and end with a letter or digit"
            ),
            Self::TooLong => write!(f, "project name is longer than 63 characters"),
        }
    }
}

impl std::error::Error for DomainError {}

/// Normalizes and validates a project name so it can be used as a DNS label.
pub fn project_name(name: &str) -> Result<String, DomainError> {
    let normalized = name.trim().replace('_', "-").to_ascii_lowercase();

    if normalized.is_empty() {
        return Err(DomainError::Empty);
    }

    if normalized.len() > 63 {
        return Err(DomainError::TooLong);
    }

    let bytes = normalized.as_bytes();

    let valid_edges = bytes.first().is_some_and(|b| b.is_ascii_alphanumeric())
        && bytes.last().is_some_and(|b| b.is_ascii_alphanumeric());

    let valid_characters = bytes
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'-');

    if !valid_edges || !valid_characters {
        return Err(DomainError::Invalid);
    }

    Ok(normalized)
}

pub fn project_domain(name: &str) -> Result<String, DomainError> {
    project_name(name).map(|name| format!("{name}.{TLD}"))
}

/// Best-effort conversion of an arbitrary folder name into a valid project name.
pub fn slugify(raw: &str) -> String {
    let mut slug = String::with_capacity(raw.len());

    for c in raw.trim().chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }

    let slug = slug.trim_matches('-');
    slug.chars()
        .take(63)
        .collect::<String>()
        .trim_end_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_name() {
        assert_eq!(
            project_domain("Lyrius_cloud"),
            Ok("lyrius-cloud.test".to_string())
        );
    }

    #[test]
    fn reject_empty_name() {
        assert_eq!(project_domain(" "), Err(DomainError::Empty));
    }

    #[test]
    fn reject_invalid_paths() {
        assert_eq!(project_domain("../project"), Err(DomainError::Invalid));
    }

    #[test]
    fn reject_long_name() {
        let name = "a".repeat(64);

        assert_eq!(project_domain(&name), Err(DomainError::TooLong));
    }

    #[test]
    fn slugify_folder_names() {
        assert_eq!(slugify("My App (v2)"), "my-app-v2");
        assert_eq!(slugify("__site.local__"), "site-local");
        assert_eq!(slugify("..."), "");
        assert!(project_name(&slugify(&"x".repeat(80))).is_ok());
    }
}
