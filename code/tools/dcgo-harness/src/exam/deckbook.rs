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
use std::path::Path;

use serde::Deserialize;

use crate::exam::scenario::ScenarioSeat;
use crate::pool;

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
