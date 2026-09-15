use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ogm_core::{
    catalog::CatalogEntry, desktop, glob::matches_any, merge_catalog_entries, model::CustomGame,
    reconcile, Category,
};
use ogm_net::{ArtworkSource, GithubClient, ReleasesSource, SgdbClient};
use ogm_store::{
    load_catalog_fragments, parse_catalog, read_installed_version, Config, Paths, State, UserGames,
};

const CATALOG_TOML: &str = include_str!("../../../catalog.toml");

#[derive(Parser)]
#[command(name = "ogm", about = "omarchy-games-menu backend")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scan desktop entries and write state.json
    Scan {
        /// Print full state JSON instead of a summary
        #[arg(long)]
        json: bool,
    },
    /// Check GitHub releases and SteamGridDB artwork
    Refresh {
        /// Ignore the refresh interval cache
        #[arg(long)]
        force: bool,
    },
    /// Launch a game by id
    Launch { id: String },
    /// Add a custom game
    Add {
        #[arg(long)]
        name: String,
        #[arg(long)]
        exec: String,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        github: Option<String>,
        #[arg(long)]
        sgdb_query: Option<String>,
        #[arg(long)]
        icon: Option<String>,
    },
    /// Remove a game (custom: delete; catalog: hide)
    Remove { id: String },
    /// Hide a game from state.json
    Hide { id: String },
    /// Unhide a game
    Show { id: String },
    /// Print paths and configuration status
    Doctor,
    /// List games from state.json
    List {
        #[arg(long)]
        json: bool,
    },
}

fn now_rfc3339() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    ogm_core::time::epoch_to_rfc3339(secs)
}

fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn unique_id(base: &str, taken: &HashSet<String>) -> String {
    if !taken.contains(base) {
        return base.to_string();
    }
    for n in 2.. {
        let candidate = format!("{base}-{n}");
        if !taken.contains(&candidate) {
            return candidate;
        }
    }
    unreachable!()
}

fn parse_category(s: &str) -> Result<Category> {
    serde_json::from_value(serde_json::Value::String(s.to_lowercase()))
        .map_err(|_| anyhow::anyhow!("unknown category {s:?} (port, decomp, recomp, fangame, wine, arcade, emulator, tool, custom)"))
}

struct DiscoveredEntry {
    entry: desktop::DesktopEntry,
    stem: String,
    /// Selected via X-OGM-Managed=true (vs the legacy desktop_globs).
    via_marker: bool,
}

fn scan_applications_dirs(config: &Config) -> Vec<DiscoveredEntry> {
    let mut out = Vec::new();
    for dir in &config.applications_dirs {
        let Ok(read_dir) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let via_glob = matches_any(stem, &config.desktop_globs);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let parsed = desktop::parse_desktop(&text);
            let via_marker = parsed.ogm.as_ref().map(|m| m.managed).unwrap_or(false);
            if !via_marker && !via_glob {
                continue;
            }
            out.push(DiscoveredEntry {
                entry: parsed,
                stem: stem.to_string(),
                via_marker,
            });
        }
    }
    out.sort_by(|a, b| a.stem.cmp(&b.stem));
    out
}

/// X-OGM-* metadata becomes the topmost catalog-like override: it wins over
/// catalog.d fragments and the bundled catalog, but fields it doesn't set
/// (github/sgdb_query/category, and always name/version_file) carry over from
/// the catalog entry with the same desktop_id.
fn synthesize_marker_overlay(
    discovered: &[DiscoveredEntry],
    catalog: &[CatalogEntry],
) -> Vec<CatalogEntry> {
    let mut overlay = Vec::new();
    for d in discovered {
        let Some(meta) = d.entry.ogm.as_ref().filter(|m| m.managed) else {
            continue;
        };
        let base = catalog
            .iter()
            .find(|e| e.desktop_id.as_deref() == Some(d.stem.as_str()));
        overlay.push(CatalogEntry {
            id: base.map(|e| e.id.clone()).unwrap_or_else(|| d.stem.clone()),
            desktop_id: Some(d.stem.clone()),
            name: base.and_then(|e| e.name.clone()),
            category: meta
                .category
                .or_else(|| base.map(|e| e.category))
                .unwrap_or(Category::Custom),
            github: meta
                .github
                .clone()
                .or_else(|| base.and_then(|e| e.github.clone())),
            sgdb_query: meta
                .sgdb_query
                .clone()
                .or_else(|| base.and_then(|e| e.sgdb_query.clone())),
            version_file: base.and_then(|e| e.version_file.clone()),
            hidden_default: base.map(|e| e.hidden_default).unwrap_or(false),
        });
    }
    overlay
}

