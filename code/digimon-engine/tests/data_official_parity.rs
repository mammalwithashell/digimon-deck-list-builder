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

#[path = "support/printed_circles.rs"]
mod printed_circles;
use printed_circles::RAINBOW_CIRCLES;

/// Official-mirror circles the card face does NOT print (the mirror keeps
/// every printing's rows / has DB errors; CLAUDE.md "Printed card data").
/// `(card_id, color, level, cost)`. Each entry must still be in the mirror,
/// so a corrected mirror retires it.
const MIRROR_CIRCLE_ERRORS: &[(&str, i64, i64, i64)] = &[
    // Sunflowmon BT10-048: the card image prints one Green Lv.3 / cost 2
    // circle; the mirror adds a Green Lv.3 / cost 0 row.
    ("BT10-048", 3, 3, 0),
];

const ALL_COLOURS: std::ops::RangeInclusive<i64> = 0..=6;

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
    for (id, color, level, cost) in MIRROR_CIRCLE_ERRORS {
        let still_in_mirror = official_cards
            .get(*id)
            .and_then(|e| e.get("digivolve_costs"))
            .and_then(|v| v.as_array())
            .is_some_and(|cs| {
                cs.iter().any(|c| {
                    (c["card_color"].as_i64(), c["level"].as_i64(), c["memory_cost"].as_i64())
                        == (Some(*color), Some(*level), Some(*cost))
                })
            });
        if !still_in_mirror {
            failures.push(format!(
                "{id}: MIRROR_CIRCLE_ERRORS entry is no longer in the mirror; delete it"
            ));
        }
    }
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
            if MIRROR_CIRCLE_ERRORS
                .iter()
                .any(|e| e.0 == id.as_str() && (e.1, e.2, e.3) == t)
            {
                continue;
            }
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

