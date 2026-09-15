use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Port,
    Decomp,
    Recomp,
    Fangame,
    Wine,
    Arcade,
    Emulator,
    Tool,
    Custom,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::Port => "port",
            Category::Decomp => "decomp",
            Category::Recomp => "recomp",
            Category::Fangame => "fangame",
            Category::Wine => "wine",
            Category::Arcade => "arcade",
            Category::Emulator => "emulator",
            Category::Tool => "tool",
            Category::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GithubInfo {
    pub repo: String,
    #[serde(default)]
    pub latest_tag: Option<String>,
    #[serde(default)]
    pub latest_url: Option<String>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub checked_at: Option<String>,
    #[serde(default)]
    pub has_update: bool,
}

impl GithubInfo {
    pub fn unchecked(repo: impl Into<String>) -> Self {
        GithubInfo {
            repo: repo.into(),
            latest_tag: None,
            latest_url: None,
            published_at: None,
            checked_at: None,
            has_update: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SgdbInfo {
    pub id: u64,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub hero: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub category: Category,
    pub exec: String,
    pub icon: String,
    pub desktop_id: Option<String>,
    pub custom: bool,
    pub added_at: String,
    #[serde(default)]
    pub last_played: Option<String>,
    #[serde(default)]
    pub play_count: u64,
    #[serde(default)]
    pub installed_version: Option<String>,
    #[serde(default)]
    pub github: Option<GithubInfo>,
    #[serde(default)]
    pub sgdb: Option<SgdbInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomGame {
    pub id: String,
    pub name: String,
    pub exec: String,
    #[serde(default = "default_custom_category")]
    pub category: Category,
    #[serde(default)]
    pub github: Option<String>,
    #[serde(default)]
    pub sgdb_query: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
}

fn default_custom_category() -> Category {
    Category::Custom
}

pub const DEFAULT_ICON: &str = "applications-games";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&Category::Fangame).unwrap(),
            "\"fangame\""
        );
        let c: Category = serde_json::from_str("\"recomp\"").unwrap();
        assert_eq!(c, Category::Recomp);
    }

    #[test]
    fn game_schema_matches_contract() {
        let game = Game {
            id: "ship-of-harkinian".into(),
            name: "Zelda: Ocarina of Time - Ship of Harkinian".into(),
            category: Category::Port,
            exec: "/usr/bin/distrobox-enter -n gaming -- soh".into(),
            icon: "applications-games".into(),
            desktop_id: Some("gaming-ship-of-harkinian".into()),
            custom: false,
            added_at: "2026-09-13T12:00:00Z".into(),
            last_played: None,
            play_count: 0,
            installed_version: None,
            github: Some(GithubInfo {
                repo: "HarbourMasters/Shipwright".into(),
                latest_tag: Some("9.1.2".into()),
                latest_url: Some("https://github.com/x/releases/tag/9.1.2".into()),
                published_at: Some("2026-09-01T00:00:00Z".into()),
                checked_at: Some("2026-09-13T12:00:00Z".into()),
                has_update: false,
            }),
            sgdb: Some(SgdbInfo {
                id: 5234567,
                release_date: Some("1998-11-23".into()),
                cover: Some("/home/u/.cache/ogm/covers/ship-of-harkinian.jpg".into()),
                hero: None,
            }),
        };
        let v: serde_json::Value = serde_json::to_value(&game).unwrap();
        assert_eq!(v["category"], "port");
        assert_eq!(v["custom"], false);
        assert_eq!(v["play_count"], 0);
        assert_eq!(v["last_played"], serde_json::Value::Null);
        assert_eq!(v["github"]["repo"], "HarbourMasters/Shipwright");
        assert_eq!(v["github"]["has_update"], false);
        assert_eq!(v["sgdb"]["id"], 5234567);
        assert_eq!(v["sgdb"]["hero"], serde_json::Value::Null);
        let back: Game = serde_json::from_value(v).unwrap();
        assert_eq!(back, game);
    }

    #[test]
    fn optional_blocks_round_trip_as_null() {
        let json = r#"{
            "id":"x","name":"X","category":"custom","exec":"true",
            "icon":"applications-games","desktop_id":null,"custom":true,
            "added_at":"2026-01-01T00:00:00Z","last_played":null,
            "installed_version":null,"github":null,"sgdb":null
        }"#;
        let g: Game = serde_json::from_str(json).unwrap();
        assert!(g.github.is_none() && g.sgdb.is_none() && g.desktop_id.is_none());
        assert_eq!(g.play_count, 0, "old state files default play_count to 0");
        let round: Game = serde_json::from_str(&serde_json::to_string(&g).unwrap()).unwrap();
        assert_eq!(round, g);
    }
}
