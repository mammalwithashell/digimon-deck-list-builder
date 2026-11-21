//! The printed standard digivolve circle of the DSL fixtures.
//!
//! DebugRunner builds a card's data from its YAML spec and never reads
//! `data/cards.json`, so the spec's `alt_paths` are the ONLY place a fixture
//! learns which level and colour the card digivolves from. Two faults lived
//! there (bug scan of 1,224 logged games, 2026-11-21):
//!
//! * **Rainbow circles** (official EN DB "Multicolor N from Lv.X", JP 「全」 =
//!   all colours, DCGO one `EvoCosts` row per colour) are ANY colour.
//!   Kimeramon (BT8-084), Kimeramon (EX9-074), Susanoomon (BT18-102) and
//!   Omnimon: Merciful Mode (EX13-077) did not author theirs at all, and
//!   Susanoomon (EX12-076) authored it as `self_color_count_gte: 2` ("a Lv.6
//!   with 2+ colours").
//! * **Colour-specific circles** were authored colour-free
//!   (`from: { level_eq: N }`) on 35 specs, so the card digivolved from any
//!   colour: Dimetromon (P-189) and Cyclonemon (BT24-011), printed
//!   "Red Lv.3: 2", onto the white Ukkomon (BT16-082).
//!
//! Rules: CR 4.3 §2-3-5-1/-2 (a digivolution requirement is the colour and
//! level of the card digivolved from, and the cost), §8-1-3-1 (the chosen card
//! must meet the chosen requirement). The tables are in
//! `tests/support/printed_circles.rs`; each row was read off the card image.
//! The production twin of this file is `tests/digivolve_color_requirements.rs`.

use crate::printed_circles::{ALL_COLOURS, COLOURED_CIRCLES, RAINBOW_CIRCLES, RAINBOW_CIRCLE_SPECS};

use digimon_engine::action::space::encode_digivolve;
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};

/// A single-colour Lv.`level` Digimon with a name and a trait no special
/// digivolution condition in the pool asks for.
fn plain_base(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(4000);
    c.colors = vec![color];
    c.traits = vec!["Beast".to_string()];
    c
}

/// One plain Lv.`level` base per colour, in `ALL_COLOURS` order.
fn one_base_per_colour(level: u8) -> Vec<CardData> {
    ALL_COLOURS
        .iter()
        .map(|c| plain_base(&format!("BASE-{c:?}"), level, *c))
        .collect()
}

/// The DSL fixture `card_id` in hand and every one of `bases` on the field, in
/// Main phase with 10 memory. Returns the runner and each base's field slot.
/// (Every DSL runner builds the whole effect registry, so one runner carries
/// all the bases of a sweep.)
fn staged(card_id: &str, bases: Vec<CardData>) -> (DebugRunner, Vec<usize>) {
    let ids: Vec<String> = bases.iter().map(|b| b.card_id.clone()).collect();
    let mut builder = DebugRunner::builder()
        .dsl_card(card_id)
        .unwrap_or_else(|e| panic!("{card_id} in the embedded DSL pack: {e}"));
    for base in bases {
        builder = builder.add_card(base);
    }
    let mut r = builder.hand(0, &[card_id]).memory(10).start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slots = ids
        .iter()
        .map(|id| r.place_on_field(0, id, Some(0)).index as usize)
        .collect();
    (r, slots)
}

/// For each staged base: does the action mask offer digivolving the hand card
/// onto it?
fn mask_offers(r: &DebugRunner, slots: &[usize]) -> Vec<bool> {
    let mask = build_action_mask(&r.game, 0);
    slots
        .iter()
        .map(|slot| mask[encode_digivolve(0, *slot as u16) as usize] == 1.0)
        .collect()
}