/// Printed INHERITED text vs the official Bandai DB, for the ids that have been
/// adjudicated against the card face.
///
/// `card_data.rs` parses printed keywords out of
/// `inherited_effect_description_eng`, so a wrong string there silently grants a
/// keyword to every Digimon that inherits from the card. BT25-026 Crescemon was
/// ingested with a DIFFERENT card's inherited text — `<Security A. +1>` instead of
/// the printed "[Your Turn] This Digimon's attack target can't change." — which gave
/// the whole Crescemon line a free extra security check. The DCGO exam measured it:
/// `qa/dcgo-exams/BT25/BT25-026-inherited0.yaml` diffed `p1.security ours=3 dcgo=4`
/// with an extra card in `p1.trash`. Fixed via `data/card_overrides.json` +
/// `data/cards.json` on 2026-09-21.
///
/// Scope is a REVIEWED allowlist, not the whole pool: a normalized sweep finds 16
/// gross inherited-text mismatches against `card_official.json`, and the other 15 are
/// a separate, unreviewed family (`BT19-011`, `BT19-037`, `BT19-050`, `EX9-013`,
/// `EX9-020`, `LM-025`, `LM-026`, `LM-043`, `P-191` all carry an "Ace Overflow" line
/// in the inherited field; `BT1-060`, `BT3-027`, `BT21-059`, `EX4-032`, `EX4-033`,
/// `EX4-034` are wording drift). Add an id here once its face has been read.
#[test]
fn official_db_inherited_text_matches_cards_json() {
    const REVIEWED: &[&str] = &["BT25-026"];

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

    // Compare on letters/digits only: the two sources differ freely in
    // whitespace, full-width brackets and reminder-text parentheses.
    fn norm(s: &str) -> String {
        s.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect()
    }

    let mut failures: Vec<String> = Vec::new();
    for id in REVIEWED {
        let want = official["cards"][id]["text_sections"]
            .as_array()
            .map(|secs| {
                secs.iter()
                    .filter(|s| s["label"].as_str() == Some("Inherited Effect"))
                    .filter_map(|s| s["text"].as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();
        let got = cards[id]["inherited_effect_description_eng"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if want.is_empty() {
            failures.push(format!(
                "{id}: official mirror has no Inherited Effect section"
            ));
            continue;
        }
        if norm(&want) != norm(&got) {
            failures.push(format!(
                "{id}: cards.json inherited text does not match the official DB\n  official: {want}\n  ours:     {got}"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} reviewed card(s) drifted from the official Bandai DB's inherited text \
         (reconcile via data/card_overrides.json + data/cards.json):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn read_json(path: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(
        &std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display())),
    )
    .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// `(colour, level, cost)` rows of an `evo_costs` / `digivolve_costs` array.
fn circle_rows(value: Option<&serde_json::Value>) -> Option<HashSet<(i64, i64, i64)>> {
    let rows = value?.as_array()?;
    Some(
        rows.iter()
            .map(|e| {
                (
                    e["card_color"].as_i64().unwrap_or(-1),
                    e["level"].as_i64().unwrap_or(-1),
                    e["memory_cost"].as_i64().unwrap_or(-1),
                )
            })
            .collect(),
    )
}

/// `(level, cost)` pairs carried by ALL seven colours — i.e. rainbow circles.
fn seven_colour_circles(rows: &HashSet<(i64, i64, i64)>) -> Vec<(i64, i64)> {
    let mut pairs: Vec<(i64, i64)> = rows.iter().map(|(_, l, c)| (*l, *c)).collect();
    pairs.sort_unstable();
    pairs.dedup();
    pairs
        .into_iter()
        .filter(|(l, c)| ALL_COLOURS.clone().all(|colour| rows.contains(&(colour, *l, *c))))
        .collect()
}

/// Every rainbow circle is seven rows — in `data/cards.json` (what real games
/// read), in `data/card_overrides.json` (what the next ingest re-applies), in
/// the official mirror `data/card_official.json` (what reconciles compare
/// against) and in the per-card JSON beside the YAML — and no card outside
/// `RAINBOW_CIRCLES` (`support/printed_circles.rs`) carries one.
#[test]
fn rainbow_circles_are_one_row_per_colour_in_every_data_file() {
    let cards_path = repo_data("cards.json");
    let overrides_path = repo_data("card_overrides.json");
    let official_path = repo_data("card_official.json");
    if !cards_path.exists() || !overrides_path.exists() || !official_path.exists() {
        eprintln!("Skipping: data/*.json missing — full repo checkout required");
        return;
    }
    let cards = read_json(&cards_path);
    let overrides = read_json(&overrides_path);
    let official = read_json(&official_path);

    let mut failures: Vec<String> = Vec::new();

    for (id, level, cost) in RAINBOW_CIRCLES {
        let (level, cost) = (&i64::from(*level), &i64::from(*cost));
        let want: HashSet<(i64, i64, i64)> =
            ALL_COLOURS.clone().map(|colour| (colour, *level, *cost)).collect();
        let per_card_json = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("cards")
            .join(id.split('-').next().unwrap().to_lowercase())
            .join(format!("{id}.json"));
        let per_card = per_card_json.exists().then(|| read_json(&per_card_json));
        let sources: [(&str, Option<HashSet<(i64, i64, i64)>>); 4] = [
            ("data/cards.json", circle_rows(cards[*id].get("evo_costs"))),
            (
                "data/card_overrides.json (re-applied by every ingest)",
                circle_rows(overrides[*id].get("evo_costs")),
            ),
            (
                "data/card_official.json (the official mirror)",
                circle_rows(official["cards"][*id].get("digivolve_costs")),
            ),
            (
                "the per-card JSON under code/digimon-engine/cards/",
                match &per_card {
                    Some(doc) => circle_rows(doc.get("evo_costs")),
                    // No per-card JSON for this id: nothing to keep in step.
                    None => Some(want.clone()),
                },
            ),
        ];
        for (source, got) in sources {
            match got {
                Some(got) if got == want => {}
                Some(got) => {
                    let mut got: Vec<_> = got.into_iter().collect();
                    got.sort_unstable();
                    failures.push(format!(
                        "{id}: rainbow Lv.{level} / cost {cost} circle must be one row per colour \
                         (0..=6) in {source}; found {got:?}"
                    ));
                }
                None => failures.push(format!(
                    "{id}: rainbow Lv.{level} / cost {cost} circle has no evo-cost rows in {source}"
                )),
            }
        }
    }

    // Closed list: a seven-colour circle anywhere else is either a new rainbow
    // card (read its face, then add it above) or a data error.
    let listed = |id: &str, level: i64, cost: i64| {
        RAINBOW_CIRCLES
            .iter()
            .any(|e| e.0 == id && i64::from(e.1) == level && i64::from(e.2) == cost)
    };
    let card_map = cards.as_object().expect("cards.json is an object keyed by card id");
    for (id, card) in card_map {
        if let Some(rows) = circle_rows(card.get("evo_costs")) {
            for (level, cost) in seven_colour_circles(&rows) {
                if !listed(id, level, cost) {
                    failures.push(format!(
                        "{id}: data/cards.json carries a seven-colour Lv.{level} / cost {cost} \
                         circle but the card is not in RAINBOW_CIRCLES"
                    ));
                }
            }
        }
    }
    for (id, entry) in official["cards"].as_object().expect("official cards map") {
        if let Some(rows) = circle_rows(entry.get("digivolve_costs")) {
            for (level, cost) in seven_colour_circles(&rows) {
                if !listed(id, level, cost) {
                    failures.push(format!(
                        "{id}: the official mirror lists a \"Multicolor {cost} from Lv.{level}\" \
                         (rainbow) circle but the card is not in RAINBOW_CIRCLES"
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} rainbow-circle data fault(s) — a rainbow circle is ANY colour, held as seven \
         evo-cost rows (see RAINBOW_CIRCLES):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
