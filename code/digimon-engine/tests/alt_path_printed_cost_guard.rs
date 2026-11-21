//! Pool-wide printed-cost guard for BARE (level/colour-gated) digivolve alt-paths.
//!
//! Bug class (BT21-015 / BT21-017 / BT25-057 / ST23-09 / EX10-020): a YAML
//! `alt_paths: kind: digivolve` entry whose `from:` gate is a bare
//! `{ level_eq }` or `{ level_eq, color_is }` predicate IS a claim that the
//! card prints a standard digivolve-cost circle with that level/colour/cost.
//! When the authored cost is CHEAPER than the printed circle — or the card
//! prints no such circle at all — the path creates a digivolve route the real
//! card does not have, and it leaks into production through the
//! `alt_path_registry` (`collect_dsl_alt_digivolve_routes` in
//! `src/dna_digivolve.rs`), where real games consult it alongside the printed
//! `evo_costs` table.
//!
//! This guard sweeps EVERY digivolve alt-path in the embedded DSL pack and,
//! for each bare gate, compares its cost against the card's printed
//! digivolution table:
//!
//!   1. **Primary baseline — official Bandai DB mirror**
//!      (`data/card_official.json`, `digivolve_costs`). Per the project's
//!      source priority, the official DB is authoritative for printed card
//!      data and outranks `cards.json` (lossy API ingest). When the card is
//!      present in the mirror, its rows are the truth — INCLUDING "no rows"
//!      (= the card prints no standard circles, e.g. EX10-020 Puppetmon).
//!   2. **Fallback — production `cards.json` rows**
//!      (`deck_tools::full_card_data()`), used only for cards the official
//!      mirror does not carry.
//!
//! FAIL conditions (unless allowlisted):
//!   - `no-printed-row`: no baseline row matches the gate's level (and colour,
//!     when the gate has one) — the path invents a circle.
//!   - `colour-free-on-coloured-circle`: the gate has NO `color_is`, but the
//!     card's circle at that level is colour-specific. A colour-free gate is a
//!     real route from a base of ANY colour, so the card digivolves onto
//!     off-colour Digimon. This happened in logged games: Dimetromon (P-189)
//!     and Cyclonemon (BT24-011), both printed "Red Lv.3: 2", digivolved onto
//!     the white Ukkomon (BT16-082); 35 specs had the shape (2026-11-21).
//!     Only a RAINBOW circle — baseline rows for all seven colours at that
//!     level — may be colour-free (see `RAINBOW_CIRCLES` in
//!     `support/printed_circles.rs`).
//!   - `cost-not-printed`: the path's cost equals no matching baseline row. A
//!     CHEAPER path undercuts the printed circle (the BT21-015 / BT21-017
//!     free-digivolve class). A COSTLIER one is wrong too: production keeps it
//!     as a second route beside the `cards.json` row, so the player is asked
//!     "which digivolution cost do you pay?" with an option the card does not
//!     print, and DebugRunner fixtures get the wrong evo cost
//!     (MedievalGallantmon (EX8-074) 4 for a printed 3; Pyramidimon (EX10-033)
//!     and (EX11-044) 4 for 3; Magneticdramon (EX10-036) 6 for 4).
//!   - `rainbow-not-colour-free`: the baseline prints a rainbow circle but the
//!     spec has no colour-free bare path at that level and cost. DebugRunner
//!     fixtures never see `cards.json`, so without it the card's own tests run
//!     against a card that cannot use its circle (Susanoomon (EX12-076) carried
//!     `self_color_count_gte: 2` there: "Multicolor" misread as "2+ colours").
//!
//! Authoring the printed circle as an alt-path at its printed cost and colour
//! is the established convention (it also backfills DebugRunner fixtures'
//! `evo_costs` — see `card_data_from_compiled`), and production deduplicates
//! the equal-cost route against the `cards.json` row. A two-colour circle is
//! two entries, one per colour.
//!
//! Gates with ANY other constraint (traits, names, `all_of`, formulas,
//! materials, conditions, `direction: into`, ...) are special digivolution
//! conditions, not circles — they are out of scope here and covered by
//! `alt_path_reachability.rs` and the per-card behavioral suites.

use std::collections::HashMap;

use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledCardKind, CompiledColor, CompiledCost,
    CompiledPredicate,
};

