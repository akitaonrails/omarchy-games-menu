pub mod catalog;
pub mod desktop;
pub mod filter;
pub mod glob;
pub mod model;
pub mod reconcile;
pub mod sort;
pub mod time;
pub mod version;

pub use catalog::{merge_catalog_entries, CatalogEntry};
pub use desktop::DesktopEntry;
pub use filter::{filter_games, GameFilter};
pub use model::{Category, CustomGame, Game, GithubInfo, SgdbInfo};
pub use reconcile::reconcile;
pub use sort::{sort_games, SortDir, SortKey};
pub use version::{compare_versions, has_update};
