use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

fn qml(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../qml")
        .join(name);
    fs::read_to_string(path).unwrap()
}

#[test]
fn theme_keeps_text_scale_independent_from_cover_zoom() {
    let theme = qml("Theme.qml");
    assert!(theme.contains("fontBase / 12"));
    let font_section = theme.split("readonly property real _fs:").nth(1).unwrap();
    assert!(!font_section.lines().next().unwrap().contains("coverScale"));
}

#[test]
fn motion_and_geometry_are_shared_tokens() {
    let theme = qml("Theme.qml");
    assert!(theme.contains("readonly property int radius"));
    assert!(theme.contains("readonly property int motionFast"));
    assert!(theme.contains("reducedMotion ? 0 : 140"));

    for file in ["GameCard.qml", "GameGrid.qml", "TopBar.qml"] {
        let source = qml(file);
        assert!(!source.contains("duration: 120"), "{file}");
        assert!(!source.contains("scale: (cardMouse"), "{file}");
    }
}

#[test]
fn overlay_follows_canonical_omarchy_inputs_and_exposes_control_names() {
    let shell = qml("shell.qml");
    assert!(shell.contains("stateHome + \"/omarchy/current\""));
    assert!(shell.contains("omarchyCurrent + \"/theme/colors.toml\""));
    assert!(shell.contains("configHome + \"/omarchy/shell.toml\""));
    assert!(shell.contains("configHome + \"/hypr/looknfeel.lua\""));
    assert!(shell.contains("ogmCmd([\"appearance\"])"));

    for file in ["TopBar.qml", "AddGameDialog.qml", "GameCard.qml"] {
        assert!(qml(file).contains("Accessible.name"), "{file}");
    }
}

#[cfg(unix)]
#[test]
fn checkout_launcher_is_executable_for_local_install_symlinks() {
    let launcher = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../games-menu");
    let mode = fs::metadata(launcher).unwrap().permissions().mode();
    assert_ne!(mode & 0o111, 0);
}
