use std::fs;
use std::process::Command;

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
fn appearance_reads_xdg_theme_and_user_preferences() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let theme = root.join("state/omarchy/current/theme");
    let config = root.join("config/omarchy");
    let empty_path = root.join("bin");
    fs::create_dir_all(&theme).unwrap();
    fs::create_dir_all(&config).unwrap();
    fs::create_dir_all(&empty_path).unwrap();
    fs::write(theme.join("colors.toml"), COLORS).unwrap();
    fs::write(
        theme.join("shell.toml"),
        "[font]\nbase-size = 13\n[launcher]\nbackground-alpha = 0.8\n",
    )
    .unwrap();
    fs::write(
        config.join("shell.toml"),
        "[font]\nbase-size = 15\n[motion]\nreduced = true\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_ogm"))
        .arg("appearance")
        .env("HOME", root)
        .env("XDG_STATE_HOME", root.join("state"))
        .env("XDG_CONFIG_HOME", root.join("config"))
        .env("PATH", empty_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let appearance: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(appearance["background"], "#1a1b26");
    assert_eq!(appearance["card"], "#24283b");
    assert_eq!(appearance["font_base"], 15.0);
    assert_eq!(appearance["overlay_alpha"], 0.8);
    assert_eq!(appearance["reduced_motion"], true);
}

#[test]
fn appearance_has_a_readable_fallback_without_omarchy() {
    let tmp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ogm"))
        .arg("appearance")
        .env("HOME", tmp.path())
        .env("XDG_STATE_HOME", tmp.path().join("state"))
        .env("XDG_CONFIG_HOME", tmp.path().join("config"))
        .env("PATH", tmp.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let appearance: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(appearance["background"], "#1b2838");
    assert_eq!(appearance["text"], "#c7d5e0");
    assert_eq!(appearance["font_base"], 12.0);
}
