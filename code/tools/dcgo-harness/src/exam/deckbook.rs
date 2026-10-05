//! Resolves a scenario's `rest:` seat name to a concrete deck list.
//!
//! Moved out of `main.rs` (2026-08-28, MCP task 6) so the CLI's `exam` command
//! and the MCP's `run_scenario` / `exam_probe` build the SAME deck for the
//! SAME scenario. A second copy of this resolution living in the MCP handler
//! could silently drift from the CLI's -- different remainder ordering,
//! different starter-deck fallback -- and that would change the game a
//! scenario lowers against depending on which caller ran it, exactly the
//! class of tooling-artifact "divergence" this project keeps having to rule
//! out.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::exam::scenario::ScenarioSeat;
use crate::pool;

/// The deck book (a `submit --decks` pool file) that names every deck in
/// `rest` -- the scenario's `rest:` seat names -- searched the way the CI
/// sim-only driver (`code/tools/exam_sim_all.py`) searches: books beside the
/// scenario first, then every other `*.json` pool under `exams_root`, in path
/// order. `None` when no pool names them all (the stock starter decks, which
/// every [`DeckBook`] carries, may still).
///
/// Per-set books reuse names for different lists, which is why the scenario's
/// own directory wins; pass an explicit book when two others would both match.
pub fn book_for(scenario: &Path, rest: &[&str], exams_root: &Path) -> Option<PathBuf> {
    let needed: Vec<String> = rest.iter().map(|r| r.to_lowercase()).collect();
    let mut books = Vec::new();
    let mut stack = vec![exams_root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().and_then(|e| e.to_str()) == Some("json") {
                books.push(p);
            }
        }
    }
    let here = scenario.parent().map(|d| d.to_path_buf());
    let same_dir = |p: &Path| match (&here, p.parent()) {
        (Some(h), Some(d)) => std::fs::canonicalize(h).ok() == std::fs::canonicalize(d).ok(),
        _ => false,
    };
    books.sort_by_key(|p| (!same_dir(p), p.clone()));
    books.into_iter().find(|p| {
        pool::load_pool(p)
            .map(|pool| {
                let names: Vec<String> = pool.decks.iter().map(|d| d.name.to_lowercase()).collect();
                needed.iter().all(|n| names.contains(n))
            })
            .unwrap_or(false)
    })
}

/// One named deck, main and eggs kept apart. [`ordered_deck`] flattens them
/// for our engine (`Game::new_inner` re-splits by card kind), while
/// `--emit-job` needs the boundary: a DCGO job's flat list is main-then-eggs
/// by convention, and a job silently emitted with no eggs would run a
/// different game than the scenario claims.
#[derive(Debug, Clone)]
pub struct DeckEntry {
    pub main: Vec<String>,
    pub eggs: Vec<String>,
}

/// Resolves a scenario's `rest:` name to a card list.
///
/// Two sources, both already in the repo: the harness deck-pool JSON that
/// `submit --decks` consumes, and `data/starter_decks.json`. Nothing here
/// invents a deck — an unknown name is an error naming what IS available,
/// because a silently-substituted deck would change the line under test.
pub struct DeckBook {
    /// lowercased name -> deck entry.
    by_name: BTreeMap<String, DeckEntry>,
    source: String,
}

#[derive(Debug, Deserialize)]
struct StarterDeckFile {
    starter_decks: Vec<StarterDeck>,
}

#[derive(Debug, Deserialize)]
struct StarterDeck {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    set: String,
    #[serde(default)]
    main_deck: Vec<String>,
    #[serde(default)]
    egg_deck: Vec<String>,
}

