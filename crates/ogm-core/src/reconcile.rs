use std::collections::{HashMap, HashSet};

use crate::catalog::CatalogEntry;
use crate::desktop::DesktopEntry;
use crate::model::{Category, CustomGame, Game, GithubInfo, DEFAULT_ICON};

/// Build the game list from discovered desktop entries, the curated
/// catalog, user customizations and the previous state. Pure: the caller
/// performs all I/O (desktop scan, version-file reads, clock).
///
/// - `discovered` pairs a parsed desktop entry with its file stem (desktop id).
/// - `installed_versions` maps catalog id -> detected installed version.
/// - Prev-state games matched by id keep added_at/last_played/github/sgdb,
///   and installed_version when no fresh detection overrides it.
pub fn reconcile(
    catalog: &[CatalogEntry],
    discovered: &[(DesktopEntry, String)],
    custom: &[CustomGame],
    hidden: &HashSet<String>,
    prev_state: &[Game],
    installed_versions: &HashMap<String, String>,
    now: &str,
) -> Vec<Game> {
    let prev: HashMap<&str, &Game> = prev_state.iter().map(|g| (g.id.as_str(), g)).collect();
    let catalog_by_desktop: HashMap<&str, &CatalogEntry> = catalog
        .iter()
        .filter_map(|e| e.desktop_id.as_deref().map(|d| (d, e)))
        .collect();

    let mut games = Vec::new();
    for (entry, desktop_id) in discovered {
        let (id, name, category, github) = match catalog_by_desktop.get(desktop_id.as_str()) {
            Some(c) => (
                c.id.clone(),
                c.name
                    .clone()
                    .or_else(|| entry.name.clone())
                    .unwrap_or_else(|| desktop_id.clone()),
                c.category,
                c.github.clone(),
            ),
            None => (
                desktop_id.clone(),
                entry.name.clone().unwrap_or_else(|| desktop_id.clone()),
                Category::Custom,
                None,
            ),
        };
        let prior = prev.get(id.as_str());
        let installed_version = installed_versions
            .get(&id)
            .cloned()
            .or_else(|| prior.and_then(|g| g.installed_version.clone()));
        games.push(Game {
            id: id.clone(),
            name,
            category,
            exec: entry.exec.clone().unwrap_or_default(),
            icon: entry
                .icon
                .clone()
                .unwrap_or_else(|| DEFAULT_ICON.to_string()),
            desktop_id: Some(desktop_id.clone()),
            custom: false,
            added_at: prior
                .map(|g| g.added_at.clone())
                .unwrap_or_else(|| now.to_string()),
            last_played: prior.and_then(|g| g.last_played.clone()),
            play_count: prior.map(|g| g.play_count).unwrap_or(0),
            installed_version,
            github: prior
                .and_then(|g| g.github.clone())
                .or_else(|| github.map(GithubInfo::unchecked)),
            sgdb: prior.and_then(|g| g.sgdb.clone()),
        });
    }

    for c in custom {
        if games.iter().any(|g| g.id == c.id) {
            continue;
        }
        let prior = prev.get(c.id.as_str());
        games.push(Game {
            id: c.id.clone(),
            name: c.name.clone(),
            category: c.category,
            exec: c.exec.clone(),
            icon: c.icon.clone().unwrap_or_else(|| DEFAULT_ICON.to_string()),
            desktop_id: None,
            custom: true,
            added_at: prior
                .map(|g| g.added_at.clone())
                .unwrap_or_else(|| now.to_string()),
            last_played: prior.and_then(|g| g.last_played.clone()),
            play_count: prior.map(|g| g.play_count).unwrap_or(0),
            installed_version: prior.and_then(|g| g.installed_version.clone()),
            github: prior
                .and_then(|g| g.github.clone())
                .or_else(|| c.github.clone().map(GithubInfo::unchecked)),
            sgdb: prior.and_then(|g| g.sgdb.clone()),
        });
    }

    let hidden_defaults: HashSet<&str> = catalog
        .iter()
        .filter(|e| e.hidden_default)
        .map(|e| e.id.as_str())
        .collect();
    games.retain(|g| !hidden.contains(&g.id) && !hidden_defaults.contains(g.id.as_str()));
    games
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SgdbInfo;

    fn desktop(name: &str, exec: &str) -> DesktopEntry {
        DesktopEntry {
            name: Some(name.into()),
            exec: Some(exec.into()),
            icon: None,
            comment: None,
            categories: vec![],
        }
    }

    fn catalog_entry(id: &str, desktop_id: &str) -> CatalogEntry {
        CatalogEntry {
            id: id.into(),
            desktop_id: Some(desktop_id.into()),
            name: None,
            category: Category::Port,
            github: Some("owner/repo".into()),
            sgdb_query: Some(id.into()),
            version_file: None,
            hidden_default: false,
        }
    }

    #[test]
    fn builds_game_from_catalog_and_desktop() {
        let cat = vec![catalog_entry("soh", "gaming-soh")];
        let disc = vec![(
            desktop("Ship of Harkinian (on gaming)", "/run/soh"),
            "gaming-soh".into(),
        )];
        let games = reconcile(
            &cat,
            &disc,
            &[],
            &HashSet::new(),
            &[],
            &HashMap::new(),
            "T0",
        );
        assert_eq!(games.len(), 1);
        let g = &games[0];
        assert_eq!(g.id, "soh");
        assert_eq!(g.name, "Ship of Harkinian (on gaming)");
        assert_eq!(g.desktop_id.as_deref(), Some("gaming-soh"));
        assert_eq!(g.added_at, "T0");
        assert!(!g.custom);
        assert_eq!(g.github.as_ref().unwrap().repo, "owner/repo");
    }

    #[test]
    fn catalog_name_overrides_desktop_name() {
        let mut e = catalog_entry("soh", "gaming-soh");
        e.name = Some("Ship of Harkinian".into());
        let disc = vec![(desktop("SoH (on gaming)", "/run"), "gaming-soh".into())];
        let games = reconcile(
            &[e],
            &disc,
            &[],
            &HashSet::new(),
            &[],
            &HashMap::new(),
            "T0",
        );
        assert_eq!(games[0].name, "Ship of Harkinian");
    }

    #[test]
    fn missing_desktop_is_skipped() {
        let cat = vec![catalog_entry("soh", "gaming-soh")];
        let games = reconcile(&cat, &[], &[], &HashSet::new(), &[], &HashMap::new(), "T0");
        assert!(games.is_empty());
    }

    #[test]
    fn discovered_without_catalog_entry_becomes_custom_category() {
        let disc = vec![(desktop("Mystery", "/run"), "gaming-mystery".into())];
        let games = reconcile(&[], &disc, &[], &HashSet::new(), &[], &HashMap::new(), "T0");
        assert_eq!(games[0].id, "gaming-mystery");
        assert_eq!(games[0].category, Category::Custom);
        assert!(games[0].github.is_none());
    }

    #[test]
    fn preserves_prev_state_fields_across_scans() {
        let cat = vec![catalog_entry("soh", "gaming-soh")];
        let disc = vec![(desktop("SoH", "/run"), "gaming-soh".into())];
        let first = reconcile(
            &cat,
            &disc,
            &[],
            &HashSet::new(),
            &[],
            &HashMap::new(),
            "T0",
        );
        let mut prev = first.clone();
        prev[0].last_played = Some("T5".into());
        prev[0].play_count = 7;
        prev[0].installed_version = Some("9.1.2".into());
        prev[0].sgdb = Some(SgdbInfo {
            id: 42,
            release_date: Some("1998-11-23".into()),
            cover: Some("/covers/soh.jpg".into()),
            hero: None,
        });
        prev[0].github.as_mut().unwrap().latest_tag = Some("9.1.2".into());

        let second = reconcile(
            &cat,
            &disc,
            &[],
            &HashSet::new(),
            &prev,
            &HashMap::new(),
            "T9",
        );
        let g = &second[0];
        assert_eq!(g.added_at, "T0");
        assert_eq!(g.last_played.as_deref(), Some("T5"));
        assert_eq!(g.play_count, 7);
        assert_eq!(g.installed_version.as_deref(), Some("9.1.2"));
        assert_eq!(g.sgdb.as_ref().unwrap().id, 42);
        assert_eq!(
            g.github.as_ref().unwrap().latest_tag.as_deref(),
            Some("9.1.2")
        );
    }

    #[test]
    fn fresh_version_detection_overrides_prev() {
        let cat = vec![catalog_entry("soh", "gaming-soh")];
        let disc = vec![(desktop("SoH", "/run"), "gaming-soh".into())];
        let first = reconcile(
            &cat,
            &disc,
            &[],
            &HashSet::new(),
            &[],
            &HashMap::new(),
            "T0",
        );
        let mut detected = HashMap::new();
        detected.insert("soh".to_string(), "9.2.3".to_string());
        let second = reconcile(&cat, &disc, &[], &HashSet::new(), &first, &detected, "T9");
        assert_eq!(second[0].installed_version.as_deref(), Some("9.2.3"));
    }

    #[test]
    fn hidden_ids_are_dropped() {
        let cat = vec![catalog_entry("soh", "gaming-soh")];
        let disc = vec![(desktop("SoH", "/run"), "gaming-soh".into())];
        let hidden = HashSet::from(["soh".to_string()]);
        let games = reconcile(&cat, &disc, &[], &hidden, &[], &HashMap::new(), "T0");
        assert!(games.is_empty());
    }

    #[test]
    fn hidden_default_catalog_entries_are_dropped() {
        let mut e = catalog_entry("tool", "gaming-tool");
        e.hidden_default = true;
        let disc = vec![(desktop("Tool", "/run"), "gaming-tool".into())];
        let games = reconcile(
            &[e],
            &disc,
            &[],
            &HashSet::new(),
            &[],
            &HashMap::new(),
            "T0",
        );
        assert!(games.is_empty());
    }

    #[test]
    fn custom_games_are_appended_and_preserved() {
        let custom = vec![CustomGame {
            id: "my-game".into(),
            name: "My Game".into(),
            exec: "/run/mine".into(),
            category: Category::Fangame,
            github: Some("me/mine".into()),
            sgdb_query: Some("Mine".into()),
            icon: None,
        }];
        let games = reconcile(
            &[],
            &[],
            &custom,
            &HashSet::new(),
            &[],
            &HashMap::new(),
            "T0",
        );
        assert_eq!(games.len(), 1);
        assert!(games[0].custom);
        assert_eq!(games[0].icon, DEFAULT_ICON);
        assert_eq!(games[0].github.as_ref().unwrap().repo, "me/mine");

        let mut prev = games.clone();
        prev[0].last_played = Some("T3".into());
        let again = reconcile(
            &[],
            &[],
            &custom,
            &HashSet::new(),
            &prev,
            &HashMap::new(),
            "T9",
        );
        assert_eq!(again[0].added_at, "T0");
        assert_eq!(again[0].last_played.as_deref(), Some("T3"));
    }

    #[test]
    fn custom_id_collision_with_discovered_is_ignored() {
        let disc = vec![(desktop("SoH", "/run"), "gaming-soh".into())];
        let custom = vec![CustomGame {
            id: "gaming-soh".into(),
            name: "Impostor".into(),
            exec: "/nope".into(),
            category: Category::Custom,
            github: None,
            sgdb_query: None,
            icon: None,
        }];
        let games = reconcile(
            &[],
            &disc,
            &custom,
            &HashSet::new(),
            &[],
            &HashMap::new(),
            "T0",
        );
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].name, "SoH");
    }
}
