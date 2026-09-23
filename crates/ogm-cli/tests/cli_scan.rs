use std::fs;
use std::process::Command;

fn ogm() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ogm"))
}

fn set_xdg(cmd: &mut Command, root: &std::path::Path) {
    cmd.env("XDG_CONFIG_HOME", root.join("xdg-config"))
        .env("XDG_DATA_HOME", root.join("xdg-data"))
        .env("XDG_CACHE_HOME", root.join("xdg-cache"));
}

#[test]
fn scan_writes_state_json_against_temp_xdg() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    let apps = root.join("apps");
    fs::create_dir_all(&apps).unwrap();
    fs::write(
        apps.join("gaming-ship-of-harkinian.desktop"),
        "[Desktop Entry]\nName=Zelda: Ocarina of Time - Ship of Harkinian (on gaming)\n\
         Exec=/usr/bin/distrobox-enter -n gaming -- /mnt/data/distrobox/gaming/bin/ship-of-harkinian\n\
         Icon=applications-games\nCategories=Game;\n",
    )
    .unwrap();
    fs::write(
        apps.join("screamer.desktop"),
        "[Desktop Entry]\nName=Screamer (on gaming)\nExec=/usr/bin/distrobox-enter -n gaming -- /mnt/data/distrobox/gaming/bin/screamer-launch %f\nIcon=dosbox-staging\nCategories=Game;\n",
    )
    .unwrap();
    fs::write(
        apps.join("unrelated.desktop"),
        "[Desktop Entry]\nName=Not A Game\nExec=/bin/true\n",
    )
    .unwrap();

    let config_dir = root.join("xdg-config/ogm");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.toml"),
        format!("applications_dirs = [\"{}\"]\n", apps.display()),
    )
    .unwrap();

    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.arg("scan").arg("--json").output().unwrap()
    };
    assert!(
        out.status.success(),
        "scan --json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let state_path = root.join("xdg-data/ogm/state.json");
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(state["version"], 1);
    assert!(state["generated_at"].as_str().unwrap().contains('T'));

    let games = state["games"].as_array().unwrap();
    assert_eq!(games.len(), 2, "state: {state}");

    let soh = games
        .iter()
        .find(|g| g["id"] == "ship-of-harkinian")
        .expect("ship-of-harkinian present");
    assert_eq!(soh["name"], "Zelda: Ocarina of Time - Ship of Harkinian");
    assert_eq!(soh["category"], "port");
    assert_eq!(soh["desktop_id"], "gaming-ship-of-harkinian");
    assert_eq!(soh["custom"], false);
    assert_eq!(soh["github"]["repo"], "HarbourMasters/Shipwright");
    assert!(soh["last_played"].is_null());
    assert!(soh["sgdb"].is_null());

    let screamer = games
        .iter()
        .find(|g| g["id"] == "screamer")
        .expect("screamer present");
    assert_eq!(screamer["category"], "wine");
    // %f field code stripped
    assert!(!screamer["exec"].as_str().unwrap().contains("%f"));

    // add a custom game, rescan, remove it
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args([
            "add",
            "--name",
            "My Test Game",
            "--exec",
            "/bin/true",
            "--category",
            "fangame",
        ])
        .output()
        .unwrap()
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap().trim(),
        "my-test-game"
    );

    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let games = state["games"].as_array().unwrap();
    assert_eq!(games.len(), 3);
    let custom = games.iter().find(|g| g["id"] == "my-test-game").unwrap();
    assert_eq!(custom["custom"], true);
    assert_eq!(custom["category"], "fangame");

    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args(["remove", "my-test-game"]).output().unwrap()
    };
    assert!(out.status.success());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(state["games"].as_array().unwrap().len(), 2);

    // hide + show a catalog game
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args(["hide", "screamer"]).output().unwrap()
    };
    assert!(out.status.success());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(state["games"].as_array().unwrap().len(), 1);
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args(["show", "screamer"]).output().unwrap()
    };
    assert!(out.status.success());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    assert_eq!(state["games"].as_array().unwrap().len(), 2);

    // list works against the same state
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args(["list", "--json"]).output().unwrap()
    };
    assert!(out.status.success());
    let listed: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(listed.as_array().unwrap().len(), 2);

    // catalog.d fragments: override a bundled entry, skip a broken file
    let frag_dir = config_dir.join("catalog.d");
    fs::create_dir_all(&frag_dir).unwrap();
    fs::write(
        frag_dir.join("10-override.toml"),
        "[[game]]\nid = \"soh-override\"\ndesktop_id = \"gaming-ship-of-harkinian\"\n\
         name = \"Zelda OoT (fragment)\"\ncategory = \"port\"\nsgdb_query = \"Ship of Harkinian\"\n",
    )
    .unwrap();
    fs::write(frag_dir.join("90-broken.toml"), "[[game]\nthis is not toml").unwrap();
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.arg("scan").output().unwrap()
    };
    assert!(out.status.success());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let games = state["games"].as_array().unwrap();
    let soh = games
        .iter()
        .find(|g| g["desktop_id"] == "gaming-ship-of-harkinian")
        .unwrap();
    assert_eq!(soh["id"], "soh-override");
    assert_eq!(soh["name"], "Zelda OoT (fragment)");
    let errors = state["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 1, "state: {state}");
    assert!(errors[0].as_str().unwrap().contains("90-broken.toml"));

    // launch increments play_count and sets last_played
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args([
            "add",
            "--name",
            "Launcher Probe",
            "--exec",
            &format!("touch {}", root.join("launched").display()),
            "--web-url",
            "https://example.com/probe",
        ])
        .output()
        .unwrap()
    };
    assert!(out.status.success());
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args(["launch", "launcher-probe"]).output().unwrap()
    };
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for _ in 0..50 {
        if root.join("launched").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(root.join("launched").exists(), "launch did not spawn exec");
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let probe = state["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == "launcher-probe")
        .cloned()
        .unwrap();
    assert_eq!(probe["play_count"], 1);
    assert!(probe["last_played"].is_string());
    // second launch increments again (and survives the rescan add performs)
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args(["launch", "launcher-probe"]).output().unwrap()
    };
    assert!(out.status.success());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let probe = state["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == "launcher-probe")
        .cloned()
        .unwrap();
    assert_eq!(probe["play_count"], 2);

    // web skeleton attached from --web-url (update_url falls back to web_url)
    assert_eq!(
        probe["web"]["update_url"], "https://example.com/probe",
        "probe state: {probe}"
    );
    assert_eq!(probe["web"]["has_update"], false);

    // a set badge is cleared by launch
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    for g in state["games"].as_array_mut().unwrap() {
        if g["id"] == "launcher-probe" {
            g["web"]["has_update"] = serde_json::Value::Bool(true);
            g["web"]["latest"] = serde_json::Value::String("fp".into());
        }
    }
    fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap()).unwrap();
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.args(["launch", "launcher-probe"]).output().unwrap()
    };
    assert!(out.status.success());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let probe = state["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == "launcher-probe")
        .cloned()
        .unwrap();
    assert_eq!(probe["web"]["has_update"], false);
    assert_eq!(probe["web"]["latest"], "fp", "polled fields preserved");

    // X-OGM metadata: marker entry overrides bundled category, keeps catalog github;
    // marker-only file not matching globs is still discovered.
    fs::write(
        apps.join("gaming-starship.desktop"),
        "[Desktop Entry]\nName=Starship - Star Fox 64 (on gaming)\n\
         Exec=/usr/bin/distrobox-enter -n gaming -- /run/starship\nIcon=applications-games\n\
         X-OGM-Managed=true\nX-OGM-Category=wine\n",
    )
    .unwrap();
    fs::write(
        apps.join("zz-marker-game.desktop"),
        "[Desktop Entry]\nName=Marker Only Game\nExec=/bin/true\n\
         X-OGM-Managed=true\nX-OGM-Category=fangame\nX-OGM-SGDBQuery=Marker Game\n\
         X-OGM-WebURL=https://example.com/marker\nX-OGM-UpdateRegex=build ([0-9]+)\n",
    )
    .unwrap();
    let out = {
        let mut cmd = ogm();
        set_xdg(&mut cmd, root);
        cmd.arg("scan").output().unwrap()
    };
    assert!(out.status.success());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&state_path).unwrap()).unwrap();
    let games = state["games"].as_array().unwrap();
    let starship = games.iter().find(|g| g["id"] == "starship").unwrap();
    assert_eq!(
        starship["category"], "wine",
        "X-OGM-Category beats catalog port"
    );
    assert_eq!(
        starship["github"]["repo"], "HarbourMasters/Starship",
        "github falls through from bundled catalog"
    );
    let marker = games
        .iter()
        .find(|g| g["id"] == "zz-marker-game")
        .expect("marker-only entry discovered without glob match");
    assert_eq!(marker["category"], "fangame");
    assert_eq!(marker["name"], "Marker Only Game");
    assert_eq!(marker["web"]["update_url"], "https://example.com/marker");
    assert_eq!(marker["web"]["regex"], "build ([0-9]+)");
    assert_eq!(marker["web"]["has_update"], false);
}
