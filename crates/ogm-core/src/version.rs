use std::cmp::Ordering;

fn segments(v: &str) -> Vec<String> {
    let v = v.trim();
    let v = if v.len() > 1
        && (v.starts_with('v') || v.starts_with('V'))
        && v[1..].starts_with(|c: char| c.is_ascii_digit())
    {
        &v[1..]
    } else {
        v
    };
    v.split(['.', '-', '_'])
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn compare_segment(a: &str, b: &str) -> Ordering {
    let an = a.chars().all(|c| c.is_ascii_digit());
    let bn = b.chars().all(|c| c.is_ascii_digit());
    if an && bn {
        match (a.parse::<u64>(), b.parse::<u64>()) {
            (Ok(x), Ok(y)) => x.cmp(&y),
            _ => a.cmp(b),
        }
    } else {
        a.cmp(b)
    }
}

/// Tolerant version ordering: strips a leading v/V, splits on `.-_`,
/// numeric segments compare numerically, anything else lexically.
/// Missing trailing segments are treated as 0, so "1.0" == "v1.0.0".
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let sa = segments(a);
    let sb = segments(b);
    let len = sa.len().max(sb.len());
    for i in 0..len {
        let x = sa.get(i).map(|s| s.as_str()).unwrap_or("0");
        let y = sb.get(i).map(|s| s.as_str()).unwrap_or("0");
        let ord = compare_segment(x, y);
        if ord != Ordering::Equal {
            return ord;
        }
    }
    Ordering::Equal
}

fn looks_like_version(v: &str) -> bool {
    !v.trim().is_empty() && v.chars().any(|c| c.is_ascii_digit())
}

pub fn has_update(installed: Option<&str>, latest: Option<&str>) -> bool {
    match (installed, latest) {
        (Some(i), Some(l)) if looks_like_version(i) && looks_like_version(l) => {
            compare_versions(l, i) == Ordering::Greater
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_table() {
        let cases: &[(&str, &str, Ordering)] = &[
            ("v1.10.2", "v1.9.9", Ordering::Greater),
            ("9.0.1", "9.0.1", Ordering::Equal),
            ("1.0", "v1.0.0", Ordering::Equal),
            ("1.0", "1.0.1", Ordering::Less),
            ("R85", "R86", Ordering::Less),
            ("v7533", "v7532", Ordering::Greater),
            ("1.0.0-alpha", "1.0.0-beta", Ordering::Less),
            ("2.0", "10.0", Ordering::Less),
            ("v26.03.28.0-stable", "v26.03.28.0", Ordering::Greater),
            ("1.0.0", "v1", Ordering::Equal),
        ];
        for (a, b, want) in cases {
            assert_eq!(
                compare_versions(a, b),
                *want,
                "compare_versions({a:?}, {b:?})"
            );
            assert_eq!(
                compare_versions(b, a),
                want.reverse(),
                "compare_versions({b:?}, {a:?})"
            );
        }
    }

    #[test]
    fn garbage_does_not_panic() {
        assert_eq!(compare_versions("", ""), Ordering::Equal);
        let _ = compare_versions("???", "1.0");
        let _ = compare_versions("99999999999999999999999999", "1");
        let _ = compare_versions("a.b.c", "x.y.z");
    }

    #[test]
    fn has_update_rules() {
        assert!(has_update(Some("1.0"), Some("1.1")));
        assert!(!has_update(Some("1.1"), Some("1.0")));
        assert!(!has_update(Some("1.0"), Some("1.0")));
        assert!(!has_update(None, Some("1.0")));
        assert!(!has_update(Some("1.0"), None));
        assert!(!has_update(None, None));
        assert!(!has_update(Some("garbage"), Some("also-garbage")));
        assert!(has_update(Some("v9.1.1"), Some("9.1.2")));
    }
}
