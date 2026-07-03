//! Data-layer parity guard: printed digivolve circles vs `data/cards.json`.
//!
//! The digimoncard.io API ingest behind `data/cards.json` silently drops
//! digivolve circles — most often the off-colour circle of a dual-colour card
//! (BG Imperial line, 2026-06-15; BT12-043 ShineGreymon, BT11-018 Shoutmon DX,
//! BT12-038 GeoGreymon, 2026-07-02), but a 2026-07-02 pool-wide sweep against
//! the official Bandai DB mirror (`data/card_official.json`, built by
//! `code/tools/build_card_bundles.py` from world.digimoncard.com — the
//! authoritative source for printed card data per CLAUDE.md) found **826**
//! affected cards. All were reconciled into `data/card_overrides.json`
//! (durable) + `data/cards.json` (live) on that date.
//!
//! This guard keeps them reconciled: every digivolve circle the official DB
//! prints for a card MUST be present in that card's `evo_costs` in
//! `data/cards.json`. If a future re-ingest drops circles again, this fails
//! with the exact card list.
//!
//! The inverse direction (cards.json circles absent from the official DB) is
//! NOT asserted — 19 such cards are parked in
//! `qa/qa-reports/evo_costs_extra_circles_review.json` pending manual review
//! (they may be official-parse misses or genuine cards.json errors; extra
//! circles in cards.json are also non-destructive for gameplay until judged).

use std::collections::HashSet;
use std::path::PathBuf;

fn repo_data(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
        .join(file)
}

#[test]
fn official_db_digivolve_circles_present_in_cards_json() {
    let official_path = repo_data("card_official.json");
    let cards_path = repo_data("cards.json");
    if !official_path.exists() || !cards_path.exists() {
        eprintln!("Skipping: data/card_official.json or data/cards.json missing — full repo checkout required");
        return;
    }

    let official: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&official_path).expect("read official"))
            .expect("parse card_official.json");
    let cards: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&cards_path).expect("read cards"))
            .expect("parse cards.json");

    // Index cards.json evo_costs by card id.
    let mut ours: std::collections::HashMap<String, HashSet<(i64, i64, i64)>> =
        std::collections::HashMap::new();
    let seq: Box<dyn Iterator<Item = &serde_json::Value>> = match &cards {
        serde_json::Value::Array(a) => Box::new(a.iter()),
        serde_json::Value::Object(o) => Box::new(o.values()),
        _ => panic!("unexpected cards.json shape"),
    };
    for c in seq {
        let id = c
            .get("cardnumber")
            .or_else(|| c.get("id"))
            .or_else(|| c.get("card_id"))
            .and_then(|v| v.as_str());
        let Some(id) = id else { continue };
        let set = ours.entry(id.to_string()).or_default();
        if let Some(evo) = c.get("evo_costs").and_then(|v| v.as_array()) {
            for e in evo {
                set.insert((
                    e["card_color"].as_i64().unwrap_or(-1),
                    e["level"].as_i64().unwrap_or(-1),
                    e["memory_cost"].as_i64().unwrap_or(-1),
                ));
            }
        }
    }

    let mut failures: Vec<String> = Vec::new();
    let official_cards = official["cards"].as_object().expect("official cards map");
    for (id, entry) in official_cards {
        let Some(circles) = entry.get("digivolve_costs").and_then(|v| v.as_array()) else {
            continue;
        };
        if circles.is_empty() {
            continue;
        }
        let Some(have) = ours.get(id) else {
            // Card exists in the official mirror but not in cards.json at all
            // (e.g. newest promos not yet ingested) — out of scope here.
            continue;
        };
        for c in circles {
            let t = (
                c["card_color"].as_i64().unwrap_or(-1),
                c["level"].as_i64().unwrap_or(-1),
                c["memory_cost"].as_i64().unwrap_or(-1),
            );
            if !have.contains(&t) {
                failures.push(format!(
                    "{id}: missing printed circle (color={}, Lv.{}, cost {})",
                    t.0, t.1, t.2
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} printed digivolve circles from the official Bandai DB are missing in \
         data/cards.json (API-ingest drop — reconcile via data/card_overrides.json, \
         see the 2026-07-02 pool-wide sweep):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
