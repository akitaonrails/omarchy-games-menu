use std::fs;
use std::path::Path;

fn first_version_token(content: &str) -> Option<String> {
    let bytes: Vec<&str> = content.split_whitespace().collect();
    for token in bytes {
        let token = token.trim_matches(|c: char| c == 'v' || c == 'V');
        let mut parts = token.split('.');
        let all_numeric = parts
            .clone()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
        if all_numeric && parts.by_ref().count() >= 2 {
            return Some(token.to_string());
        }
    }
    None
}

/// Best-effort installed-version read: first whitespace token matching
/// `v?\d+(\.\d+)+`, else the whole trimmed file content when non-empty.
pub fn read_installed_version(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    if let Some(token) = first_version_token(&content) {
        return Some(token);
    }
    let trimmed = content.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_first_version_like_token() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("version");
        fs::write(&f, "release v9.2.3 build\n").unwrap();
        assert_eq!(read_installed_version(&f).as_deref(), Some("9.2.3"));
    }

    #[test]
    fn falls_back_to_trimmed_content() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("version");
        fs::write(&f, "  nightly-2026-09-01  \n").unwrap();
        assert_eq!(
            read_installed_version(&f).as_deref(),
            Some("nightly-2026-09-01")
        );
    }

    #[test]
    fn missing_or_empty_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_installed_version(&dir.path().join("nope")).is_none());
        let f = dir.path().join("empty");
        fs::write(&f, "   \n").unwrap();
        assert!(read_installed_version(&f).is_none());
    }
}