/// Digivolve the hand card `card_id` onto field `slot`: `Ok(memory paid)` when
/// the action puts the card on top, `Err(why)` otherwise.
fn digivolve_cost(r: &mut DebugRunner, card_id: &str, slot: usize) -> Result<i16, String> {
    let mem_before = r.game.memory;
    if !r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand) {
        return Err("the digivolve action was rejected".to_string());
    }
    let top = r.game.players[0].battle_area[slot]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string();
    if top != card_id {
        return Err(format!(
            "the digivolve did not complete (top card is still {top}; a cost choice is pending)"
        ));
    }
    Ok(mem_before - r.game.memory)
}

#[test]
fn rainbow_circle_specs_digivolve_from_every_colour_at_the_printed_cost() {
    let mut failures: Vec<String> = Vec::new();
    for (id, level, cost) in RAINBOW_CIRCLES {
        if !RAINBOW_CIRCLE_SPECS.contains(id) {
            continue;
        }
        // One base per colour at the printed level, then a red one a level
        // below and a level above: the circle is any COLOUR, not any level.
        let mut bases = one_base_per_colour(*level);
        bases.push(plain_base("BASE-LOW", level - 1, CardColor::Red));
        bases.push(plain_base("BASE-HIGH", level + 1, CardColor::Red));
        let (mut r, slots) = staged(id, bases);
        let offered = mask_offers(&r, &slots);
        for (i, color) in ALL_COLOURS.iter().enumerate() {
            if !offered[i] {
                failures.push(format!(
                    "{id}: the mask does not offer the digivolve over a {color:?} Lv.{level} \
                     (rainbow circle = any colour)"
                ));
            }
        }
        if offered[7] || offered[8] {
            failures.push(format!(
                "{id}: the Lv.{level} circle was offered over a base of another level \
                 (below: {}, above: {})",
                offered[7], offered[8]
            ));
        }
        // The action itself, over a colour the old red-only data rejected
        // (ALL_COLOURS[1] is Blue: Kimeramon (BT8-084) onto the blue Shellmon).
        let got = digivolve_cost(&mut r, id, slots[1]);
        if got != Ok(*cost) {
            failures.push(format!(
                "{id} over a Blue Lv.{level}: expected the printed cost {cost}, got {got:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} rainbow-circle spec fault(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every rainbow-circle card that has a spec is in the list this file sweeps
/// (so a newly implemented one — BigUkkomon (BT16-083), Susanoomon (BT7-112),
/// Luminamon (Nene Version) (BT19-102) — is not skipped silently).
#[test]
fn every_implemented_rainbow_circle_card_is_swept() {
    let registry = digimon_engine::dsl_registry::from_embedded_cached().expect("embedded pack");
    for (id, _, _) in RAINBOW_CIRCLES {
        assert_eq!(
            registry.lookup(id).is_some(),
            RAINBOW_CIRCLE_SPECS.contains(id),
            "{id}: RAINBOW_CIRCLE_SPECS (tests/support/printed_circles.rs) must list exactly \
             the rainbow-circle cards that have a YAML spec"
        );
    }
}

/// Sweep every `stride`-th row of `COLOURED_CIRCLES` starting at `offset` (the
/// table is split over three tests so they run in parallel).
fn check_coloured_circles(offset: usize, stride: usize) {
    let mut failures: Vec<String> = Vec::new();
    for (id, level, cost, printed) in COLOURED_CIRCLES.iter().skip(offset).step_by(stride) {
        let (mut r, slots) = staged(id, one_base_per_colour(*level));
        let offered = mask_offers(&r, &slots);
        for (i, color) in ALL_COLOURS.iter().enumerate() {
            match (printed.contains(color), offered[i]) {
                (true, false) => failures.push(format!(
                    "{id}: the mask does not offer the digivolve over a {color:?} Lv.{level}, \
                     a printed circle colour"
                )),
                (false, true) => failures.push(format!(
                    "{id}: the mask offers the digivolve over a {color:?} Lv.{level}, but the \
                     card prints only {printed:?} circles"
                )),
                _ => {}
            }
        }
        // The action itself, once per printed colour: the first in the sweep
        // runner, any further colour in a fresh one (the hand card is spent).
        for (n, color) in printed.iter().enumerate() {
            let got = if n == 0 {
                let slot = slots[ALL_COLOURS.iter().position(|c| c == color).unwrap()];
                digivolve_cost(&mut r, id, slot)
            } else {
                let (mut fresh, fresh_slots) =
                    staged(id, vec![plain_base("PLAIN-BASE", *level, *color)]);
                digivolve_cost(&mut fresh, id, fresh_slots[0])
            };
            if got != Ok(*cost) {
                failures.push(format!(
                    "{id} over a {color:?} Lv.{level}: expected the printed cost {cost}, got {got:?}"
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} colour-specific circle spec fault(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn colour_specific_specs_take_only_their_printed_colours_part_1() {
    check_coloured_circles(0, 3);
}

#[test]
fn colour_specific_specs_take_only_their_printed_colours_part_2() {
    check_coloured_circles(1, 3);
}

#[test]
fn colour_specific_specs_take_only_their_printed_colours_part_3() {
    check_coloured_circles(2, 3);
}

/// The logged illegal digivolve, on the DSL fixtures: Dimetromon (P-189) onto
/// the white Ukkomon (BT16-082) (glowing-dawn--medusamon pair-001-0 step 7).
#[test]
fn dimetromon_p_189_spec_rejects_the_white_ukkomon() {
    let mut r = DebugRunner::builder()
        .dsl_card("P-189")
        .expect("P-189 in the embedded DSL pack")
        .dsl_card("BT16-082")
        .expect("BT16-082 in the embedded DSL pack")
        .hand(0, &["P-189"])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "BT16-082", Some(0)).index as usize;

    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        mask[encode_digivolve(0, slot as u16) as usize],
        0.0,
        "mask must not offer Dimetromon (P-189) onto the white Ukkomon (BT16-082)"
    );
    assert!(
        !r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand),
        "a red Lv.3 circle does not accept a white Lv.3"
    );
}

/// The `evo_costs` table DebugRunner builds from a spec's `alt_paths` (it never
/// reads `cards.json`) holds each printed circle exactly as production does: one
/// row per printed colour, seven for a rainbow circle. `Game::can_digivolve`
/// (Blast Digivolve) and the borrowed-circle routes read this table, so a
/// rainbow fixture with an empty table refused every base.
#[test]
fn fixture_evo_costs_hold_every_printed_circle_colour() {
    let mut failures: Vec<String> = Vec::new();
    let mut check = |id: &str, level: u8, cost: i16, want: Vec<CardColor>| {
        let r = DebugRunner::builder()
            .dsl_card(id)
            .unwrap_or_else(|e| panic!("{id} in the embedded DSL pack: {e}"))
            .build();
        let data = r
            .game
            .card_data
            .iter()
            .find(|c| c.card_id == id)
            .unwrap_or_else(|| panic!("{id} staged in the fixture"));
        let mut got: Vec<u8> = data
            .digivolution_costs()
            .iter()
            .filter(|e| e.level == level && i16::try_from(e.memory_cost) == Ok(cost))
            .map(|e| e.card_color)
            .collect();
        got.sort_unstable();
        got.dedup();
        let mut want: Vec<u8> = want.iter().map(|c| *c as u8).collect();
        want.sort_unstable();
        if got != want {
            failures.push(format!(
                "{id}: the fixture's Lv.{level} / cost {cost} circle has colour codes {got:?},                  the printed circle has {want:?}"
            ));
        }
    };
    for (id, level, cost) in RAINBOW_CIRCLES {
        if RAINBOW_CIRCLE_SPECS.contains(id) {
            check(id, *level, *cost, ALL_COLOURS.to_vec());
        }
    }
    for (id, level, cost, printed) in COLOURED_CIRCLES {
        check(id, *level, *cost, printed.to_vec());
    }
    assert!(
        failures.is_empty(),
        "{} fixture evo-cost fault(s):
{}",
        failures.len(),
        failures.join("
")
    );
}
