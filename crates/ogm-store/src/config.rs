use std::fs;
use std::path::PathBuf;

use ogm_core::CatalogEntry;
use serde::{Deserialize, Serialize};

use crate::paths::Paths;
use crate::StoreError;

pub fn default_desktop_globs() -> Vec<String> {
    [
        "gaming-*",
        "pc-*",
        "screamer",
        "screamer2",
        "screamer-rally",
        "ridge-racer",
        "sega-rally-hd",
        "streets-of-rage-remake",
        "virtua-fighter",
        "virtua-racing",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub sgdb_api_key: String,
    pub github_token: Option<String>,
    pub refresh_interval_hours: u64,
    pub desktop_globs: Vec<String>,
    pub applications_dirs: Vec<PathBuf>,
    /// Extra catalog fragment dirs, checked after the default catalog.d.
    pub catalog_dirs: Vec<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sgdb_api_key: String::new(),
            github_token: None,
            refresh_interval_hours: 3,
            desktop_globs: default_desktop_globs(),
            applications_dirs: Vec::new(),
            catalog_dirs: Vec::new(),
        }
    }
}

impl Config {
    /// Load config.toml; missing file or missing keys fall back to defaults.
    /// `applications_dirs` defaults to the XDG applications dir when unset.
    pub fn load(paths: &Paths) -> Result<Config, StoreError> {
        let file = paths.config_file();
        let mut config = match fs::read_to_string(&file) {
            Ok(text) => toml::from_str::<Config>(&text).map_err(|e| StoreError::Parse {
                path: file.clone(),
                message: e.to_string(),
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Config::default(),
            Err(e) => {
                return Err(StoreError::Io {
                    path: file,
                    source: e,
                })
            }
        };
        if config.applications_dirs.is_empty() {
            config.applications_dirs = vec![paths.applications_dir.clone()];
        }
        Ok(config)
    }

    /// Default catalog.d first, then any configured extra dirs.
    pub fn all_catalog_dirs(&self, paths: &Paths) -> Vec<PathBuf> {
        std::iter::once(paths.catalog_d_dir())
            .chain(self.catalog_dirs.iter().cloned())
            .collect()
    }
}

#[derive(Debug, Deserialize)]
struct CatalogFile {
    game: Vec<CatalogEntry>,
}

pub fn parse_catalog(content: &str) -> Result<Vec<CatalogEntry>, StoreError> {
    let parsed: CatalogFile = toml::from_str(content).map_err(|e| StoreError::Parse {
        path: PathBuf::from("<catalog>"),
        message: e.to_string(),
    })?;
    Ok(parsed.game)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_yields_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let config = Config::load(&paths).unwrap();
        assert_eq!(config.sgdb_api_key, "");
        assert_eq!(config.refresh_interval_hours, 3);
        assert!(config.github_token.is_none());
        assert!(config.desktop_globs.contains(&"gaming-*".to_string()));
        assert_eq!(
            config.applications_dirs,
            vec![paths.applications_dir.clone()]
        );
        assert_eq!(config.all_catalog_dirs(&paths), vec![paths.catalog_d_dir()]);
    }

    #[test]
    fn partial_file_merges_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        fs::create_dir_all(&paths.config_dir).unwrap();
        fs::write(
            paths.config_file(),
            "sgdb_api_key = \"k\"\napplications_dirs = [\"/opt/apps\"]\n",
        )
        .unwrap();
        let config = Config::load(&paths).unwrap();
        assert_eq!(config.sgdb_api_key, "k");
        assert_eq!(config.refresh_interval_hours, 3);
        assert_eq!(config.applications_dirs, vec![PathBuf::from("/opt/apps")]);
        assert!(config.desktop_globs.contains(&"pc-*".to_string()));
    }

    #[test]
    fn configured_catalog_dirs_come_after_default() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        fs::create_dir_all(&paths.config_dir).unwrap();
        fs::write(paths.config_file(), "catalog_dirs = [\"/opt/fragments\"]\n").unwrap();
        let config = Config::load(&paths).unwrap();
        assert_eq!(
            config.all_catalog_dirs(&paths),
            vec![paths.catalog_d_dir(), PathBuf::from("/opt/fragments")]
        );
    }

    #[test]
    fn parses_catalog_toml() {
        let toml = r##"
[[game]]
id = "ship-of-harkinian"
desktop_id = "gaming-ship-of-harkinian"
name = "Ship of Harkinian"
category = "port"
github = "HarbourMasters/Shipwright"
sgdb_query = "Ship of Harkinian"
web_url = "https://example.com/soh"
update_url = "https://example.com/soh/dl"
update_regex = "version ([0-9.]+)"

[[game]]
id = "steam"
desktop_id = "gaming-steam"
category = "tool"
"##;
        let entries = parse_catalog(toml).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, "ship-of-harkinian");
        assert_eq!(
            entries[0].github.as_deref(),
            Some("HarbourMasters/Shipwright")
        );
        assert_eq!(
            entries[0].web_url.as_deref(),
            Some("https://example.com/soh")
        );
        assert_eq!(
            entries[0].update_url.as_deref(),
            Some("https://example.com/soh/dl")
        );
        assert_eq!(
            entries[0].update_regex.as_deref(),
            Some("version ([0-9.]+)")
        );
        assert!(!entries[0].hidden_default);
        assert_eq!(entries[1].category, ogm_core::Category::Tool);
        assert!(entries[1].github.is_none());
        assert!(entries[1].web_url.is_none() && entries[1].update_url.is_none());
    }
}