fn detect_installed_versions(catalog: &[CatalogEntry]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for entry in catalog {
        if let Some(vf) = &entry.version_file {
            if let Some(v) = read_installed_version(Path::new(vf)) {
                out.insert(entry.id.clone(), v);
            }
        }
    }
    out
}

/// Bundled catalog.toml merged with catalog.d fragments (fragments win).
/// Returns the merged entries plus per-file fragment reports and warnings.
fn load_full_catalog(
    paths: &Paths,
    config: &Config,
) -> Result<(Vec<CatalogEntry>, ogm_store::CatalogFragments)> {
    let bundled = parse_catalog(CATALOG_TOML).context("parsing bundled catalog")?;
    let fragments = load_catalog_fragments(&config.all_catalog_dirs(paths));
    let merged = merge_catalog_entries(&bundled, &fragments.entries);
    Ok((merged, fragments))
}

fn run_scan(paths: &Paths) -> Result<State> {
    let config = Config::load(paths).context("loading config.toml")?;
    let user = UserGames::load(paths).context("loading games.json")?;
    let prev = State::load(paths).context("loading state.json")?;
    let (catalog, fragments) = load_full_catalog(paths, &config)?;
    let discovered = scan_applications_dirs(&config);
    let catalog =
        merge_catalog_entries(&catalog, &synthesize_marker_overlay(&discovered, &catalog));
    let versions = detect_installed_versions(&catalog);
    let discovered_pairs: Vec<(desktop::DesktopEntry, String)> = discovered
        .iter()
        .map(|d| (d.entry.clone(), d.stem.clone()))
        .collect();
    let games = reconcile(
        &catalog,
        &discovered_pairs,
        &user.custom,
        &user.hidden_set(),
        &prev.games,
        &versions,
        &now_rfc3339(),
    );
    let state = State {
        version: 1,
        generated_at: now_rfc3339(),
        games,
        errors: fragments.warnings,
    };
    state.save(paths).context("writing state.json")?;
    Ok(state)
}

fn cover_path(paths: &Paths, id: &str) -> PathBuf {
    paths.covers_dir().join(format!("{id}.jpg"))
}

fn sgdb_queries(catalog: &[CatalogEntry], user: &UserGames) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for entry in catalog {
        if let (Some(desktop_id), Some(q)) = (&entry.desktop_id, &entry.sgdb_query) {
            map.insert(desktop_id.clone(), q.clone());
        }
        if let Some(q) = &entry.sgdb_query {
            map.insert(entry.id.clone(), q.clone());
        }
    }
    for c in &user.custom {
        if let Some(q) = &c.sgdb_query {
            map.insert(c.id.clone(), q.clone());
        }
    }
    map
}

