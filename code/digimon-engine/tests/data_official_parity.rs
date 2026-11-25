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
//! The inverse direction is asserted too, since 2026-11-22
//! (`cards_json_circles_are_printed`): an extra circle is NOT harmless. It is a
//! live digivolution the card does not print, and beside a printed circle of
//! the same colour and level it opens a "which cost do you pay?" choice
//! (Garurumon (BT21-067), Mastemon (EX6-029)). The 19 cards once parked in
//! `qa/qa-reports/evo_costs_extra_circles_review.json` were judged from the
//! card face then; the cards whose mirror rows are themselves wrong are in
//! `FACE_READ_CIRCLES` (`support/printed_circles.rs`), which outranks the
//! mirror for them in both directions.

use std::collections::HashSet;
use std::path::PathBuf;

#[path = "support/printed_circles.rs"]
mod printed_circles;
use printed_circles::{
    face_read_rows, FACE_READ_CIRCLES, RAINBOW_CIRCLES, SPLIT_CIRCLES_OUTSIDE_THE_MIRROR,
};

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
        // The card face, read off the image, outranks the mirror for these
        // (`face_read_circles_are_exact_in_every_data_file` pins them).
        if face_read_rows(id).is_some() {
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

/// Split circles of cards the official mirror does not carry (EX12 and two
/// promos): the API-only `cards.json` had kept one colour of each, so a base of
/// the other printed colour could not be digivolved onto. Every data file the
/// engine or the next ingest reads must carry exactly the printed colours
/// (`SPLIT_CIRCLES_OUTSIDE_THE_MIRROR`, read off the card faces).
#[test]
fn split_circles_outside_the_mirror_carry_every_printed_colour() {
    let cards_path = repo_data("cards.json");
    let overrides_path = repo_data("card_overrides.json");
    if !cards_path.exists() || !overrides_path.exists() {
        eprintln!("Skipping: data/*.json missing — full repo checkout required");
        return;
    }
    let cards = read_json(&cards_path);
    let overrides = read_json(&overrides_path);
    let mut failures: Vec<String> = Vec::new();
    for (id, level, cost, colours) in SPLIT_CIRCLES_OUTSIDE_THE_MIRROR {
        let (level, cost) = (i64::from(*level), i64::from(*cost));
        let want: HashSet<(i64, i64, i64)> =
            colours.iter().map(|c| (*c as i64, level, cost)).collect();
        let per_card_json = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("cards")
            .join(id.split('-').next().unwrap().to_lowercase())
            .join(format!("{id}.json"));
        let per_card = per_card_json.exists().then(|| read_json(&per_card_json));
        let mut sources: Vec<(&str, Option<HashSet<(i64, i64, i64)>>)> = vec![
            ("data/cards.json", circle_rows(cards[*id].get("evo_costs"))),
            (
                "data/card_overrides.json (re-applied by every ingest)",
                circle_rows(overrides[*id].get("evo_costs")),
            ),
        ];
        if let Some(doc) = &per_card {
            sources.push((
                "the per-card JSON under code/digimon-engine/cards/",
                circle_rows(doc.get("evo_costs")),
            ));
        }
        for (source, got) in sources {
            // Only this circle's (level, cost) is compared: a card may print
            // other circles too.
            let got: Option<HashSet<(i64, i64, i64)>> = got.map(|rows| {
                rows.into_iter()
                    .filter(|(_, l, c)| *l == level && *c == cost)
                    .collect()
            });
            if got.as_ref() != Some(&want) {
                let mut found: Vec<_> = got.unwrap_or_default().into_iter().collect();
                found.sort_unstable();
                failures.push(format!(
                    "{id}: the printed Lv.{level} / cost {cost} circle has colours {colours:?}                      but {source} carries {found:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} split-circle data fault(s):
{}",
        failures.len(),
        failures.join("
")
    );
}

/// The six three-colour EX12 cards. The API truncated each to two colours, so
/// production saw `card_colors` with one colour missing: a Digimon that counts
/// as fewer colours (Omnimon: Merciful Mode (EX13-077) adds DP per colour among
/// your Digimon and Tamers) and two Options that can be played without their
/// third colour (CR 4.3 §4-22-3). The card face shows three colour bars and
/// DCGO's `cardColors` lists three (read 2026-11-21).
const THREE_COLOUR_CARDS: &[(&str, [i64; 3])] = &[
    ("EX12-037", [1, 2, 0]), // Omnimon: Blue / Yellow / Red
    ("EX12-060", [5, 6, 0]), // Chaosdramon: Black / Purple / Red
    ("EX12-070", [2, 3, 1]), // Sanmyojin Arrival (Option): Yellow / Green / Blue
    ("EX12-071", [5, 1, 0]), // Saneiketsu Invitation (Option): Black / Blue / Red
    ("EX12-076", [2, 4, 0]), // Susanoomon: Yellow / White / Red
    ("EX12-077", [4, 0, 6]), // Proximamon: White / Red / Purple
];

#[test]
fn three_colour_ex12_cards_keep_all_three_colours() {
    let cards_path = repo_data("cards.json");
    let overrides_path = repo_data("card_overrides.json");
    if !cards_path.exists() || !overrides_path.exists() {
        eprintln!("Skipping: data/*.json missing — full repo checkout required");
        return;
    }
    let cards = read_json(&cards_path);
    let overrides = read_json(&overrides_path);
    let sorted = |v: Option<&serde_json::Value>| -> Option<Vec<i64>> {
        let mut colours: Vec<i64> = v?.as_array()?.iter().filter_map(|c| c.as_i64()).collect();
        colours.sort_unstable();
        Some(colours)
    };
    let mut failures: Vec<String> = Vec::new();
    for (id, colours) in THREE_COLOUR_CARDS {
        let mut want = colours.to_vec();
        want.sort_unstable();
        let per_card_json = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("cards")
            .join("ex12")
            .join(format!("{id}.json"));
        let per_card = read_json(&per_card_json);
        for (source, got) in [
            ("data/cards.json", sorted(cards[*id].get("card_colors"))),
            (
                "data/card_overrides.json (re-applied by every ingest)",
                sorted(overrides[*id].get("card_colors")),
            ),
            (
                "the per-card JSON under code/digimon-engine/cards/",
                sorted(per_card.get("card_colors")),
            ),
        ] {
            if got.as_ref() != Some(&want) {
                failures.push(format!(
                    "{id}: card colour codes must be {want:?} (three-colour card) in {source};                      found {got:?}"
                ));
            }
        }
    }
    // The YAML specs feed DebugRunner fixtures, which never read cards.json.
    let spec_failures = std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            use digimon_dsl::compiled::CompiledColor;
            let registry = digimon_engine::dsl_registry::from_embedded_cached()
                .expect("embedded cards.pack loads");
            let mut out: Vec<String> = Vec::new();
            for (id, colours) in THREE_COLOUR_CARDS {
                let spec = registry
                    .lookup(id)
                    .unwrap_or_else(|| panic!("{id} in the embedded DSL pack"));
                let mut got: Vec<i64> = spec
                    .color
                    .iter()
                    .map(|c| match c {
                        CompiledColor::Red => 0,
                        CompiledColor::Blue => 1,
                        CompiledColor::Yellow => 2,
                        CompiledColor::Green => 3,
                        CompiledColor::White => 4,
                        CompiledColor::Black => 5,
                        CompiledColor::Purple => 6,
                    })
                    .collect();
                got.sort_unstable();
                let mut want = colours.to_vec();
                want.sort_unstable();
                if got != want {
                    out.push(format!(
                        "{id}: the YAML spec's `color:` has codes {got:?}, the card prints {want:?}"
                    ));
                }
            }
            out
        })
        .expect("spawn large-stack thread")
        .join()
        .expect("spec colour check panicked");
    failures.extend(spec_failures);
    assert!(
        failures.is_empty(),
        "{} three-colour card data fault(s):
{}",
        failures.len(),
        failures.join("
")
    );
}

/// The per-card JSON beside a card's YAML, if the card has one.
fn per_card_json(id: &str) -> Option<serde_json::Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("cards")
        .join(id.split('-').next().unwrap().to_lowercase())
        .join(format!("{id}.json"));
    path.exists().then(|| read_json(&path))
}

/// The cards whose circles were read off the card face on 2026-11-22
/// (`FACE_READ_CIRCLES`) carry exactly the printed circles in every data file:
/// `data/cards.json` (what real games read), `data/card_overrides.json` (what
/// the next ingest re-applies) and the per-card JSON. A row the face does not
/// print is a live digivolution the card does not have (CR 4.3 2-3-5-1); a
/// missing one refuses a digivolution it has.
#[test]
fn face_read_circles_are_exact_in_every_data_file() {
    let cards_path = repo_data("cards.json");
    let overrides_path = repo_data("card_overrides.json");
    if !cards_path.exists() || !overrides_path.exists() {
        eprintln!("Skipping: data/*.json missing — full repo checkout required");
        return;
    }
    let cards = read_json(&cards_path);
    let overrides = read_json(&overrides_path);
    let mut failures: Vec<String> = Vec::new();
    for (id, _) in FACE_READ_CIRCLES {
        let want: HashSet<(i64, i64, i64)> = face_read_rows(id)
            .unwrap()
            .into_iter()
            .map(|(c, l, k)| (i64::from(c), i64::from(l), k))
            .collect();
        let mut sources: Vec<(&str, Option<HashSet<(i64, i64, i64)>>)> =
            vec![("data/cards.json", circle_rows(cards[*id].get("evo_costs")))];
        if let Some(rows) = overrides.get(*id).and_then(|o| o.get("evo_costs")) {
            sources.push((
                "data/card_overrides.json (re-applied by every ingest)",
                circle_rows(Some(rows)),
            ));
        }
        if let Some(doc) = per_card_json(id) {
            sources.push((
                "the per-card JSON under code/digimon-engine/cards/",
                circle_rows(doc.get("evo_costs")),
            ));
        }
        for (source, got) in sources {
            if got.as_ref() != Some(&want) {
                let mut found: Vec<_> = got.unwrap_or_default().into_iter().collect();
                found.sort_unstable();
                let mut printed: Vec<_> = want.iter().copied().collect();
                printed.sort_unstable();
                failures.push(format!(
                    "{id}: the card prints {printed:?} (colour, level, cost); {source} carries {found:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} data file(s) disagree with the card face:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every circle `data/cards.json` holds is one the card prints: it is in the
/// official mirror, or the card's circles were read off its face
/// (`FACE_READ_CIRCLES`). Cards the mirror does not carry (or carries only as a
/// failed-lookup skeleton with no `colors`) are out of scope; their rows come
/// from the ingest alone and other guards pin the known ones.
#[test]
fn cards_json_circles_are_printed() {
    let cards_path = repo_data("cards.json");
    let official_path = repo_data("card_official.json");
    if !cards_path.exists() || !official_path.exists() {
        eprintln!("Skipping: data/*.json missing — full repo checkout required");
        return;
    }
    let cards = read_json(&cards_path);
    let official = read_json(&official_path);
    let official = official["cards"].as_object().expect("official cards map");
    let mut failures: Vec<String> = Vec::new();
    for (id, card) in cards.as_object().expect("cards.json is an object") {
        if face_read_rows(id).is_some() {
            continue;
        }
        let Some(page) = official.get(id).filter(|p| p.get("colors").is_some()) else {
            continue;
        };
        let printed = circle_rows(page.get("digivolve_costs")).unwrap_or_default();
        let mut extra: Vec<_> = circle_rows(card.get("evo_costs"))
            .unwrap_or_default()
            .into_iter()
            .filter(|row| !printed.contains(row))
            .collect();
        extra.sort_unstable();
        if !extra.is_empty() {
            failures.push(format!(
                "{id}: cards.json carries {extra:?} (colour, level, cost), which the official \
                 mirror does not list. Read the card image: remove the row from cards.json and \
                 card_overrides.json if the face does not print it, or add the card to \
                 FACE_READ_CIRCLES (tests/support/printed_circles.rs) if the mirror is wrong."
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} card(s) hold a digivolve circle the card does not print:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
