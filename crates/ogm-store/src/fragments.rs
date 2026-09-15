use std::fs;
use std::path::PathBuf;

use ogm_core::CatalogEntry;

use crate::config::parse_catalog;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CatalogFragments {
    /// Entries in load order (dir order, then filename order within a dir).
    pub entries: Vec<CatalogEntry>,
    /// Successfully parsed files with their entry counts, in load order.
    pub files: Vec<(PathBuf, usize)>,
    /// Human-readable warnings for files that failed to parse.
    pub warnings: Vec<String>,
}

/// Load every `*.toml` fragment from each dir (missing dirs are fine),
/// files sorted by name within a dir for deterministic merge order.
/// Unparseable files are skipped and reported in `warnings`.
pub fn load_catalog_fragments(dirs: &[PathBuf]) -> CatalogFragments {
    let mut out = CatalogFragments::default();
    for dir in dirs {
        let Ok(read_dir) = fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<PathBuf> = read_dir
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("toml"))
            .collect();
        files.sort();
        for file in files {
            let parsed = fs::read_to_string(&file)
                .ok()
                .and_then(|text| parse_catalog(&text).ok());
            match parsed {
                Some(entries) => {
                    out.files.push((file, entries.len()));
                    out.entries.extend(entries);
                }
                None => out.warnings.push(format!(
                    "catalog fragment {} failed to parse",
                    file.display()
                )),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fragment(id: &str, name: &str) -> String {
        format!("[[game]]\nid = \"{id}\"\ncategory = \"port\"\nname = \"{name}\"\n")
    }

    #[test]
    fn missing_dir_is_fine() {
        let dir = tempfile::tempdir().unwrap();
        let out = load_catalog_fragments(&[dir.path().join("nope")]);
        assert!(out.entries.is_empty());
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn fragments_load_in_filename_order() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("20-second.toml"), fragment("b", "B")).unwrap();
        fs::write(dir.path().join("10-first.toml"), fragment("a", "A")).unwrap();
        fs::write(dir.path().join("ignored.txt"), "not toml").unwrap();
        let out = load_catalog_fragments(&[dir.path().to_path_buf()]);
        let ids: Vec<&str> = out.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(out.files.len(), 2);
        assert_eq!(out.files[0].1, 1);
        assert!(out.warnings.is_empty());
    }

    #[test]
    fn bad_toml_is_skipped_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("good.toml"), fragment("a", "A")).unwrap();
        fs::write(dir.path().join("bad.toml"), "[[game]\nnot toml at all").unwrap();
        let out = load_catalog_fragments(&[dir.path().to_path_buf()]);
        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.warnings.len(), 1);
        assert!(out.warnings[0].contains("bad.toml"));
    }

    #[test]
    fn multiple_dirs_load_in_dir_order() {
        let tmp = tempfile::tempdir().unwrap();
        let d1 = tmp.path().join("one");
        let d2 = tmp.path().join("two");
        fs::create_dir_all(&d1).unwrap();
        fs::create_dir_all(&d2).unwrap();
        fs::write(d2.join("x.toml"), fragment("second-dir", "2")).unwrap();
        fs::write(d1.join("x.toml"), fragment("first-dir", "1")).unwrap();
        let out = load_catalog_fragments(&[d1, d2]);
        let ids: Vec<&str> = out.entries.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["first-dir", "second-dir"]);
    }
}