/// Per-card allowlist for bare paths that are FAITHFUL despite not matching a
/// baseline row. Every entry needs a justification verified against the
/// official bundle (`data/card_bundles/<ID>.md`) AND the card image.
/// Shape: (card_id, level, colour gate (None = level-only), cost, justification).
const ALLOWLIST: &[(&str, u8, Option<CompiledColor>, i32, &str)] = &[
    // Empty since 2026-11-21. Ukkomon (BT16-082) and Ukkomon (P-123) were
    // listed here because the official mirror dropped their rainbow circle;
    // the mirror builder now keeps it (seven rows), so their colour-free gate
    // is checked against the baseline like every other path.
];

/// Map a compiled colour to the raw `evo_costs[*].card_color` integer code
/// (`cards.json` convention; mirror of `debug_runner::compiled_color_to_evo_code`).
fn evo_code(color: CompiledColor) -> u8 {
    match color {
        CompiledColor::Red => 0,
        CompiledColor::Blue => 1,
        CompiledColor::Yellow => 2,
        CompiledColor::Green => 3,
        CompiledColor::White => 4,
        CompiledColor::Black => 5,
        CompiledColor::Purple => 6,
    }
}

fn color_name(color: Option<CompiledColor>) -> String {
    match color {
        None => "any-colour".to_string(),
        Some(c) => format!("{c:?}").to_lowercase(),
    }
}

/// Accept ONLY a flat predicate whose sole constraints are `level_eq` and
/// optionally `color_is` (plus an optional `kind: Digimon`). Implemented as
/// equality against a `Default` predicate with just those fields set, so any
/// other constraint (trait/name/dp/`all_of`/... — i.e. a special digivolution
/// condition rather than a printed circle) cleanly disqualifies the gate, and
/// future predicate fields are covered automatically.
fn bare_circle_gate(p: &CompiledPredicate) -> Option<(u8, Option<CompiledColor>)> {
    let level = p.level_eq?;
    if !matches!(p.kind, None | Some(CompiledCardKind::Digimon)) {
        return None;
    }
    let mut expected = CompiledPredicate::default();
    expected.level_eq = Some(level);
    expected.color_is = p.color_is;
    expected.kind = p.kind;
    if *p == expected {
        Some((level, p.color_is))
    } else {
        None
    }
}

/// One printed digivolution row: (colour code, level, memory cost).
type Row = (u8, u8, i64);

/// Parse `data/card_official.json` into card_id -> printed rows. Presence in
/// the map means the official mirror carries the card (possibly with ZERO
/// rows — an authoritative "no standard circles").
fn official_rows() -> HashMap<String, Vec<Row>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/card_official.json");
    let raw = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read data/card_official.json ({path}): {e}"));
    let value: serde_json::Value =
        serde_json::from_str(&raw).expect("card_official.json is valid JSON");
    let cards = value
        .get("cards")
        .and_then(|c| c.as_object())
        .expect("card_official.json has a top-level `cards` object");
    let mut out = HashMap::new();
    for (card_id, card) in cards {
        let rows = card
            .get("digivolve_costs")
            .and_then(|r| r.as_array())
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| {
                        Some((
                            row.get("card_color")?.as_u64()? as u8,
                            row.get("level")?.as_u64()? as u8,
                            row.get("memory_cost")?.as_i64()?,
                        ))
                    })
                    .collect::<Vec<Row>>()
            })
            .unwrap_or_default();
        out.insert(card_id.clone(), rows);
    }
    out
}

/// Printed rows for `card_id`: the official mirror when it carries the card
/// (authoritative, including "no circles"), else production `cards.json`.
fn baseline_rows(
    official: &HashMap<String, Vec<Row>>,
    production: &HashMap<String, digimon_engine::card_data::CardData>,
    card_id: &str,
) -> Option<(Vec<Row>, &'static str)> {
    if let Some(rows) = official.get(card_id) {
        return Some((rows.clone(), "official Bandai DB mirror"));
    }
    let data = production.get(card_id)?;
    Some((
        data.digivolution_costs()
            .iter()
            .map(|r| (r.card_color, r.level, i64::from(r.memory_cost)))
            .collect(),
        "production cards.json",
    ))
}

