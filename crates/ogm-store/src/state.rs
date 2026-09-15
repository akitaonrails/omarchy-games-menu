use std::fs;
use std::io::Write;
use std::path::Path;

use ogm_core::Game;
use serde::{Deserialize, Serialize};

use crate::paths::Paths;
use crate::StoreError;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub games: Vec<Game>,
    #[serde(default)]
    pub errors: Vec<String>,
}

fn default_version() -> u32 {
    1
}

/// Write `contents` to `path` via a sibling temp file + rename so readers
/// never observe a truncated file.
pub fn write_atomic(path: &Path, contents: &str) -> Result<(), StoreError> {
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| StoreError::Io {
        path: parent.to_path_buf(),
        source: e,
    })?;
    let mut tmp = path.to_path_buf();
    let tmp_name = format!(
        ".{}.{}.tmp",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("state"),
        std::process::id()
    );
    tmp.set_file_name(tmp_name);
    let write_result = (|| -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(contents.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    write_result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        StoreError::Io {
            path: path.to_path_buf(),
            source: e,
        }
    })
}

impl State {
    pub fn load(paths: &Paths) -> Result<State, StoreError> {
        let file = paths.state_file();
        match fs::read_to_string(&file) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| StoreError::Parse {
                path: file.clone(),
                message: e.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(State {
                version: 1,
                ..State::default()
            }),
            Err(e) => Err(StoreError::Io {
                path: file,
                source: e,
            }),
        }
    }

    pub fn save(&self, paths: &Paths) -> Result<(), StoreError> {
        let file = paths.state_file();
        let text = serde_json::to_string_pretty(self).map_err(|e| StoreError::Parse {
            path: file.clone(),
            message: e.to_string(),
        })?;
        write_atomic(&file, &text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogm_core::{Category, Game};

    fn sample_game() -> Game {
        Game {
            id: "x".into(),
            name: "X".into(),
            category: Category::Port,
            exec: "true".into(),
            icon: "i".into(),
            desktop_id: Some("gaming-x".into()),
            custom: false,
            added_at: "2026-09-13T12:00:00Z".into(),
            last_played: None,
            play_count: 0,
            installed_version: None,
            github: None,
            sgdb: None,
        }
    }

    #[test]
    fn state_round_trip_via_atomic_write() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let state = State {
            version: 1,
            generated_at: "2026-09-13T12:00:00Z".into(),
            games: vec![sample_game()],
            errors: vec!["sgdb: timeout".into()],
        };
        state.save(&paths).unwrap();
        let loaded = State::load(&paths).unwrap();
        assert_eq!(loaded, state);
        // no temp file left behind
        let leftovers: Vec<_> = fs::read_dir(paths.data_dir.parent().unwrap().join("data"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .filter(|n| n.to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty());
    }

    #[test]
    fn missing_state_loads_empty() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let state = State::load(&paths).unwrap();
        assert_eq!(state.version, 1);
        assert!(state.games.is_empty());
    }

    #[test]
    fn atomic_write_replaces_existing() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("f.json");
        write_atomic(&target, "one").unwrap();
        write_atomic(&target, "two").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "two");
    }
}
