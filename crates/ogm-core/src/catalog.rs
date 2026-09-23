use serde::{Deserialize, Serialize};

use crate::model::Category;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    #[serde(default)]
    pub desktop_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    pub category: Category,
    #[serde(default)]
    pub github: Option<String>,
    #[serde(default)]
    pub sgdb_query: Option<String>,
    #[serde(default)]
    pub web_url: Option<String>,
    #[serde(default)]
    pub update_url: Option<String>,
    #[serde(default)]
    pub update_regex: Option<String>,
    #[serde(default)]
    pub version_file: Option<String>,
    #[serde(default)]
    pub hidden_default: bool,
}

/// Merge catalog sources in priority order: earlier sources win position,
/// later sources win content. An entry from `overlay` replaces an entry
/// from `base` when their `desktop_id` matches, or (for custom-style
/// entries without a desktop file) when their `id` matches.
pub fn merge_catalog_entries(base: &[CatalogEntry], overlay: &[CatalogEntry]) -> Vec<CatalogEntry> {
    let mut out: Vec<CatalogEntry> = base.to_vec();
    for entry in overlay {
        let existing = out.iter().position(|e| {
            e.id == entry.id || (entry.desktop_id.is_some() && e.desktop_id == entry.desktop_id)
        });
        match existing {
            Some(i) => out[i] = entry.clone(),
            None => out.push(entry.clone()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, desktop_id: Option<&str>, name: Option<&str>) -> CatalogEntry {
        CatalogEntry {
            id: id.into(),
            desktop_id: desktop_id.map(|s| s.into()),
            name: name.map(|s| s.into()),
            category: Category::Port,
            github: None,
            sgdb_query: None,
            web_url: None,
            update_url: None,
            update_regex: None,
            version_file: None,
            hidden_default: false,
        }
    }

    #[test]
    fn entry_defaults() {
        let e = CatalogEntry {
            id: "x".into(),
            desktop_id: None,
            name: None,
            category: Category::Custom,
            github: None,
            sgdb_query: None,
            web_url: None,
            update_url: None,
            update_regex: None,
            version_file: None,
            hidden_default: false,
        };
        assert!(!e.hidden_default);
    }

    #[test]
    fn overlay_replaces_bundled_entry_by_desktop_id() {
        let base = vec![
            entry("soh", Some("gaming-soh"), Some("Ship of Harkinian")),
            entry("starship", Some("gaming-starship"), None),
        ];
        let overlay = vec![entry(
            "soh-renamed",
            Some("gaming-soh"),
            Some("My Zelda Port"),
        )];
        let merged = merge_catalog_entries(&base, &overlay);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].id, "soh-renamed");
        assert_eq!(merged[0].name.as_deref(), Some("My Zelda Port"));
        assert_eq!(merged[1].id, "starship");
    }

    #[test]
    fn overlay_replaces_by_id_for_custom_style_entries() {
        let base = vec![entry("mine", None, Some("Old Name"))];
        let overlay = vec![entry("mine", None, Some("New Name"))];
        let merged = merge_catalog_entries(&base, &overlay);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name.as_deref(), Some("New Name"));
    }

    #[test]
    fn new_fragment_entries_are_appended() {
        let base = vec![entry("a", Some("gaming-a"), None)];
        let overlay = vec![entry("b", Some("gaming-b"), None)];
        let merged = merge_catalog_entries(&base, &overlay);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[1].id, "b");
    }

    #[test]
    fn sequential_overlays_compose_deterministically() {
        let base = vec![entry("a", Some("gaming-a"), Some("A"))];
        let first = vec![
            entry("a", Some("gaming-a"), Some("A2")),
            entry("b", Some("gaming-b"), None),
        ];
        let second = vec![entry("a", Some("gaming-a"), Some("A3"))];
        let merged = merge_catalog_entries(&merge_catalog_entries(&base, &first), &second);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].name.as_deref(), Some("A3"));
        assert_eq!(merged[1].id, "b");
    }
}
