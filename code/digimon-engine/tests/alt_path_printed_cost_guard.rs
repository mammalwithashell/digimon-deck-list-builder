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
//!   - `split-circle-incomplete`: the baseline prints a circle accepting two to
//!     six colours at one (level, cost), and the spec authors a bare gate for
//!     at least one of those colours at that level and cost but not for every
//!     one. Production reads `cards.json`, so live play is unaffected, but
//!     DebugRunner fixtures back-fill `evo_costs` from the alt-paths and lose
//!     the missing colour's digivolution (99 specs had the shape, 2026-11-22).
//!     Specs that author NO gate for the group, and gates with extra
//!     conditions (trait/name), are out of scope. The rule is the pure function
//!     `split_circles`, with its own tests at the bottom of this file.
//!   - `stale-phantom-row`: an entry of `BASELINE_PHANTOM_ROWS` names a row the
//!     baseline no longer carries (the data was corrected): delete the entry.
//!
//! The baseline is not the card. Where it lists a row the card face does not
//! print, the row goes in `BASELINE_PHANTOM_ROWS` and is dropped before every
//! rule above, so authoring it fails as `no-printed-row` and the remaining
//! colours of its circle are still checked.
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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use digimon_dsl::compiled::{
    CompiledAltPathDirection, CompiledAltPathKind, CompiledCardKind, CompiledColor, CompiledCost,
    CompiledPredicate,
};

const COLOUR_NAMES: [&str; 7] = ["red", "blue", "yellow", "green", "white", "black", "purple"];

/// Baseline rows the card face does NOT print: `(card id, colour, level, cost,
/// what the card prints instead)`.
///
/// The official mirror (and `cards.json`, which was reconciled from it) lists
/// each of these rows; the card image and the DCGO card asset's `EvoCosts`
/// agree with each other that the card has no such circle. Every entry was read
/// off the card image on 2026-11-22 (every printing in the local image mirror).
///
/// A listed row is removed from the baseline before any rule runs, so:
/// - a spec that authors it fails `no-printed-row` (it would give every test of
///   the card an illegal digivolution);
/// - the other colours of the same circle are still held to the split-circle
///   rule (an entry never switches a whole circle off).
///
/// These are DATA errors that are still live: production reads `cards.json`,
/// so real games offer the phantom digivolution until the row is removed there
/// (and from `card_overrides.json`, with the card added to
/// `PRINTED_OVER_MIRROR` in `code/tests/test_cards_json_integrity.py`). When
/// that happens for a card whose baseline here is `cards.json`, or the mirror
/// is corrected, the entry fails as `stale-phantom-row` and must be deleted.
/// Do not add an entry without reading the card image.
const BASELINE_PHANTOM_ROWS: &[(&str, &str, u8, i64, &str)] = &[
    // Imperialdramon: Dragon Mode (BT12-030): the cost 3 on the card is the
    // name-gated "Digivolve: 3 from [Paildramon] or [Dinobeemon]" line, not a
    // Lv.4 circle. With these rows production digivolves it from ANY blue or
    // green Lv.4 for 3.
    ("BT12-030", "blue", 4, 3, "two circles: blue Lv.5 / 4 and green Lv.5 / 4"),
    ("BT12-030", "green", 4, 3, "two circles: blue Lv.5 / 4 and green Lv.5 / 4"),
    // Mastemon (EX6-029): one circle, cost 4. No cost-5 circle on any printing.
    ("EX6-029", "yellow", 5, 5, "one yellow|purple Lv.5 / 4 circle"),
    ("EX6-029", "purple", 5, 5, "one yellow|purple Lv.5 / 4 circle"),
    // Cendrillmon (EX7-030), a mono-yellow card.
    ("EX7-030", "black", 5, 4, "one yellow Lv.5 / 4 circle"),
    // Skadimon (EX8-028), a blue/yellow card: the circle has two colours on
    // all three printings (compare the three-colour Gallantmon (X Antibody)
    // (EX8-073)).
    ("EX8-028", "purple", 5, 4, "one blue|yellow Lv.5 / 4 circle"),
    // Pyramidimon (EX8-055), a mono-black card.
    ("EX8-055", "red", 5, 4, "one black Lv.5 / 4 circle"),
];

