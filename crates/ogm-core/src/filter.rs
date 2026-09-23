use crate::model::{Category, Game};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameFilter {
    pub categories: Vec<Category>,
    pub query: String,
}

impl GameFilter {
    pub fn matches(&self, game: &Game) -> bool {
        let category_ok = self.categories.is_empty() || self.categories.contains(&game.category);
        let query = self.query.trim();
        let query_ok = query.is_empty() || game.name.to_lowercase().contains(&query.to_lowercase());
        category_ok && query_ok
    }
}

pub fn filter_games<'a>(games: &'a [Game], filter: &GameFilter) -> Vec<&'a Game> {
    games.iter().filter(|g| filter.matches(g)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Game;

    fn game(id: &str, name: &str, category: Category) -> Game {
        Game {
            id: id.into(),
            name: name.into(),
            category,
            exec: "true".into(),
            icon: "i".into(),
            desktop_id: None,
            custom: false,
            added_at: "2026-01-01T00:00:00Z".into(),
            last_played: None,
            play_count: 0,
            installed_version: None,
            github: None,
            sgdb: None,
            web: None,
        }
    }

    fn fixture() -> Vec<Game> {
        vec![
            game("soh", "Ship of Harkinian", Category::Port),
            game("dkc", "Donkey Kong Country Recomp", Category::Recomp),
            game("sorr", "Streets of Rage Remake", Category::Wine),
        ]
    }

    #[test]
    fn empty_filter_matches_all() {
        let g = fixture();
        assert_eq!(filter_games(&g, &GameFilter::default()).len(), 3);
    }

    #[test]
    fn filters_by_category_subset() {
        let g = fixture();
        let f = GameFilter {
            categories: vec![Category::Port, Category::Wine],
            query: String::new(),
        };
        let out = filter_games(&g, &f);
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|g| g.category != Category::Recomp));
    }

    #[test]
    fn filters_by_case_insensitive_substring() {
        let g = fixture();
        let f = GameFilter {
            categories: vec![],
            query: "SHIP".into(),
        };
        let out = filter_games(&g, &f);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, "soh");
    }

    #[test]
    fn combines_category_and_query() {
        let g = fixture();
        let f = GameFilter {
            categories: vec![Category::Recomp],
            query: "donkey".into(),
        };
        let out = filter_games(&g, &f);
        assert_eq!(out.len(), 1);
        let f2 = GameFilter {
            categories: vec![Category::Port],
            query: "donkey".into(),
        };
        assert!(filter_games(&g, &f2).is_empty());
    }
}
