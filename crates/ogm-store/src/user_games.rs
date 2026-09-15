use std::collections::HashSet;
use std::fs;

use ogm_core::CustomGame;
use serde::{Deserialize, Serialize};

use crate::paths::Paths;
use crate::StoreError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserGames {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub custom: Vec<CustomGame>,
    #[serde(default)]
    pub hidden: Vec<String>,
}

fn default_version() -> u32 {
    1
}

impl Default for UserGames {
    fn default() -> Self {
        UserGames {
            version: 1,
            custom: Vec::new(),
            hidden: Vec::new(),
        }
    }
}

impl UserGames {
    pub fn load(paths: &Paths) -> Result<UserGames, StoreError> {
        let file = paths.games_file();
        match fs::read_to_string(&file) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| StoreError::Parse {
                path: file.clone(),
                message: e.to_string(),
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UserGames::default()),
            Err(e) => Err(StoreError::Io {
                path: file,
                source: e,
            }),
        }
    }

    pub fn save(&self, paths: &Paths) -> Result<(), StoreError> {
        let file = paths.games_file();
        let text = serde_json::to_string_pretty(self).map_err(|e| StoreError::Parse {
            path: file.clone(),
            message: e.to_string(),
        })?;
        crate::state::write_atomic(&file, &text)
    }

    pub fn hidden_set(&self) -> HashSet<String> {
        self.hidden.iter().cloned().collect()
    }

    pub fn hide(&mut self, id: &str) {
        if !self.hidden.iter().any(|h| h == id) {
            self.hidden.push(id.to_string());
        }
    }

    pub fn show(&mut self, id: &str) {
        self.hidden.retain(|h| h != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ogm_core::Category;

    #[test]
    fn games_json_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let mut user = UserGames::default();
        user.custom.push(CustomGame {
            id: "mine".into(),
            name: "Mine".into(),
            exec: "/run/mine".into(),
            category: Category::Fangame,
            github: None,
            sgdb_query: Some("Mine".into()),
            icon: None,
        });
        user.hide("gaming-steam");
        user.save(&paths).unwrap();
        let loaded = UserGames::load(&paths).unwrap();
        assert_eq!(loaded, user);
        assert_eq!(loaded.version, 1);
    }

    #[test]
    fn missing_games_json_loads_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = UserGames::load(&Paths::from_root(dir.path())).unwrap();
        assert!(loaded.custom.is_empty() && loaded.hidden.is_empty());
    }

    #[test]
    fn hide_is_idempotent_and_show_removes() {
        let mut u = UserGames::default();
        u.hide("a");
        u.hide("a");
        assert_eq!(u.hidden, vec!["a"]);
        u.show("a");
        u.show("a");
        assert!(u.hidden.is_empty());
    }
}
