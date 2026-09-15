use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub applications_dir: PathBuf,
}

impl Paths {
    /// XDG-resolved paths (~/.config/ogm, ~/.local/share/ogm, ~/.cache/ogm),
    /// honoring XDG_CONFIG_HOME / XDG_DATA_HOME / XDG_CACHE_HOME.
    pub fn from_xdg() -> Option<Self> {
        let base = directories::BaseDirs::new()?;
        Some(Paths {
            config_dir: base.config_dir().join("ogm"),
            data_dir: base.data_dir().join("ogm"),
            cache_dir: base.cache_dir().join("ogm"),
            applications_dir: base.data_dir().join("applications"),
        })
    }

    /// Everything under one explicit root (tests, sandboxes).
    pub fn from_root(root: &Path) -> Self {
        Paths {
            config_dir: root.join("config"),
            data_dir: root.join("data"),
            cache_dir: root.join("cache"),
            applications_dir: root.join("applications"),
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn games_file(&self) -> PathBuf {
        self.config_dir.join("games.json")
    }

    pub fn state_file(&self) -> PathBuf {
        self.data_dir.join("state.json")
    }

    pub fn covers_dir(&self) -> PathBuf {
        self.cache_dir.join("covers")
    }

    pub fn catalog_d_dir(&self) -> PathBuf {
        self.config_dir.join("catalog.d")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_root_layout() {
        let p = Paths::from_root(Path::new("/tmp/x"));
        assert_eq!(p.config_file(), PathBuf::from("/tmp/x/config/config.toml"));
        assert_eq!(p.games_file(), PathBuf::from("/tmp/x/config/games.json"));
        assert_eq!(p.state_file(), PathBuf::from("/tmp/x/data/state.json"));
        assert_eq!(p.covers_dir(), PathBuf::from("/tmp/x/cache/covers"));
        assert_eq!(p.catalog_d_dir(), PathBuf::from("/tmp/x/config/catalog.d"));
        assert_eq!(p.applications_dir, PathBuf::from("/tmp/x/applications"));
    }
}
