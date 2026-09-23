use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::model::Game;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    Name,
    DateAdded,
    ReleaseDate,
    MostPlayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDir {
    Asc,
    Desc,
}

const DAY: i64 = 86_400;

/// Frecency: `count * decay`. Decay steps (mirrored by the QML frontend,
/// keep in sync): 1.0 if played within 7 days, 0.6 within 30, 0.3 within
/// 90, 0.1 if older; 0.0 when never played or the timestamp is garbage.
pub fn frecency_score(play_count: u64, last_played: Option<&str>, now: i64) -> f64 {
    if play_count == 0 {
        return 0.0;
    }
    let Some(ts) = last_played.and_then(crate::time::rfc3339_to_epoch) else {
        return 0.0;
    };
    let age = now - ts;
    let decay = if age <= 7 * DAY {
        1.0
    } else if age <= 30 * DAY {
        0.6
    } else if age <= 90 * DAY {
        0.3
    } else {
        0.1
    };
    play_count as f64 * decay
}

fn compare_release_date(a: &Game, b: &Game, asc: bool) -> Ordering {
    let da = a.sgdb.as_ref().and_then(|s| s.release_date.as_deref());
    let db = b.sgdb.as_ref().and_then(|s| s.release_date.as_deref());
    match (da, db) {
        (Some(x), Some(y)) => {
            let ord = x.cmp(y);
            if asc {
                ord
            } else {
                ord.reverse()
            }
        }
        // nulls sort last in both directions
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_most_played(a: &Game, b: &Game, asc: bool, now: i64) -> Ordering {
    let sa = frecency_score(a.play_count, a.last_played.as_deref(), now);
    let sb = frecency_score(b.play_count, b.last_played.as_deref(), now);
    match (sa == 0.0, sb == 0.0) {
        // never-played sorts last in both directions
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            // asc = most played at top
            let ord = sb.partial_cmp(&sa).unwrap_or(Ordering::Equal);
            if asc {
                ord
            } else {
                ord.reverse()
            }
        }
    }
}

pub fn sort_games(games: &mut [Game], key: SortKey, dir: SortDir, now: i64) {
    let asc = matches!(dir, SortDir::Asc);
    games.sort_by(|a, b| {
        let ord = match key {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::DateAdded => a.added_at.cmp(&b.added_at),
            SortKey::ReleaseDate => compare_release_date(a, b, asc),
            SortKey::MostPlayed => compare_most_played(a, b, asc, now),
        };
        let ord = match key {
            SortKey::ReleaseDate | SortKey::MostPlayed => ord,
            _ if asc => ord,
            _ => ord.reverse(),
        };
        if ord == Ordering::Equal {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else {
            ord
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Category, Game, SgdbInfo};

    const NOW: i64 = 1_789_300_800; // 2026-09-13T12:00:00Z

    fn days_ago(days: i64) -> String {
        crate::time::epoch_to_rfc3339(NOW - days * DAY)
    }

    fn game(id: &str, name: &str, added: &str, release: Option<&str>) -> Game {
        Game {
            id: id.into(),
            name: name.into(),
            category: Category::Port,
            exec: "true".into(),
            icon: "i".into(),
            desktop_id: None,
            custom: false,
            added_at: added.into(),
            last_played: None,
            play_count: 0,
            installed_version: None,
            github: None,
            sgdb: release.map(|d| SgdbInfo {
                id: 1,
                release_date: Some(d.into()),
                cover: None,
                hero: None,
            }),
            web: None,
        }
    }

    fn ids(games: &[Game]) -> Vec<&str> {
        games.iter().map(|g| g.id.as_str()).collect()
    }

    #[test]
    fn sorts_by_name_case_insensitive() {
        let mut g = vec![
            game("b", "banjo", "2026-01-01", None),
            game("a", "Azahar", "2026-01-01", None),
            game("c", "Cemu", "2026-01-01", None),
        ];
        sort_games(&mut g, SortKey::Name, SortDir::Asc, NOW);
        assert_eq!(ids(&g), ["a", "b", "c"]);
        sort_games(&mut g, SortKey::Name, SortDir::Desc, NOW);
        assert_eq!(ids(&g), ["c", "b", "a"]);
    }

    #[test]
    fn sorts_by_date_added() {
        let mut g = vec![
            game("new", "N", "2026-09-01", None),
            game("old", "O", "2025-01-01", None),
        ];
        sort_games(&mut g, SortKey::DateAdded, SortDir::Asc, NOW);
        assert_eq!(ids(&g), ["old", "new"]);
        sort_games(&mut g, SortKey::DateAdded, SortDir::Desc, NOW);
        assert_eq!(ids(&g), ["new", "old"]);
    }

    #[test]
    fn release_date_nulls_last_regardless_of_direction() {
        let mut g = vec![
            game("none", "NoDate", "x", None),
            game("late", "Late", "x", Some("2005-05-01")),
            game("early", "Early", "x", Some("1993-11-01")),
        ];
        sort_games(&mut g, SortKey::ReleaseDate, SortDir::Asc, NOW);
        assert_eq!(ids(&g), ["early", "late", "none"]);
        sort_games(&mut g, SortKey::ReleaseDate, SortDir::Desc, NOW);
        assert_eq!(ids(&g), ["late", "early", "none"]);
    }

    #[test]
    fn release_date_ties_fall_back_to_name() {
        let mut g = vec![
            game("z", "Zelda", "x", Some("2000-01-01")),
            game("a", "Alto", "x", Some("2000-01-01")),
        ];
        sort_games(&mut g, SortKey::ReleaseDate, SortDir::Asc, NOW);
        assert_eq!(ids(&g), ["a", "z"]);
    }

    #[test]
    fn frecency_decay_boundaries() {
        for (days, want) in [
            (7, 1.0),
            (8, 0.6),
            (30, 0.6),
            (31, 0.3),
            (90, 0.3),
            (91, 0.1),
        ] {
            let ts = days_ago(days);
            assert_eq!(
                frecency_score(10, Some(&ts), NOW),
                10.0 * want,
                "day {days}"
            );
        }
    }

    #[test]
    fn frecency_zero_for_never_played_or_garbage() {
        assert_eq!(frecency_score(0, Some(&days_ago(1)), NOW), 0.0);
        assert_eq!(frecency_score(5, None, NOW), 0.0);
        assert_eq!(frecency_score(5, Some("not-a-date"), NOW), 0.0);
    }

    fn played(id: &str, name: &str, count: u64, days: i64) -> Game {
        let mut g = game(id, name, "x", None);
        g.play_count = count;
        g.last_played = Some(days_ago(days));
        g
    }

    #[test]
    fn most_played_asc_puts_highest_score_first_never_played_last() {
        let mut g = vec![
            game("never", "Never", "x", None),
            played("warm", "Warm", 2, 2),   // 2 * 1.0
            played("hot", "Hot", 10, 40),   // 10 * 0.3 = 3.0
            played("cold", "Cold", 1, 200), // 1 * 0.1
        ];
        sort_games(&mut g, SortKey::MostPlayed, SortDir::Asc, NOW);
        assert_eq!(ids(&g), ["hot", "warm", "cold", "never"]);
        sort_games(&mut g, SortKey::MostPlayed, SortDir::Desc, NOW);
        assert_eq!(ids(&g), ["cold", "warm", "hot", "never"]);
    }

    #[test]
    fn most_played_ties_break_by_name() {
        let mut g = vec![played("z", "Zelda", 3, 1), played("a", "Alto", 3, 1)];
        sort_games(&mut g, SortKey::MostPlayed, SortDir::Asc, NOW);
        assert_eq!(ids(&g), ["a", "z"]);
    }

    #[test]
    fn most_played_zero_count_with_timestamp_is_never_played() {
        let mut g = vec![played("zero", "Zero", 0, 1), played("one", "One", 1, 60)];
        sort_games(&mut g, SortKey::MostPlayed, SortDir::Asc, NOW);
        assert_eq!(ids(&g), ["one", "zero"]);
    }

    #[test]
    fn sort_key_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&SortKey::MostPlayed).unwrap(),
            "\"most_played\""
        );
        let k: SortKey = serde_json::from_str("\"most_played\"").unwrap();
        assert_eq!(k, SortKey::MostPlayed);
    }
}
