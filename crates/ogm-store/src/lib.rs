pub mod config;
pub mod fragments;
pub mod paths;
pub mod state;
pub mod user_games;
pub mod version_file;

pub use config::{default_desktop_globs, parse_catalog, Config};
pub use fragments::{load_catalog_fragments, CatalogFragments};
pub use paths::Paths;
pub use state::{write_atomic, State};
pub use user_games::UserGames;
pub use version_file::read_installed_version;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("io error on {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse {path}: {message}")]
    Parse {
        path: std::path::PathBuf,
        message: String,
    },
}
