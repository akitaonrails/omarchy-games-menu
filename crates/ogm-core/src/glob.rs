/// Simple stem matcher: a trailing `*` makes the pattern a prefix,
/// anything else is an exact match. Case-sensitive on purpose: desktop
/// file stems are.
pub fn matches_glob(stem: &str, pattern: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => stem.starts_with(prefix),
        None => stem == pattern,
    }
}

pub fn matches_any(stem: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| matches_glob(stem, p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_and_exact() {
        assert!(matches_glob("gaming-ship-of-harkinian", "gaming-*"));
        assert!(!matches_glob("pc-outrun-2006", "gaming-*"));
        assert!(matches_glob("screamer", "screamer"));
        assert!(!matches_glob("screamer2", "screamer"));
        assert!(!matches_glob("gaming.desktop", "gaming-*"));
    }

    #[test]
    fn matches_any_uses_list() {
        let pats = vec!["gaming-*".to_string(), "screamer".to_string()];
        assert!(matches_any("gaming-x", &pats));
        assert!(matches_any("screamer", &pats));
        assert!(!matches_any("screamer2", &pats));
        assert!(!matches_any("other", &pats));
    }
}
