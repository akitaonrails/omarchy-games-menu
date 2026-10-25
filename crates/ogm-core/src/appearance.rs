use serde::{Deserialize, Serialize};
use thiserror::Error;
use toml::{Table, Value};

const DEFAULT_BASE_SIZE: f64 = 12.0;

/// QML-facing semantic appearance derived from Omarchy's canonical files.
/// Parsing stays in ogm-core; callers own filesystem and compositor I/O.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Appearance {
    pub overlay_background: String,
    pub overlay_alpha: f64,
    pub background: String,
    pub card: String,
    pub card_edge: String,
    pub accent: String,
    pub text: String,
    pub muted: String,
    pub update: String,
    pub ok: String,
    pub font_base: f64,
    pub reduced_motion: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            overlay_background: "#0d1117".into(),
            overlay_alpha: 0.90,
            background: "#1b2838".into(),
            card: "#2a475e".into(),
            card_edge: "#3a5f7e".into(),
            accent: "#66c0f4".into(),
            text: "#c7d5e0".into(),
            muted: "#8f98a0".into(),
            update: "#f0a232".into(),
            ok: "#5c7e10".into(),
            font_base: DEFAULT_BASE_SIZE,
            reduced_motion: false,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AppearanceError {
    #[error("invalid colors.toml: {0}")]
    ColorsToml(String),
    #[error("missing color {0:?}")]
    MissingColor(&'static str),
    #[error("color {key:?} must be #RRGGBB or #RRGGBBAA")]
    InvalidColor { key: &'static str },
}

impl Appearance {
    pub fn fallback_from_shell(
        theme_shell: Option<&str>,
        user_shell: Option<&str>,
        animations_enabled: Option<bool>,
    ) -> Self {
        let theme = parse_table(theme_shell);
        let user = parse_table(user_shell);
        let mut appearance = Self::default();
        appearance.overlay_background = launcher_color(&user)
            .or_else(|| launcher_color(&theme))
            .unwrap_or(appearance.overlay_background);
        appearance.overlay_alpha = launcher_alpha(&user)
            .or_else(|| launcher_alpha(&theme))
            .unwrap_or(appearance.overlay_alpha);
        appearance.font_base = font_base(&user)
            .or_else(|| font_base(&theme))
            .unwrap_or(DEFAULT_BASE_SIZE)
            .clamp(9.0, 20.0);
        appearance.reduced_motion = animations_enabled
            .map(|enabled| !enabled)
            .unwrap_or_else(|| user_reduced_motion(&user));
        appearance
    }

    /// Resolve Omarchy's palette and shell settings. A compositor answer is
    /// authoritative for motion; the user's shell setting is its fallback.
    pub fn from_sources(
        colors: &str,
        theme_shell: Option<&str>,
        user_shell: Option<&str>,
        animations_enabled: Option<bool>,
    ) -> Result<Self, AppearanceError> {
        let palette: Table = toml::from_str(colors)
            .map_err(|error| AppearanceError::ColorsToml(error.to_string()))?;
        let theme = parse_table(theme_shell);
        let user = parse_table(user_shell);

        let background = color(&palette, "background", Some("bg"))?;
        let dark_background = color(&palette, "dark_background", Some("dark_bg"))?;
        let lighter_background = color(&palette, "lighter_background", Some("lighter_bg"))?;
        let foreground = color(&palette, "foreground", Some("fg"))?;

        Ok(Self {
            overlay_background: launcher_color(&user)
                .or_else(|| launcher_color(&theme))
                .unwrap_or_else(|| dark_background.clone()),
            overlay_alpha: launcher_alpha(&user)
                .or_else(|| launcher_alpha(&theme))
                .unwrap_or(0.90),
            background,
            card: lighter_background,
            card_edge: color(&palette, "muted", None)?,
            accent: color(&palette, "accent", None)?,
            text: foreground,
            muted: optional_color(&palette, "dark_foreground", Some("dark_fg"))
                .transpose()?
                .unwrap_or(color(&palette, "muted", None)?),
            update: color(&palette, "yellow", None)?,
            ok: color(&palette, "green", None)?,
            font_base: font_base(&user)
                .or_else(|| font_base(&theme))
                .unwrap_or(DEFAULT_BASE_SIZE)
                .clamp(9.0, 20.0),
            reduced_motion: animations_enabled
                .map(|enabled| !enabled)
                .unwrap_or_else(|| user_reduced_motion(&user)),
        })
    }
}

pub fn parse_hyprland_animations(json: &str) -> Option<bool> {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()?
        .get("bool")?
        .as_bool()
}

fn parse_table(text: Option<&str>) -> Option<Table> {
    text.and_then(|source| toml::from_str(source).ok())
}

fn table_value<'a>(table: &'a Option<Table>, section: &str, key: &str) -> Option<&'a Value> {
    table.as_ref()?.get(section)?.as_table()?.get(key)
}