fn colour_code(name: &str) -> u8 {
    COLOUR_NAMES
        .iter()
        .position(|n| *n == name)
        .unwrap_or_else(|| panic!("`{name}` is not a card colour")) as u8
}

/// `rows` without the rows `BASELINE_PHANTOM_ROWS` lists for `card_id`.
fn without_phantom_rows(card_id: &str, rows: Vec<Row>) -> Vec<Row> {
    rows.into_iter()
        .filter(|(colour, level, cost)| {
            !BASELINE_PHANTOM_ROWS.iter().any(|(id, c, l, k, _)| {
                *id == card_id && colour_code(c) == *colour && l == level && k == cost
            })
        })
        .collect()
}

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
/// (authoritative, including "no circles"), else production `cards.json` —
/// minus the rows `BASELINE_PHANTOM_ROWS` records as not printed.
fn baseline_rows(
    official: &HashMap<String, Vec<Row>>,
    production: &HashMap<String, digimon_engine::card_data::CardData>,
    card_id: &str,
) -> Option<(Vec<Row>, &'static str)> {
    let (rows, source) = raw_baseline_rows(official, production, card_id)?;
    Some((without_phantom_rows(card_id, rows), source))
}

/// The baseline exactly as the data files hold it.
fn raw_baseline_rows(
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

/// The bare circle gate of one alt-path, if it is one: `(level, colour, cost)`.
/// Paths with materials / extra costs / burst riders / markers / conditions,
/// a non-literal cost or any other `from:` constraint are special mechanics,
/// never printed circles.
fn bare_circle_path(
    ap: &digimon_dsl::compiled::CompiledAltPath,
) -> Option<(u8, Option<CompiledColor>, i32)> {
    if !matches!(ap.kind, CompiledAltPathKind::Digivolve)
        || ap.direction != CompiledAltPathDirection::From
        || !ap.materials.is_empty()
        || !ap.extra_cost.is_empty()
        || !ap.on_burst_turn_end.is_empty()
        || ap.stacks_unsuspended
        || ap.marker
        || ap.condition.is_some()
    {
        return None;
    }
    let Some(CompiledCost::Literal(cost)) = ap.cost else {
        return None;
    };
    let (level, color) = bare_circle_gate(ap.from.as_deref()?)?;
    Some((level, color, cost))
}

/// A printed split circle the spec authors for some of its colours only.
#[derive(Debug, PartialEq, Eq)]
struct IncompleteSplit {
    level: u8,
    cost: i64,
    printed: BTreeSet<u8>,
    authored: BTreeSet<u8>,
    missing: BTreeSet<u8>,
}

/// What a spec does with each printed split circle of its card, as `(level,
/// cost)` keys.
#[derive(Debug, Default, PartialEq, Eq)]
struct SplitCircles {
    /// Every printed colour is authored.
    complete: Vec<(u8, i64)>,
    /// No printed colour is authored (out of scope: the spec does not author
    /// this circle at all).
    unauthored: Vec<(u8, i64)>,
    /// Some but not all printed colours are authored: the violation.
    incomplete: Vec<IncompleteSplit>,
}

/// The split-circle rule. `printed` is the card's printed rows, `authored` the
/// spec's bare COLOURED circle gates, both as `(colour, level, cost)`.
///
/// A split circle is two to six colours at one `(level, cost)`: one colour is a
/// plain circle and seven is a rainbow one (authored colour-free, checked
/// elsewhere). Each split circle is judged on its own, so a card with two of
/// them (two levels, or two costs at one level) cannot hide one behind the
/// other, and a colour authored at another level or cost does not count.
fn split_circles(printed: &[Row], authored: &[Row]) -> SplitCircles {
    let mut circles: BTreeMap<(u8, i64), BTreeSet<u8>> = BTreeMap::new();
    for (colour, level, cost) in printed {
        circles.entry((*level, *cost)).or_default().insert(*colour);
    }
    let mut out = SplitCircles::default();
    for ((level, cost), printed) in circles {
        if !(2..7).contains(&printed.len()) {
            continue;
        }
        let authored: BTreeSet<u8> = authored
            .iter()
            .filter(|(colour, l, c)| *l == level && *c == cost && printed.contains(colour))
            .map(|(colour, _, _)| *colour)
            .collect();
        if authored.is_empty() {
            out.unauthored.push((level, cost));
        } else if authored == printed {
            out.complete.push((level, cost));
        } else {
            let missing = printed.difference(&authored).copied().collect();
            out.incomplete.push(IncompleteSplit {
                level,
                cost,
                printed,
                authored,
                missing,
            });
        }
    }
    out
}

fn colour_names(set: &BTreeSet<u8>) -> String {
    set.iter()
        .map(|c| COLOUR_NAMES[*c as usize])
        .collect::<Vec<_>>()
        .join("+")
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
    let mut split_complete = 0usize;
    let mut split_incomplete = 0usize;
    let mut split_unauthored: Vec<String> = Vec::new();
    let mut violations: Vec<String> = Vec::new();

    // Deterministic order for stable diagnostics.
    let mut cards: Vec<(&String, &digimon_dsl::compiled::CompiledCard)> = registry.iter().collect();
    cards.sort_by(|a, b| a.0.cmp(b.0));

    for (card_id, compiled) in cards {
        for ap in &compiled.alt_paths {
            let Some((level, color, cost)) = bare_circle_path(ap) else {
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
                // The gate may be one the data files do list but the card
                // does not print: say so, the generic message would point
                // the author at a baseline that seems to agree with the gate.
                let phantom = color.and_then(|c| {
                    BASELINE_PHANTOM_ROWS.iter().find(|(id, pc, pl, pk, _)| {
                        *id == card_id.as_str()
                            && colour_code(pc) == evo_code(c)
                            && *pl == level
                            && *pk == i64::from(cost)
                    })
                });
                if let Some((_, _, _, _, prints)) = phantom {
                    violations.push(format!(
                        "  {card_id} [{} Lv.{level} -> cost {cost}]: the {source} lists this \
                         row, but the card face does not print it (BASELINE_PHANTOM_ROWS: the \
                         card prints {prints}). The alt-path is a digivolve route the card does \
                         not have. Remove it from the YAML.",
                        color_name(color),
                    ));
                    continue;
                }
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

        // Split circles: a printed circle accepting 2..=6 colours at one
        // (level, cost) must be authored for EVERY colour once the spec authors
        // a bare gate for at least one of them (`split_circles`).
        if let Some((rows, source)) = baseline_rows(&official, &production, card_id.as_str()) {
            let authored: Vec<Row> = compiled
                .alt_paths
                .iter()
                .filter_map(bare_circle_path)
                .filter_map(|(level, color, cost)| {
                    Some((evo_code(color?), level, i64::from(cost)))
                })
                .collect();
            let report = split_circles(&rows, &authored);
            split_complete += report.complete.len();
            for _ in &report.unauthored {
                split_unauthored.push(card_id.clone());
            }
            for gap in report.incomplete {
                split_incomplete += 1;
                violations.push(format!(
                    "  {card_id}: split circle Lv.{level} / cost {cost} (printed colours {printed} \
                     per the {source}) is authored for only [{authored}]; missing [{missing}]. \
                     Author one `{{ level_eq: {level}, color_is: <colour> }}` alt_path per \
                     printed colour at cost {cost} -- after reading the card image: if the \
                     card does not print the missing colour, the baseline row is the error \
                     and belongs in BASELINE_PHANTOM_ROWS.",
                    level = gap.level,
                    cost = gap.cost,
                    printed = colour_names(&gap.printed),
                    authored = colour_names(&gap.authored),
                    missing = colour_names(&gap.missing),
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

    // A phantom-row entry whose row the baseline no longer lists is dead
    // weight that would hide a later regression: the data was fixed, so the
    // entry goes.
    for (card_id, colour, level, cost, prints) in BASELINE_PHANTOM_ROWS {
        let row = (colour_code(colour), *level, *cost);
        let listed = raw_baseline_rows(&official, &production, card_id)
            .is_some_and(|(rows, _)| rows.contains(&row));
        if !listed {
            violations.push(format!(
                "  {card_id}: BASELINE_PHANTOM_ROWS lists a {colour} Lv.{level} / cost {cost} \
                 row (the card prints {prints}), but the baseline no longer carries that row \
                 -- stale entry, delete it.",
            ));
        }
    }

    eprintln!(
        "[alt-path-printed-cost] exercised {exercised} bare digivolve paths \
         ({allowlisted} allowlisted); {rainbow_cards} rainbow circle(s) in the pack; \
         {split_complete} split circle(s) authored in every colour, {split_incomplete} \
         incomplete; {} split circle(s) the spec authors no bare gate for (out of scope): {:?}",
        split_unauthored.len(),
        split_unauthored,
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
    // 308 split circles are authored in every colour today (99 of them
    // completed on 2026-11-22). Far fewer means the split-circle rule stopped
    // seeing circles (baseline rows or bare gates no longer parse), not that
    // the specs are clean.
    assert!(
        split_complete >= 280,
        "expected >=280 split circles authored in every colour, found {split_complete} -- the \
         split-circle rule is passing vacuously"
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

// ---------------------------------------------------------------------------
// The split-circle rule and the phantom-row filter, on hand-built rows.
// Colour codes: red 0, blue 1, yellow 2, green 3, white 4, black 5, purple 6.
// ---------------------------------------------------------------------------

fn set(colours: &[u8]) -> BTreeSet<u8> {
    colours.iter().copied().collect()
}

#[test]
fn a_two_colour_circle_with_one_colour_authored_is_incomplete() {
    // Paildramon (ST9-05) before 2026-11-22: blue|green Lv.4 / 4, blue only.
    let printed = [(1, 4, 4), (3, 4, 4)];
    let report = split_circles(&printed, &[(1, 4, 4)]);
    assert_eq!(
        report.incomplete,
        vec![IncompleteSplit {
            level: 4,
            cost: 4,
            printed: set(&[1, 3]),
            authored: set(&[1]),
            missing: set(&[3]),
        }]
    );
    assert!(report.complete.is_empty() && report.unauthored.is_empty());

    let fixed = split_circles(&printed, &[(1, 4, 4), (3, 4, 4)]);
    assert_eq!(fixed.complete, vec![(4, 4)]);
    assert!(fixed.incomplete.is_empty());
}

#[test]
fn a_three_colour_circle_with_two_colours_authored_is_incomplete() {
    // Magnamon (X Antibody) (BT16-102): yellow|blue|black Lv.5 / 4.
    let printed = [(2, 5, 4), (1, 5, 4), (5, 5, 4)];
    let report = split_circles(&printed, &[(2, 5, 4), (1, 5, 4)]);
    assert_eq!(report.incomplete.len(), 1);
    assert_eq!(report.incomplete[0].missing, set(&[5]));
    assert!(report.complete.is_empty());
}

#[test]
fn each_split_circle_of_a_card_is_judged_on_its_own() {
    // Two split circles at two levels: red|blue Lv.5 / 4 and red|blue Lv.6 / 2.
    let printed = [(0, 5, 4), (1, 5, 4), (0, 6, 2), (1, 6, 2)];
    // The Lv.5 one is complete, the Lv.6 one has red only.
    let report = split_circles(&printed, &[(0, 5, 4), (1, 5, 4), (0, 6, 2)]);
    assert_eq!(report.complete, vec![(5, 4)]);
    assert_eq!(report.incomplete.len(), 1);
    assert_eq!((report.incomplete[0].level, report.incomplete[0].cost), (6, 2));
    assert_eq!(report.incomplete[0].missing, set(&[1]));

    // Two costs at ONE level are two circles too.
    let printed = [(0, 5, 4), (1, 5, 4), (0, 5, 3), (1, 5, 3)];
    let report = split_circles(&printed, &[(0, 5, 4), (1, 5, 4), (1, 5, 3)]);
    assert_eq!(report.complete, vec![(5, 4)]);
    assert_eq!(report.incomplete[0].missing, set(&[0]));

    // A circle the spec does not author at all is reported as such, and does
    // not excuse the one it half-authors.
    let printed = [(0, 5, 4), (1, 5, 4), (0, 6, 2), (1, 6, 2)];
    let report = split_circles(&printed, &[(0, 5, 4)]);
    assert_eq!(report.unauthored, vec![(6, 2)]);
    assert_eq!(report.incomplete.len(), 1);
}

#[test]
fn a_second_colour_at_another_cost_or_level_does_not_complete_the_circle() {
    let printed = [(1, 4, 4), (3, 4, 4)];
    // Green authored at cost 5 (the cost rule reports the 5 as well).
    let wrong_cost = split_circles(&printed, &[(1, 4, 4), (3, 4, 5)]);
    assert_eq!(wrong_cost.incomplete.len(), 1);
    assert_eq!(wrong_cost.incomplete[0].missing, set(&[3]));
    // Green authored at Lv.5.
    let wrong_level = split_circles(&printed, &[(1, 4, 4), (3, 5, 4)]);
    assert_eq!(wrong_level.incomplete.len(), 1);
    assert_eq!(wrong_level.incomplete[0].missing, set(&[3]));
    // A colour the circle does not print does not stand in for one it does.
    let wrong_colour = split_circles(&printed, &[(1, 4, 4), (6, 4, 4)]);
    assert_eq!(wrong_colour.incomplete.len(), 1);
    assert_eq!(wrong_colour.incomplete[0].authored, set(&[1]));
}

#[test]
fn plain_and_rainbow_circles_are_not_split_circles() {
    // One colour: a plain circle.
    assert_eq!(split_circles(&[(0, 3, 2)], &[(0, 3, 2)]), SplitCircles::default());
    // Seven colours: a rainbow circle, authored colour-free.
    let rainbow: Vec<Row> = (0u8..7).map(|c| (c, 4, 4)).collect();
    assert_eq!(split_circles(&rainbow, &[]), SplitCircles::default());
    // Two plain circles of different levels are not one split circle.
    assert_eq!(
        split_circles(&[(0, 3, 2), (1, 4, 3)], &[(0, 3, 2)]),
        SplitCircles::default()
    );
}

#[test]
fn a_phantom_row_leaves_the_baseline_of_its_card_and_no_other() {
    // Cendrillmon (EX7-030): the baseline lists yellow and black Lv.5 / 4, the
    // card prints yellow only.
    let listed: Vec<Row> = vec![(2, 5, 4), (5, 5, 4)];
    let printed = without_phantom_rows("EX7-030", listed.clone());
    assert_eq!(printed, vec![(2, 5, 4)]);
    // With the phantom gone the circle is a plain one: nothing asks the spec
    // for black.
    assert_eq!(split_circles(&printed, &[(2, 5, 4)]), SplitCircles::default());
    // The same rows on another card are left alone.
    assert_eq!(without_phantom_rows("EX7-031", listed.clone()), listed);

    // Skadimon (EX8-028): a three-colour baseline circle with one phantom is
    // still a two-colour split circle, and still has to be whole.
    let skadimon = without_phantom_rows("EX8-028", vec![(1, 5, 4), (2, 5, 4), (6, 5, 4)]);
    assert_eq!(skadimon, vec![(1, 5, 4), (2, 5, 4)]);
    assert_eq!(split_circles(&skadimon, &[(1, 5, 4)]).incomplete.len(), 1);
    assert_eq!(split_circles(&skadimon, &[(1, 5, 4), (2, 5, 4)]).complete, vec![(5, 4)]);
}

#[test]
fn every_phantom_row_names_a_real_colour_once() {
    let mut seen = BTreeSet::new();
    for (card_id, colour, level, cost, prints) in BASELINE_PHANTOM_ROWS {
        assert!(!prints.is_empty(), "{card_id}: say what the card prints");
        assert!(
            seen.insert((*card_id, colour_code(colour), *level, *cost)),
            "{card_id}: {colour} Lv.{level} / cost {cost} is listed twice"
        );
    }
}