async fn run_refresh(paths: &Paths, force: bool) -> Result<()> {
    let config = Config::load(paths).context("loading config.toml")?;
    let user = UserGames::load(paths).context("loading games.json")?;
    let mut state = State::load(paths).context("loading state.json")?;
    let (catalog, fragments) = load_full_catalog(paths, &config)?;
    let now = now_rfc3339();
    let now_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let stale_after = (config.refresh_interval_hours.max(1)) as i64 * 3600;

    let mut errors: Vec<String> = fragments.warnings;

    let github = GithubClient::new(config.github_token.clone()).context("github client")?;
    for game in &mut state.games {
        let Some(info) = &game.github else { continue };
        let checked = info
            .checked_at
            .as_deref()
            .and_then(ogm_core::time::rfc3339_to_epoch);
        let stale = checked.map(|c| now_epoch - c > stale_after).unwrap_or(true);
        if !force && !stale {
            continue;
        }
        let repo = info.repo.clone();
        match github.latest_release(&repo, None).await {
            Ok(ogm_net::ReleaseOutcome::Released(rel)) => {
                let info = game.github.as_mut().expect("checked above");
                info.latest_tag = Some(rel.tag.clone());
                info.latest_url = Some(rel.url);
                info.published_at = rel.published_at;
                info.checked_at = Some(now.clone());
                info.has_update =
                    ogm_core::has_update(game.installed_version.as_deref(), Some(rel.tag.as_str()));
            }
            Ok(ogm_net::ReleaseOutcome::NotModified) => {
                game.github.as_mut().expect("checked above").checked_at = Some(now.clone());
            }
            Ok(ogm_net::ReleaseOutcome::NotFound) => {
                let info = game.github.as_mut().expect("checked above");
                info.checked_at = Some(now.clone());
                info.has_update = false;
            }
            Err(e) => errors.push(format!("github {}: {e}", game.id)),
        }
    }

    if !config.sgdb_api_key.is_empty() {
        let sgdb = SgdbClient::new(config.sgdb_api_key.clone()).context("sgdb client")?;
        let queries = sgdb_queries(&catalog, &user);
        for game in &mut state.games {
            let Some(query) = queries.get(&game.id).cloned() else {
                continue;
            };
            let cover = cover_path(paths, &game.id);
            if cover.exists() && game.sgdb.is_some() {
                continue;
            }
            let result: Result<(u64, Option<String>, Option<String>), String> = async {
                let hit = sgdb
                    .search_game(&query)
                    .await
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| format!("no match for {query:?}"))?;
                let grid = sgdb.best_grid(hit.id).await.map_err(|e| e.to_string())?;
                if let Some(url) = grid {
                    sgdb.download(&url, &cover)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok((hit.id, hit.release_date, Some(cover.display().to_string())))
                } else {
                    Ok((hit.id, hit.release_date, None))
                }
            }
            .await;
            match result {
                Ok((id, release_date, cover)) => {
                    game.sgdb = Some(ogm_core::SgdbInfo {
                        id,
                        release_date,
                        cover,
                        hero: None,
                    });
                }
                Err(e) => errors.push(format!("sgdb {}: {e}", game.id)),
            }
        }
    }

    state.generated_at = now;
    state.errors = errors;
    state.save(paths).context("writing state.json")?;
    if !state.errors.is_empty() {
        for e in &state.errors {
            eprintln!("warning: {e}");
        }
    }
    Ok(())
}

