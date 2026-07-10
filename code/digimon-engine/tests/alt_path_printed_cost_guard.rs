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
//!   - `cheaper-than-printed`: the path's cost is lower than every matching
//!     baseline row — the path undercuts the printed circle.
//!
//! Equal-or-costlier paths pass: authoring the printed circle as an alt-path
//! at its printed cost is the established convention (it also backfills
//! DebugRunner fixtures' `evo_costs` — see `card_data_from_compiled`), and
//! production deduplicates the equal-cost route against the `cards.json` row.
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
    (
        "BT16-082",
        2,
        None,
        1,
        "Ukkomon prints a RAINBOW (any-colour) 'Lv.2: cost 1' digivolve circle — verified on \
         the card image (DCGO mirror BT16-082.webp). The official-DB mirror cannot represent \
         a rainbow circle and carries no rows; cards.json approximates it as red-only. The \
         colour-free level-only gate at the printed cost IS the faithful authoring.",
    ),
    (
        "P-123",
        2,
        None,
        0,
        "Ukkomon (promo) prints a RAINBOW (any-colour) 'Lv.2: cost 0' digivolve circle — \
         verified on the card image (P-123.webp). Same rainbow-circle representation gap in \
         the official mirror as BT16-082.",
    ),
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

#[test]
fn bare_digivolve_alt_paths_never_undercut_or_invent_printed_circles() {
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

            // Baseline rows: official mirror when it carries the card
            // (authoritative, including "no circles"), else production.
            let (rows, source): (Vec<Row>, &str) = match official.get(card_id.as_str()) {
                Some(rows) => (rows.clone(), "official Bandai DB mirror"),
                None => match production.get(card_id.as_str()) {
                    Some(data) => (
                        data.digivolution_costs()
                            .iter()
                            .map(|r| (r.card_color, r.level, i64::from(r.memory_cost)))
                            .collect(),
                        "production cards.json",
                    ),
                    None => {
                        violations.push(format!(
                            "  {card_id} [{} Lv.{level} -> cost {cost}]: card is in NEITHER \
                             data/card_official.json NOR cards.json — no printed data to \
                             validate the bare digivolve path against",
                            color_name(color),
                        ));
                        continue;
                    }
                },
            };

            let matching: Vec<&Row> = rows
                .iter()
                .filter(|(row_color, row_level, _)| {
                    *row_level == level && color.map_or(true, |c| *row_color == evo_code(c))
                })
                .collect();

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
            }
        }
    }

    eprintln!(
        "[alt-path-printed-cost] exercised {exercised} bare digivolve paths \
         ({allowlisted} allowlisted)"
    );

    assert!(
        violations.is_empty(),
        "{} bare digivolve alt-path(s) undercut or invent printed digivolution circles:\n{}",
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
}
