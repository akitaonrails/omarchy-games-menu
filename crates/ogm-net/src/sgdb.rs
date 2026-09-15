use async_trait::async_trait;

use crate::NetError;

const BASE: &str = "https://www.steamgriddb.com/api/v2";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SgdbGame {
    pub id: u64,
    pub name: String,
    pub release_date: Option<String>,
    pub types: Vec<String>,
}

#[async_trait]
pub trait ArtworkSource: Send + Sync {
    /// Candidates ordered exact-name-match first, then autocomplete order.
    async fn search_games(&self, query: &str) -> Result<Vec<SgdbGame>, NetError>;
    async fn best_grid(&self, game_id: u64) -> Result<Option<String>, NetError>;
    async fn best_hero(&self, game_id: u64) -> Result<Option<String>, NetError>;
    async fn download(&self, url: &str, dest: &std::path::Path) -> Result<(), NetError>;
}

#[derive(Debug, Clone)]
pub struct SgdbClient {
    client: reqwest::Client,
    api_key: String,
}

impl SgdbClient {
    pub fn new(api_key: impl Into<String>) -> Result<Self, NetError> {
        let client = reqwest::Client::builder().user_agent("ogm").build()?;
        Ok(SgdbClient {
            client,
            api_key: api_key.into(),
        })
    }

    async fn get_json(&self, path: &str) -> Result<String, NetError> {
        if self.api_key.is_empty() {
            return Err(NetError::MissingApiKey);
        }
        let resp = self
            .client
            .get(format!("{BASE}{path}"))
            .bearer_auth(&self.api_key)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(NetError::Status(resp.status()));
        }
        Ok(resp.text().await?)
    }
}

/// Parse /search/autocomplete/{term}; returns all hits, exact
/// (case-insensitive) name matches for `query` first, autocomplete order
/// otherwise preserved.
pub fn parse_search_response(query: &str, body: &str) -> Result<Vec<SgdbGame>, NetError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| NetError::Parse(e.to_string()))?;
    let data = match v.get("data").and_then(|d| d.as_array()) {
        Some(arr) => arr,
        None => return Ok(Vec::new()),
    };
    let mut games = Vec::new();
    for hit in data {
        let Some(id) = hit.get("id").and_then(|i| i.as_u64()) else {
            continue;
        };
        let name = hit
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let release_date = hit
            .get("release_date")
            .and_then(|d| d.as_i64())
            .filter(|d| *d > 0)
            .map(ogm_core::time::epoch_to_date);
        let types = hit
            .get("types")
            .and_then(|t| t.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        games.push(SgdbGame {
            id,
            name,
            release_date,
            types,
        });
    }
    let query = query.trim().to_lowercase();
    games.sort_by_key(|g| if g.name.to_lowercase() == query { 0 } else { 1 });
    Ok(games)
}

/// Parse /grids/game/{id} or /heroes/game/{id}; first asset URL wins.
pub fn parse_asset_response(body: &str) -> Result<Option<String>, NetError> {
    let v: serde_json::Value =
        serde_json::from_str(body).map_err(|e| NetError::Parse(e.to_string()))?;
    let url = v
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("url"))
        .and_then(|u| u.as_str())
        .map(|s| s.to_string());
    Ok(url)
}

#[async_trait]
impl ArtworkSource for SgdbClient {
    async fn search_games(&self, query: &str) -> Result<Vec<SgdbGame>, NetError> {
        let body = self
            .get_json(&format!("/search/autocomplete/{query}"))
            .await?;
        parse_search_response(query, &body)
    }

    async fn best_grid(&self, game_id: u64) -> Result<Option<String>, NetError> {
        let body = self
            .get_json(&format!("/grids/game/{game_id}?dimensions=600x900"))
            .await?;
        parse_asset_response(&body)
    }

    async fn best_hero(&self, game_id: u64) -> Result<Option<String>, NetError> {
        let body = self.get_json(&format!("/heroes/game/{game_id}")).await?;
        parse_asset_response(&body)
    }

    async fn download(&self, url: &str, dest: &std::path::Path) -> Result<(), NetError> {
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(NetError::Status(resp.status()));
        }
        let bytes = resp.bytes().await?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| NetError::Parse(format!("mkdir {}: {e}", parent.display())))?;
        }
        std::fs::write(dest, bytes)
            .map_err(|e| NetError::Parse(format!("write {}: {e}", dest.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEARCH_JSON: &str = r#"{
        "success": true,
        "data": [
            {
                "id": 5234567,
                "name": "The Legend of Zelda: Ocarina of Time",
                "release_date": 911779200,
                "types": ["steam", "gog"],
                "verified": true
            },
            {"id": 999, "name": "Ocarina Clone", "release_date": 0, "types": []}
        ]
    }"#;

    const GRIDS_JSON: &str = r#"{
        "success": true,
        "data": [
            {
                "id": 111222,
                "score": 42,
                "width": 600,
                "height": 900,
                "url": "https://cdn2.steamgriddb.com/grid/abc123def456.jpg",
                "thumb": "https://cdn2.steamgriddb.com/thumb/abc123def456.jpg"
            },
            {"id": 111223, "url": "https://cdn2.steamgriddb.com/grid/zzz.png"}
        ]
    }"#;

    #[test]
    fn parses_search_fixture_with_epoch_release_date() {
        let games = parse_search_response("ocarina of time", SEARCH_JSON).unwrap();
        assert_eq!(games.len(), 2);
        let game = &games[0];
        assert_eq!(game.id, 5234567);
        assert_eq!(game.name, "The Legend of Zelda: Ocarina of Time");
        assert_eq!(game.release_date.as_deref(), Some("1998-11-23"));
        assert_eq!(game.types, vec!["steam", "gog"]);
    }

    #[test]
    fn exact_name_match_sorts_first() {
        let body = r#"{"success":true,"data":[
            {"id": 15166, "name": "Wipeout Omega Collection", "release_date": 1496707200, "types": ["steam"]},
            {"id": 5296247, "name": "Wipeout", "release_date": 812592000, "types": []}
        ]}"#;
        let games = parse_search_response("Wipeout", body).unwrap();
        assert_eq!(
            games[0].id, 5296247,
            "exact match wins over autocomplete order"
        );
        assert_eq!(games[1].id, 15166);
        // no exact match: autocomplete order preserved
        let games = parse_search_response("wipe", body).unwrap();
        assert_eq!(games[0].id, 15166);
    }

    #[test]
    fn empty_search_yields_no_candidates() {
        assert!(parse_search_response("x", r#"{"success":true,"data":[]}"#)
            .unwrap()
            .is_empty());
        assert!(parse_search_response("x", r#"{"success":false}"#)
            .unwrap()
            .is_empty());
        assert!(parse_search_response("x", "not json").is_err());
    }

    #[test]
    fn parses_grids_fixture_first_url() {
        assert_eq!(
            parse_asset_response(GRIDS_JSON).unwrap().as_deref(),
            Some("https://cdn2.steamgriddb.com/grid/abc123def456.jpg")
        );
        assert!(parse_asset_response(r#"{"success":true,"data":[]}"#)
            .unwrap()
            .is_none());
    }
}