fn launcher_color(table: &Option<Table>) -> Option<String> {
    let value = table_value(table, "launcher", "background")?.as_str()?;
    valid_color(value).then(|| value.to_owned())
}

fn launcher_alpha(table: &Option<Table>) -> Option<f64> {
    let value = table_value(table, "launcher", "background-alpha")?;
    value
        .as_float()
        .or_else(|| value.as_integer().map(|v| v as f64))
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(0.0, 1.0))
}

fn font_base(table: &Option<Table>) -> Option<f64> {
    let value = table_value(table, "font", "base-size")?;
    value
        .as_float()
        .or_else(|| value.as_integer().map(|v| v as f64))
        .filter(|value| value.is_finite())
}

fn user_reduced_motion(table: &Option<Table>) -> bool {
    table_value(table, "motion", "reduced")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn color(
    table: &Table,
    key: &'static str,
    alias: Option<&'static str>,
) -> Result<String, AppearanceError> {
    optional_color(table, key, alias).ok_or(AppearanceError::MissingColor(key))?
}

fn optional_color(
    table: &Table,
    key: &'static str,
    alias: Option<&'static str>,
) -> Option<Result<String, AppearanceError>> {
    let value = table
        .get(key)
        .or_else(|| alias.and_then(|name| table.get(name)))?;
    Some(match value.as_str() {
        Some(value) if valid_color(value) => Ok(value.to_owned()),
        _ => Err(AppearanceError::InvalidColor { key }),
    })
}

fn valid_color(value: &str) -> bool {
    matches!(value.len(), 7 | 9)
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: &str = r##"
accent = "#7aa2f7"
muted = "#414868"
background = "#1a1b26"
dark_background = "#13141c"
lighter_background = "#24283b"
foreground = "#a9b1d6"
dark_foreground = "#565f89"
yellow = "#e0af68"
green = "#9ece6a"
"##;

    #[test]
    fn maps_omarchy_palette_to_qml_roles() {
        let appearance = Appearance::from_sources(COLORS, None, None, Some(true)).unwrap();
        assert_eq!(appearance.overlay_background, "#13141c");
        assert_eq!(appearance.background, "#1a1b26");
        assert_eq!(appearance.card, "#24283b");
        assert_eq!(appearance.card_edge, "#414868");
        assert_eq!(appearance.text, "#a9b1d6");
        assert_eq!(appearance.muted, "#565f89");
        assert!(!appearance.reduced_motion);
    }

    #[test]
    fn fallback_keeps_accessibility_and_shell_preferences() {
        let shell = "[font]\nbase-size = 15\n[launcher]\nbackground-alpha = 0.7\n[motion]\nreduced = true\n";
        let appearance = Appearance::fallback_from_shell(None, Some(shell), None);
        assert_eq!(appearance.background, Appearance::default().background);
        assert_eq!(appearance.overlay_alpha, 0.7);
        assert_eq!(appearance.font_base, 15.0);
        assert!(appearance.reduced_motion);
    }

    #[test]
    fn user_shell_wins_and_values_are_bounded() {
        let theme = "[font]\nbase-size = 13\n[launcher]\nbackground-alpha = 0.8\n";
        let user = "[font]\nbase-size = 99\n[launcher]\nbackground = \"#010203\"\nbackground-alpha = 2\n[motion]\nreduced = true\n";
        let appearance = Appearance::from_sources(COLORS, Some(theme), Some(user), None).unwrap();
        assert_eq!(appearance.overlay_background, "#010203");
        assert_eq!(appearance.overlay_alpha, 1.0);
        assert_eq!(appearance.font_base, 20.0);
        assert!(appearance.reduced_motion);
    }

    #[test]
    fn compositor_motion_is_authoritative() {
        let user = "[motion]\nreduced = true\n";
        assert!(
            Appearance::from_sources(COLORS, None, Some(user), Some(false))
                .unwrap()
                .reduced_motion
        );
        assert!(
            !Appearance::from_sources(COLORS, None, Some(user), Some(true))
                .unwrap()
                .reduced_motion
        );
    }

    #[test]
    fn accepts_palette_aliases_and_rejects_bad_colors() {
        let aliases = COLORS
            .replace("background =", "bg =")
            .replace("dark_background =", "dark_bg =")
            .replace("lighter_background =", "lighter_bg =")
            .replace("foreground =", "fg =")
            .replace("dark_foreground =", "dark_fg =");
        assert!(Appearance::from_sources(&aliases, None, None, None).is_ok());
        let invalid = COLORS.replace("#7aa2f7", "blue");
        assert_eq!(
            Appearance::from_sources(&invalid, None, None, None),
            Err(AppearanceError::InvalidColor { key: "accent" })
        );
    }

    #[test]
    fn parses_hyprland_json_without_guessing() {
        assert_eq!(
            parse_hyprland_animations(r#"{"option":"animations:enabled","bool":false}"#),
            Some(false)
        );
        assert_eq!(parse_hyprland_animations("not json"), None);
    }
}