impl DeckBook {
    /// Build the deck book as a UNION: the stock starter decks first, then any
    /// `decks` pool overlaid on top (the pool wins on a name collision).
    ///
    /// It used to be either/or -- supplying `decks` returned early with ONLY
    /// the pool -- which made a directory-wide run impossible. The committed
    /// corpus spans both books: the EX12 scenarios name `toho-braves` /
    /// `toho-analog` / `toho-matt` / `st19-arisa` from
    /// `qa/dcgo-exams/EX12/toho_pool.json`, while `qa/dcgo-exams/ST1/*` name
    /// `starter_st1_gaia_red` from `data/starter_decks.json`. Under either/or
    /// there was no single invocation that could lower all 144 scenarios: the
    /// pool book failed the 3 ST1 files and the default book failed the other
    /// 141. Since `exam --scenario <dir>` is exactly how CI runs the corpus,
    /// that alone kept the gate unusable even once its missing `--root` was
    /// fixed.
    pub fn load(decks: Option<&Path>, cards_json: &Path) -> Result<DeckBook, String> {
        let mut by_name: BTreeMap<String, DeckEntry> = BTreeMap::new();
        let mut sources: Vec<String> = Vec::new();

        // Base layer: the stock starter decks. Required when no pool is given
        // (there would otherwise be no book at all); best-effort when one is,
        // so a pool-only checkout still works.
        let starter_path = cards_json
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("starter_decks.json");
        match std::fs::read_to_string(&starter_path) {
            Ok(text) => {
                let file: StarterDeckFile = serde_json::from_str(&text)
                    .map_err(|e| format!("parsing {}: {e}", starter_path.display()))?;
                for d in file.starter_decks {
                    let entry = DeckEntry {
                        main: d.main_deck.clone(),
                        eggs: d.egg_deck.clone(),
                    };
                    // Registered under every name a scenario might reasonably use.
                    for alias in [&d.id, &d.name, &d.set] {
                        if !alias.is_empty() {
                            by_name.insert(alias.to_lowercase(), entry.clone());
                        }
                    }
                }
                sources.push(starter_path.display().to_string());
            }
            Err(e) if decks.is_none() => {
                return Err(format!(
                    "no --decks given and the default deck book {} is unreadable: {e}",
                    starter_path.display()
                ));
            }
            Err(_) => {}
        }

        // Overlay: the explicit pool, which wins on a name collision.
        if let Some(path) = decks {
            let pool = pool::load_pool(path)?;
            for d in pool.decks {
                by_name.insert(
                    d.name.to_lowercase(),
                    DeckEntry {
                        main: d.cards.clone(),
                        eggs: d.eggs.clone(),
                    },
                );
            }
            sources.push(path.display().to_string());
        }

        Ok(DeckBook {
            by_name,
            source: sources.join(" + "),
        })
    }

    pub fn resolve(&self, rest: &str) -> Result<&DeckEntry, String> {
        self.by_name.get(&rest.to_lowercase()).ok_or_else(|| {
            format!(
                "unknown deck `{rest}` (from the scenario's `rest:`); {} knows: {}",
                self.source,
                self.by_name.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        })
    }
}

/// Build one seat's ordered deck: the named list, with the scenario's `stack`
/// moved to the top.
///
/// `Player::draw` **pops the end** of the list, so the top of the deck is the
/// LAST element. The stack is therefore appended in reverse, which makes
/// `stack[0]` the first card drawn — the order an author reads it in.
pub fn ordered_deck(seat: &ScenarioSeat, book: &DeckBook) -> Result<Vec<String>, String> {
    let entry = book.resolve(&seat.rest)?;
    let mut remainder = entry.main.clone();
    remainder.extend(entry.eggs.iter().cloned());
    for id in &seat.stack {
        match remainder.iter().position(|c| c == id) {
            Some(i) => {
                remainder.remove(i);
            }
            None => {
                return Err(format!(
                    "stacked card {id} is not in deck `{}` -- stacking it would \
                     silently change the deck list the scenario claims to use",
                    seat.rest
                ))
            }
        }
    }
    remainder.extend(seat.stack.iter().rev().cloned());
    Ok(remainder)
}

#[cfg(test)]
mod book_for_tests {
    use super::*;

    fn exams() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../qa/dcgo-exams")
    }

    fn name(p: Option<std::path::PathBuf>) -> Option<String> {
        p.and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
    }

    #[test]
    fn the_book_beside_the_scenario_wins() {
        let e = exams();
        let got = book_for(&e.join("BT26/BT26-103-effect1.yaml"), &["ts-jupitermon", "tm-quiet-red"], &e);
        assert_eq!(name(got), Some("ts_jupitermon_pool.json".to_string()));
    }

    #[test]
    fn a_book_elsewhere_in_the_exam_tree_is_found_when_none_sits_beside_it() {
        // Glowing Dawn scenarios for BT26 cards live in BT26/ but their decks
        // are in ST23/glowing_dawn_pool.json.
        let e = exams();
        let got = book_for(&e.join("BT26/BT26-025-effect0.yaml"), &["gd-bt26-4th", "tm-quiet-red"], &e);
        assert_eq!(name(got), Some("glowing_dawn_pool.json".to_string()));
    }

    #[test]
    fn no_book_naming_the_decks_is_none() {
        let e = exams();
        assert_eq!(book_for(&e.join("ST23/x.yaml"), &["no-such-deck", "no-such-deck"], &e), None);
    }
}
