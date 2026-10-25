pub mod appearance;
pub mod catalog;
pub mod desktop;
pub mod filter;
pub mod gamepad;
pub mod glob;
pub mod model;
pub mod reconcile;
pub mod sort;
pub mod time;
pub mod version;
pub mod web;

pub use appearance::{parse_hyprland_animations, Appearance, AppearanceError};
pub use catalog::{merge_catalog_entries, CatalogEntry};
pub use desktop::{DesktopEntry, OgmMeta};
pub use filter::{filter_games, GameFilter};
pub use gamepad::{
    event_to_json, parse_event, parse_input_devices, Decoder, PadDevice, PadEvent, EVENT_SIZE,
};
pub use model::{Category, CustomGame, Game, GithubInfo, SgdbInfo, WebInfo};
pub use reconcile::reconcile;
pub use sort::{sort_games, SortDir, SortKey};
pub use version::{compare_versions, has_update};
