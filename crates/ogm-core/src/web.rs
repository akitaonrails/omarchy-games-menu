use sha2::{Digest, Sha256};

use crate::model::WebInfo;

/// Apply `X-OGM-UpdateRegex`: capture group 1 is the version string; a
/// pattern without groups uses the whole match. None when the pattern is
/// invalid or matches nothing.
pub fn extract_version(body: &str, pattern: &str) -> Option<String> {
    let re = regex::Regex::new(pattern).ok()?;
    let caps = re.captures(body)?;
    let m = caps.get(1).or_else(|| caps.get(0))?;
    Some(m.as_str().to_string())
}

/// Content fingerprint: ETag header preferred, else Last-Modified, else
/// sha256 hex of the body.
pub fn fingerprint(etag: Option<&str>, last_modified: Option<&str>, body: &str) -> String {
    if let Some(e) = etag.map(str::trim).filter(|s| !s.is_empty()) {
        return e.to_string();
    }
    if let Some(l) = last_modified.map(str::trim).filter(|s| !s.is_empty()) {
        return l.to_string();
    }
    format!("{:x}", Sha256::digest(body.as_bytes()))
}

/// Record a check result. First successful check is a baseline
/// (`has_update = false`); a later check finding a different value flags
/// `has_update = true`. An unchanged value leaves `has_update` alone
/// (only `ogm launch` clears the badge).
pub fn apply_web_check(web: &mut WebInfo, found: String, is_version: bool, now: &str) {
    web.checked_at = Some(now.to_string());
    match &web.latest {
        None => {
            web.latest = Some(found);
            web.latest_is_version = is_version;
            web.has_update = false;
        }
        Some(prev) if *prev == found => {}
        Some(_) => {
            web.latest = Some(found);
            web.latest_is_version = is_version;
            web.has_update = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn web(latest: Option<&str>) -> WebInfo {
        WebInfo {
            url: None,
            update_url: "https://example.com/dl".into(),
            regex: None,
            latest: latest.map(|s| s.into()),
            latest_is_version: false,
            checked_at: None,
            has_update: false,
        }
    }

    #[test]
    fn extract_version_prefers_group_one() {
        let body = "<h1>Download version 2.4.1 for Linux</h1>";
        assert_eq!(
            extract_version(body, "version ([0-9.]+)").as_deref(),
            Some("2.4.1")
        );
    }

    #[test]
    fn extract_version_without_group_uses_whole_match() {
        let body = "build R85 now available";
        assert_eq!(extract_version(body, "R[0-9]+").as_deref(), Some("R85"));
    }

    #[test]
    fn extract_version_no_match_or_bad_pattern() {
        assert_eq!(extract_version("nothing here", "v[0-9]+"), None);
        assert_eq!(extract_version("anything", "([unclosed"), None);
    }

    #[test]
    fn fingerprint_prefers_etag_then_last_modified_then_sha256() {
        assert_eq!(
            fingerprint(Some("\"abc123\""), Some("Mon, 01 Jan 2026"), "body"),
            "\"abc123\""
        );
        assert_eq!(
            fingerprint(None, Some("Mon, 01 Jan 2026 00:00:00 GMT"), "body"),
            "Mon, 01 Jan 2026 00:00:00 GMT"
        );
        assert_eq!(
            fingerprint(Some("  "), None, "body"),
            fingerprint(None, None, "body")
        );
        let fp = fingerprint(None, None, "hello");
        assert_eq!(fp.len(), 64);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(fp, fingerprint(None, None, "hello"));
        assert_ne!(fp, fingerprint(None, None, "hello2"));
    }

    #[test]
    fn first_check_is_baseline_without_update() {
        let mut w = web(None);
        apply_web_check(&mut w, "fp1".into(), false, "T0");
        assert_eq!(w.latest.as_deref(), Some("fp1"));
        assert!(!w.has_update);
        assert_eq!(w.checked_at.as_deref(), Some("T0"));
    }

    #[test]
    fn changed_value_flags_update() {
        let mut w = web(Some("fp1"));
        apply_web_check(&mut w, "fp1".into(), false, "T0");
        assert!(!w.has_update, "unchanged value is not an update");
        apply_web_check(&mut w, "fp2".into(), false, "T1");
        assert!(w.has_update);
        assert_eq!(w.latest.as_deref(), Some("fp2"));
        assert_eq!(w.checked_at.as_deref(), Some("T1"));
    }

    #[test]
    fn version_extraction_marks_latest_is_version() {
        let mut w = web(None);
        apply_web_check(&mut w, "2.4.1".into(), true, "T0");
        assert!(w.latest_is_version);
        assert!(!w.has_update);
        let mut w2 = web(Some("2.4.1"));
        apply_web_check(&mut w2, "2.5.0".into(), true, "T1");
        assert!(w2.has_update && w2.latest_is_version);
    }

    #[test]
    fn cleared_badge_stays_cleared_until_value_changes() {
        let mut w = web(Some("fp1"));
        apply_web_check(&mut w, "fp2".into(), false, "T0");
        assert!(w.has_update);
        w.has_update = false; // what `ogm launch` does
        apply_web_check(&mut w, "fp2".into(), false, "T1");
        assert!(!w.has_update, "same value keeps badge cleared");
    }
}