/// `(level, cost)` pairs printed for ALL seven colours: rainbow circles.
fn rainbow_circles(rows: &[Row]) -> Vec<(u8, i64)> {
    let mut pairs: Vec<(u8, i64)> = rows.iter().map(|(_, l, c)| (*l, *c)).collect();
    pairs.sort_unstable();
    pairs.dedup();
    pairs
        .into_iter()
        .filter(|(l, c)| (0u8..7).all(|colour| rows.contains(&(colour, *l, *c))))
        .collect()
}

#[test]
fn bare_digivolve_alt_paths_match_the_printed_circles() {
    // Building the embedded registry + production card data parses the full
    // pack + cards.json; run on a large-stack thread so the guard is
    // self-contained (no RUST_MIN_STACK dependency).
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(check_printed_costs)
        .expect("spawn large-stack thread")
        .join()
        .expect("alt-path printed-cost guard thread panicked");
}

fn check_printed_costs() {
    let registry =
        digimon_engine::dsl_registry::from_embedded_cached().expect("embedded cards.pack loads");
    let official = official_rows();
    let production = digimon_engine::deck_tools::full_card_data();

    let mut exercised = 0usize;
    let mut allowlisted = 0usize;
    let mut rainbow_cards = 0usize;
    let mut violations: Vec<String> = Vec::new();

    // Deterministic order for stable diagnostics.
    let mut cards: Vec<(&String, &digimon_dsl::compiled::CompiledCard)> = registry.iter().collect();
    cards.sort_by(|a, b| a.0.cmp(b.0));

    for (card_id, compiled) in cards {
        for ap in &compiled.alt_paths {
            if !matches!(ap.kind, CompiledAltPathKind::Digivolve) {
                continue;
            }
            if ap.direction != CompiledAltPathDirection::From {
                continue;
            }
            // Paths with materials / extra costs / burst riders / markers /
            // conditions are special mechanics, never printed circles.
            if !ap.materials.is_empty()
                || !ap.extra_cost.is_empty()
                || !ap.on_burst_turn_end.is_empty()
                || ap.stacks_unsuspended
                || ap.marker
                || ap.condition.is_some()
            {
                continue;
            }
            let Some(CompiledCost::Literal(cost)) = ap.cost else {
                continue;
            };
            let Some(from) = ap.from.as_deref() else {
                continue;
            };
            let Some((level, color)) = bare_circle_gate(from) else {
                continue;
            };
            exercised += 1;

            if ALLOWLIST.iter().any(|(id, alvl, acolor, acost, _)| {
                *id == card_id.as_str() && *alvl == level && *acolor == color && *acost == cost
            }) {
                allowlisted += 1;
                continue;
            }

            let Some((rows, source)) = baseline_rows(&official, &production, card_id.as_str()) else {
                violations.push(format!(
                    "  {card_id} [{} Lv.{level} -> cost {cost}]: card is in NEITHER \
                     data/card_official.json NOR cards.json — no printed data to \
                     validate the bare digivolve path against",
                    color_name(color),
                ));
                continue;
            };

            let level_rows: Vec<&Row> = rows.iter().filter(|(_, l, _)| *l == level).collect();
            let matching: Vec<&Row> = match color {
                Some(c) => level_rows
                    .iter()
                    .copied()
                    .filter(|(row_color, _, _)| *row_color == evo_code(c))
                    .collect(),
                None => {
                    // A colour-free gate matches a base of ANY colour, so it is
                    // faithful only for a rainbow circle: all seven colours.
                    let missing: Vec<u8> = (0u8..7)
                        .filter(|c| !level_rows.iter().any(|(row_color, _, _)| row_color == c))
                        .collect();
                    if !level_rows.is_empty() && !missing.is_empty() {
                        violations.push(format!(
                            "  {card_id} [any-colour Lv.{level} -> cost {cost}]: the gate has no \
                             `color_is`, so the card digivolves from a Lv.{level} of ANY colour, \
                             but the {source} prints a colour-specific circle (rows: {rows:?}). \
                             Author one `{{ level_eq: {level}, color_is: <colour> }}` entry per \
                             printed colour. Only a rainbow circle (all seven colours) may be \
                             colour-free.",
                        ));
                        continue;
                    }
                    level_rows.clone()
                }
            };

            if matching.is_empty() {
                violations.push(format!(
                    "  {card_id} [{} Lv.{level} -> cost {cost}]: NO printed digivolve circle \
                     matches this gate in the {source} (rows: {rows:?}) — the alt-path INVENTS \
                     a digivolve route the card does not print. Fix the YAML (or allowlist with \
                     a bundle-verified justification).",
                    color_name(color),
                ));
                continue;
            }
            let printed_min = matching.iter().map(|(_, _, c)| *c).min().unwrap();
            if i64::from(cost) < printed_min {
                violations.push(format!(
                    "  {card_id} [{} Lv.{level} -> cost {cost}]: CHEAPER than the printed \
                     circle (printed cost {printed_min} per the {source}) — the \
                     BT21-015/BT21-017 free-digivolve bug class. Trait/name-gated special \
                     conditions must be authored as gated predicates, never as bare \
                     level/colour paths.",
                    color_name(color),
                ));
            } else if !matching.iter().any(|(_, _, c)| *c == i64::from(cost)) {
                let mut printed: Vec<i64> = matching.iter().map(|(_, _, c)| *c).collect();
                printed.sort_unstable();
                printed.dedup();
                violations.push(format!(
                    "  {card_id} [{} Lv.{level} -> cost {cost}]: the cost matches NO printed \
                     circle (printed cost(s) {printed:?} per the {source}). A costlier bare \
                     path is a second route beside the printed one: production offers a \
                     \"which digivolution cost do you pay?\" choice the card does not print, \
                     and DebugRunner fixtures get the wrong evo cost. Author the printed cost; \
                     a conditional or name/trait-gated cost is a gated path, not a bare one.",
                    color_name(color),
                ));
            }
        }

        // Converse: a printed rainbow circle must be authored as a colour-free
        // bare path at its level and cost.
        if let Some((rows, source)) = baseline_rows(&official, &production, card_id.as_str()) {
            for (level, cost) in rainbow_circles(&rows) {
                rainbow_cards += 1;
                let authored = compiled.alt_paths.iter().any(|ap| {
                    matches!(ap.kind, CompiledAltPathKind::Digivolve)
                        && ap.direction == CompiledAltPathDirection::From
                        && ap.materials.is_empty()
                        && ap.extra_cost.is_empty()
                        && ap.condition.is_none()
                        && ap.cost == Some(CompiledCost::Literal(cost as i32))
                        && ap.from.as_deref().and_then(bare_circle_gate) == Some((level, None))
                });
                if !authored {
                    violations.push(format!(
                        "  {card_id}: the {source} prints a RAINBOW (any-colour) Lv.{level} / \
                         cost {cost} circle, but the spec has no colour-free \
                         `from: {{ level_eq: {level} }}` digivolve path at cost {cost}. \
                         DebugRunner fixtures never see cards.json, so the card's own tests \
                         cannot use the circle. \"Multicolor\" is ANY colour, not a 2+-colour \
                         base.",
                    ));
                }
            }
        }
    }

    eprintln!(
        "[alt-path-printed-cost] exercised {exercised} bare digivolve paths \
         ({allowlisted} allowlisted); {rainbow_cards} rainbow circle(s) in the pack"
    );

    assert!(
        violations.is_empty(),
        "{} bare digivolve alt-path(s) disagree with the printed digivolution circles:\n{}",
        violations.len(),
        violations.join("\n"),
    );

    // Tripwire: the pool carries hundreds of bare (printed-circle) alt paths;
    // if the extractor or the pack regresses to exercising almost nothing,
    // fail loudly rather than passing vacuously.
    assert!(
        exercised >= 250,
        "expected to exercise >=250 bare digivolve alt-paths, only ran {exercised} — the \
         extractor or the embedded pack likely regressed"
    );
    // Seven specs print a rainbow circle today (the two Ukkomon, the two
    // Kimeramon, Susanoomon (BT18-102) and (EX12-076), Omnimon: Merciful Mode
    // (EX13-077)). Fewer would mean the baseline lost the rainbow rows again.
    assert!(
        rainbow_cards >= 7,
        "expected >=7 rainbow-circle specs in the pack, found {rainbow_cards} — the official \
         mirror or cards.json likely dropped the seven-colour rows again"
    );
}