fn run_launch(paths: &Paths, id: &str) -> Result<()> {
    let mut state = State::load(paths).context("loading state.json")?;
    let Some(game) = state.games.iter_mut().find(|g| g.id == id) else {
        bail!("no game with id {id:?} in state.json (run `ogm scan`?)");
    };
    let exec = game.exec.clone();
    game.last_played = Some(now_rfc3339());
    game.play_count += 1;
    state.save(paths).context("writing state.json")?;
    std::process::Command::new("setsid")
        .args(["sh", "-c", &exec])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("spawning game process")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_add(
    paths: &Paths,
    name: &str,
    exec: &str,
    category: Option<&str>,
    github: Option<&str>,
    sgdb_query: Option<&str>,
    icon: Option<&str>,
) -> Result<String> {
    let mut user = UserGames::load(paths).context("loading games.json")?;
    let state = State::load(paths).context("loading state.json")?;
    let config = Config::load(paths).context("loading config.toml")?;
    let (catalog, _) = load_full_catalog(paths, &config)?;
    let taken: HashSet<String> = state
        .games
        .iter()
        .map(|g| g.id.clone())
        .chain(catalog.iter().map(|c| c.id.clone()))
        .chain(user.custom.iter().map(|c| c.id.clone()))
        .collect();
    let id = unique_id(&slugify(name), &taken);
    if id.is_empty() {
        bail!("could not derive an id from name {name:?}");
    }
    user.custom.push(CustomGame {
        id: id.clone(),
        name: name.to_string(),
        exec: exec.to_string(),
        category: category
            .map(parse_category)
            .transpose()?
            .unwrap_or(Category::Custom),
        github: github.map(|s| s.to_string()),
        sgdb_query: sgdb_query.map(|s| s.to_string()),
        icon: icon.map(|s| s.to_string()),
    });
    user.save(paths).context("writing games.json")?;
    run_scan(paths)?;
    Ok(id)
}

fn run_remove(paths: &Paths, id: &str) -> Result<()> {
    let mut user = UserGames::load(paths).context("loading games.json")?;
    let before = user.custom.len();
    user.custom.retain(|c| c.id != id);
    if user.custom.len() == before {
        user.hide(id);
    }
    user.save(paths).context("writing games.json")?;
    run_scan(paths)?;
    Ok(())
}

fn run_hide(paths: &Paths, id: &str, hide: bool) -> Result<()> {
    let mut user = UserGames::load(paths).context("loading games.json")?;
    if hide {
        user.hide(id);
    } else {
        user.show(id);
    }
    user.save(paths).context("writing games.json")?;
    run_scan(paths)?;
    Ok(())
}

fn run_doctor(paths: &Paths) -> Result<()> {
    let config = Config::load(paths).context("loading config.toml")?;
    let user = UserGames::load(paths).context("loading games.json")?;
    let (catalog, fragments) = load_full_catalog(paths, &config)?;
    let discovered = scan_applications_dirs(&config);
    let state = State::load(paths).context("loading state.json")?;
    println!("config:      {}", paths.config_file().display());
    println!("games.json:  {}", paths.games_file().display());
    println!("state.json:  {}", paths.state_file().display());
    println!("covers:      {}", paths.covers_dir().display());
    println!(
        "sgdb key:    {}",
        if config.sgdb_api_key.is_empty() {
            "missing"
        } else {
            "present"
        }
    );
    println!(
        "github token:{}",
        if config.github_token.is_none() {
            " missing"
        } else {
            " present"
        }
    );
    println!("refresh:     every {}h", config.refresh_interval_hours);
    println!("applications dirs:");
    for d in &config.applications_dirs {
        println!("  {}", d.display());
    }
    println!(
        "catalog entries: {} ({} from fragments)",
        catalog.len(),
        fragments.entries.len()
    );
    println!("catalog dirs:");
    for d in config.all_catalog_dirs(paths) {
        println!("  {}", d.display());
    }
    for (file, count) in &fragments.files {
        println!("  {} -> {} entries", file.display(), count);
    }
    for w in &fragments.warnings {
        println!("  warning: {w}");
    }
    println!("discovered:        {}", discovered.len());
    println!(
        "  via X-OGM marker: {}",
        discovered.iter().filter(|d| d.via_marker).count()
    );
    println!(
        "  via legacy glob only: {}",
        discovered.iter().filter(|d| !d.via_marker).count()
    );
    println!("custom:            {}", user.custom.len());
    println!("hidden:            {}", user.hidden.len());
    println!("state games:       {}", state.games.len());
    Ok(())
}

fn run_list(paths: &Paths, json: bool) -> Result<()> {
    let state = State::load(paths).context("loading state.json")?;
    if json {
        println!("{}", serde_json::to_string_pretty(&state.games)?);
        return Ok(());
    }
    for game in &state.games {
        let update = if game.github.as_ref().map(|g| g.has_update).unwrap_or(false) {
            " [update]"
        } else {
            ""
        };
        println!(
            "{:28} {:9} {}{}",
            game.id,
            game.category.as_str(),
            game.name,
            update
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = Paths::from_xdg().context("resolving XDG directories")?;
    match cli.command {
        Command::Scan { json } => {
            let state = run_scan(&paths)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&state)?);
            } else {
                println!(
                    "{} games -> {}",
                    state.games.len(),
                    paths.state_file().display()
                );
            }
        }
        Command::Refresh { force } => run_refresh(&paths, force).await?,
        Command::Launch { id } => run_launch(&paths, &id)?,
        Command::Add {
            name,
            exec,
            category,
            github,
            sgdb_query,
            icon,
        } => {
            let id = run_add(
                &paths,
                &name,
                &exec,
                category.as_deref(),
                github.as_deref(),
                sgdb_query.as_deref(),
                icon.as_deref(),
            )?;
            println!("{id}");
        }
        Command::Remove { id } => run_remove(&paths, &id)?,
        Command::Hide { id } => run_hide(&paths, &id, true)?,
        Command::Show { id } => run_hide(&paths, &id, false)?,
        Command::Doctor => run_doctor(&paths)?,
        Command::List { json } => run_list(&paths, json)?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("My Cool Game!"), "my-cool-game");
        assert_eq!(slugify("Sonic 3 A.I.R."), "sonic-3-a-i-r");
        assert_eq!(slugify("  spaced  "), "spaced");
    }

    #[test]
    fn unique_id_suffixes_on_collision() {
        let taken = HashSet::from(["game".to_string(), "game-2".to_string()]);
        assert_eq!(unique_id("game", &taken), "game-3");
        assert_eq!(unique_id("fresh", &taken), "fresh");
    }

    #[test]
    fn parses_categories_from_cli_strings() {
        assert_eq!(parse_category("wine").unwrap(), Category::Wine);
        assert!(parse_category("nope").is_err());
    }

    fn marked(stem: &str, content: &str) -> DiscoveredEntry {
        DiscoveredEntry {
            entry: desktop::parse_desktop(content),
            stem: stem.into(),
            via_marker: true,
        }
    }

    fn catalog_fixture() -> Vec<CatalogEntry> {
        vec![CatalogEntry {
            id: "soh".into(),
            desktop_id: Some("gaming-soh".into()),
            name: Some("Ship of Harkinian".into()),
            category: Category::Port,
            github: Some("HarbourMasters/Shipwright".into()),
            sgdb_query: Some("Ship of Harkinian".into()),
            version_file: None,
            hidden_default: false,
        }]
    }

    #[test]
    fn marker_category_overrides_catalog_category() {
        let d = marked(
            "gaming-soh",
            "[Desktop Entry]\nName=SoH\nX-OGM-Managed=true\nX-OGM-Category=wine\n",
        );
        let overlay = synthesize_marker_overlay(&[d], &catalog_fixture());
        assert_eq!(overlay.len(), 1);
        assert_eq!(overlay[0].id, "soh", "catalog id carries over");
        assert_eq!(overlay[0].category, Category::Wine);
        assert_eq!(
            overlay[0].github.as_deref(),
            Some("HarbourMasters/Shipwright"),
            "github falls through from catalog"
        );
        assert_eq!(overlay[0].name.as_deref(), Some("Ship of Harkinian"));
    }

    #[test]
    fn marker_github_and_sgdb_override_catalog() {
        let d = marked(
            "gaming-soh",
            "[Desktop Entry]\nName=SoH\nX-OGM-Managed=true\nX-OGM-GitHub=fork/soh\nX-OGM-SGDBQuery=SoH\n",
        );
        let overlay = synthesize_marker_overlay(&[d], &catalog_fixture());
        assert_eq!(overlay[0].github.as_deref(), Some("fork/soh"));
        assert_eq!(overlay[0].sgdb_query.as_deref(), Some("SoH"));
        assert_eq!(overlay[0].category, Category::Port, "catalog category kept");
    }

    #[test]
    fn marker_without_category_or_catalog_defaults_custom() {
        let d = marked("zz-new", "[Desktop Entry]\nName=New\nX-OGM-Managed=true\n");
        let overlay = synthesize_marker_overlay(&[d], &catalog_fixture());
        assert_eq!(overlay[0].id, "zz-new");
        assert_eq!(overlay[0].category, Category::Custom);
        assert!(overlay[0].github.is_none());
    }

    #[test]
    fn unmarked_entries_produce_no_overlay() {
        let content = "[Desktop Entry]\nName=X\nX-OGM-Managed=yes\n";
        let d = DiscoveredEntry {
            entry: desktop::parse_desktop(content),
            stem: "gaming-soh".into(),
            via_marker: false,
        };
        assert!(synthesize_marker_overlay(&[d], &catalog_fixture()).is_empty());
    }
}
