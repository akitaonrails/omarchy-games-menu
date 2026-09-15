#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesktopEntry {
    pub name: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub comment: Option<String>,
    pub categories: Vec<String>,
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
}
