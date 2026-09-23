use crate::model::Category;

/// Game-menu metadata embedded in a .desktop file via `X-OGM-*` keys
/// (rendered by distrobox-gaming). `None` on DesktopEntry when no such
/// keys exist at all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OgmMeta {
    /// X-OGM-Managed tri-state: `Some(true)` marks the entry as managed,
    /// `Some(false)` is an explicit opt-out that wins over glob discovery,
    /// `None` when the key is absent. Only "true" counts as opted in; any
    /// other explicit value is an opt-out.
    pub managed: Option<bool>,
    pub category: Option<Category>,
    pub github: Option<String>,
    pub sgdb_query: Option<String>,
    pub web_url: Option<String>,
    pub update_url: Option<String>,
    pub update_regex: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEntry {
    pub name: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub comment: Option<String>,
    pub categories: Vec<String>,
    pub ogm: Option<OgmMeta>,
}

const FIELD_CODES: [&str; 16] = [
    "%f", "%F", "%u", "%U", "%i", "%c", "%k", "%d", "%D", "%n", "%N", "%v", "%m", "%w", "%x", "%A",
];

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('s') => out.push(' '),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn strip_field_codes(exec: &str) -> String {
    exec.split_whitespace()
        .filter_map(|token| {
            if token.starts_with('%') {
                if token == "%%" {
                    return Some("%".to_string());
                }
                if FIELD_CODES.contains(&token) {
                    return None;
                }
            }
            Some(token.replace("%%", "%"))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn parse_desktop(content: &str) -> DesktopEntry {
    let mut entry = DesktopEntry::default();
    let mut in_desktop_entry = false;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key_lower = key.trim().to_lowercase();
        if key_lower.starts_with("x-ogm-") {
            let meta = entry.ogm.get_or_insert_with(OgmMeta::default);
            let value = value.trim();
            match key_lower.as_str() {
                "x-ogm-managed" => meta.managed = Some(value == "true"),
                "x-ogm-category" => meta.category = Category::from_name(value),
                "x-ogm-github" => meta.github = Some(value.to_string()),
                "x-ogm-sgdbquery" => meta.sgdb_query = Some(value.to_string()),
                "x-ogm-weburl" => meta.web_url = Some(value.to_string()),
                "x-ogm-updateurl" => meta.update_url = Some(value.to_string()),
                "x-ogm-updateregex" => meta.update_regex = Some(value.to_string()),
                _ => {}
            }
            continue;
        }
        match key.trim() {
            "Name" => entry.name = Some(unescape(value.trim())),
            "Exec" => entry.exec = Some(strip_field_codes(&unescape(value.trim()))),
            "Icon" => entry.icon = Some(value.trim().to_string()),
            "Comment" => entry.comment = Some(unescape(value.trim())),
            "Categories" => {
                entry.categories = value
                    .split(';')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
            _ => {}
        }
    }
    entry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_outrun_fixture() {
        let content = r#"[Desktop Entry]
Type=Application
Name=OutRun 2006: Coast 2 Coast (PC on gaming)
Comment=Windows PC racing game via Wine in gaming
Exec=/usr/bin/distrobox-enter -n gaming -- /mnt/data/distrobox/gaming/bin/outrun-2006
Icon=wine
Categories=Game;
StartupWMClass=OR2006C2C.exe;
"#;
        let e = parse_desktop(content);
        assert_eq!(
            e.name.as_deref(),
            Some("OutRun 2006: Coast 2 Coast (PC on gaming)")
        );
        assert_eq!(
            e.exec.as_deref(),
            Some(
                "/usr/bin/distrobox-enter -n gaming -- /mnt/data/distrobox/gaming/bin/outrun-2006"
            )
        );
        assert_eq!(e.icon.as_deref(), Some("wine"));
        assert_eq!(e.categories, vec!["Game"]);
    }

    #[test]
    fn strips_exec_field_codes() {
        let content = "[Desktop Entry]\nName=X\nExec=flycast-hires %f\n";
        assert_eq!(
            parse_desktop(content).exec.as_deref(),
            Some("flycast-hires")
        );

        let content = "[Desktop Entry]\nName=X\nExec=app %U --icon %i %c %k\n";
        assert_eq!(parse_desktop(content).exec.as_deref(), Some("app --icon"));

        let content = "[Desktop Entry]\nName=X\nExec=app 100%% %u\n";
        assert_eq!(parse_desktop(content).exec.as_deref(), Some("app 100%"));
    }

    #[test]
    fn unescapes_desktop_chars() {
        let content = "[Desktop Entry]\nName=Zelda\\sOcarina\\nTime \\\\Test\n";
        assert_eq!(
            parse_desktop(content).name.as_deref(),
            Some("Zelda Ocarina\nTime \\Test")
        );
    }

    #[test]
    fn ignores_other_groups_and_comments() {
        let content =
            "# comment\n[Desktop Entry]\nName=Real\n[Desktop Action New]\nName=Fake\nExec=fake\n";
        let e = parse_desktop(content);
        assert_eq!(e.name.as_deref(), Some("Real"));
        assert!(e.exec.is_none());
    }

    #[test]
    fn parses_multivalue_categories() {
        let content = "[Desktop Entry]\nName=E\nCategories=Game;Emulator;\n";
        assert_eq!(parse_desktop(content).categories, vec!["Game", "Emulator"]);
    }

    #[test]
    fn parses_full_ogm_metadata() {
        let content = "[Desktop Entry]\nName=SoH\nExec=/run/soh\n\
            X-OGM-Managed=true\nX-OGM-Category=port\n\
            X-OGM-GitHub=HarbourMasters/Shipwright\nX-OGM-SGDBQuery=Ship of Harkinian\n";
        let meta = parse_desktop(content).ogm.unwrap();
        assert_eq!(meta.managed, Some(true));
        assert_eq!(meta.category, Some(Category::Port));
        assert_eq!(meta.github.as_deref(), Some("HarbourMasters/Shipwright"));
        assert_eq!(meta.sgdb_query.as_deref(), Some("Ship of Harkinian"));
    }

    #[test]
    fn ogm_marker_only() {
        let content = "[Desktop Entry]\nName=X\nX-OGM-Managed=true\n";
        let meta = parse_desktop(content).ogm.unwrap();
        assert_eq!(meta.managed, Some(true));
        assert_eq!(meta.category, None);
        assert!(meta.github.is_none() && meta.sgdb_query.is_none());
    }

    #[test]
    fn ogm_managed_is_tri_state() {
        let content = "[Desktop Entry]\nName=X\nX-OGM-Managed=false\n";
        assert_eq!(parse_desktop(content).ogm.unwrap().managed, Some(false));
        let content = "[Desktop Entry]\nName=X\nX-OGM-Managed=yes\n";
        assert_eq!(parse_desktop(content).ogm.unwrap().managed, Some(false));
        let content = "[Desktop Entry]\nName=X\nX-OGM-Managed= true \n";
        assert_eq!(parse_desktop(content).ogm.unwrap().managed, Some(true));
    }

    #[test]
    fn ogm_keys_are_case_insensitive_and_unknown_category_is_none() {
        let content = "[Desktop Entry]\nName=X\nx-ogm-managed=true\nX-Ogm-Category=bogus\n";
        let meta = parse_desktop(content).ogm.unwrap();
        assert_eq!(meta.managed, Some(true));
        assert_eq!(meta.category, None);
    }

    #[test]
    fn no_ogm_keys_means_none() {
        let content = "[Desktop Entry]\nName=X\nExec=/run\n";
        assert!(parse_desktop(content).ogm.is_none());
    }

    #[test]
    fn parses_ogm_web_keys() {
        let content = "[Desktop Entry]\nName=X\nX-OGM-Managed=true\n\
            X-OGM-WebURL=https://example.com/project\n\
            X-OGM-UpdateURL=https://example.com/downloads\n\
            X-OGM-UpdateRegex=version ([0-9.]+)\n";
        let meta = parse_desktop(content).ogm.unwrap();
        assert_eq!(meta.web_url.as_deref(), Some("https://example.com/project"));
        assert_eq!(
            meta.update_url.as_deref(),
            Some("https://example.com/downloads")
        );
        assert_eq!(meta.update_regex.as_deref(), Some("version ([0-9.]+)"));
    }

    #[test]
    fn ogm_web_keys_optional_and_case_insensitive() {
        let content =
            "[Desktop Entry]\nName=X\nx-ogm-managed=true\nx-OGM-weburl=https://example.com\n";
        let meta = parse_desktop(content).ogm.unwrap();
        assert_eq!(meta.web_url.as_deref(), Some("https://example.com"));
        assert!(meta.update_url.is_none() && meta.update_regex.is_none());
    }
}
