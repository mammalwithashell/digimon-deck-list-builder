//! Regression tests for two user-reported digivolution color-requirement bugs.
//!
//! Bug 1 — "Color requirements for digivolution is based on the primary color
//! (observed on BT25-052, BT25-056)". Root cause was DATA, not engine logic:
//! the digimoncard.io API ingest behind `data/cards.json` dropped the
//! off-primary-colour digivolve circle of dual-colour cards, so e.g.
//! BT25-052 Logimon (Green/Red, printed circles "Green Lv.3: 3" AND
//! "Red Lv.3: 3") carried only the primary Green circle — a red Lv.3 base
//! was wrongly rejected. The circles were restored from the official Bandai
//! DB (`ded458abf`, then the pool-wide 826-card sweep `71a2f3b15`, guarded by
//! `tests/data_official_parity.rs`). These tests pin the END-TO-END behavior
//! on the PRODUCTION CardData (`deck_tools::full_card_data()`): if a future
//! re-ingest drops the off-primary circle again, or the digivolve legality
//! path ever collapses to the base's/card's first colour, they fail.
//!
//! Bug 2 — "Digi-egg not giving color requirements (observed on ST1-01)".
//! ST1-01 Koromon is a red Lv.2 raised in the breeding area; a Lv.3 may only
//! digivolve onto it when one of the Lv.3's printed circles matches the
//! egg-Digimon's colour and level (e.g. ST1-03 Agumon: "Red Lv.2: 0"). These
//! tests pin that the breeding-area digivolve path (`can_digivolve` +
//! `digivolve_from_hand_onto_breeding` + the action mask's breeding branch)
//! enforces the colour requirement: a red Lv.3 digivolves, an off-colour
//! (blue-circle) Lv.3 is rejected.

use digimon_engine::action::space::{encode_digivolve, BREEDING_TARGET};
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, PlaySource};
use digimon_engine::permanent::Permanent;

#[path = "support/printed_circles.rs"]
mod printed_circles;
use printed_circles::{ALL_COLOURS, COLOURED_CIRCLES, RAINBOW_CIRCLES};

/// Production card data parses all of cards.json — run each test body on a
/// large-stack thread (same pattern as `digivolve_cost_choice_archetypes.rs`).
fn on_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn large-stack thread")
        .join()
        .expect("test thread panicked");
}

fn production_card(card_id: &str) -> CardData {
    digimon_engine::deck_tools::full_card_data()
        .get(card_id)
        .unwrap_or_else(|| panic!("{card_id} present in production cards.json"))
        .clone()
}

/// A synthetic single-colour Lv.`level` Digimon base with no traits that
/// could satisfy any alt-digivolve path (e.g. BT25-052's "[Standard App]
/// trait Lv.3: cost 2" box).
fn plain_base(id: &str, level: u8, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(4000);
    c.colors = vec![color];
    c.traits = vec!["Beast".to_string()];
    c
}

/// Stage `evolver_id` (production CardData) in hand atop a synthetic base on
/// the battle area; returns (runner, base slot).
fn field_runner(evolver: CardData, base: CardData) -> (DebugRunner, usize) {
    let evolver_id = evolver.card_id.clone();
    let base_id = base.card_id.clone();
    let mut r = DebugRunner::builder()
        .add_card(evolver)
        .add_card(base)
        .hand(0, &[&evolver_id])
        .memory(10)
        .start();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, &base_id, Some(0)).index as usize;
    (r, slot)
}

/// Assert `evolver` digivolves over `base` paying exactly `cost` memory.
fn assert_field_digivolve_ok(evolver: CardData, base: CardData, cost: i16) {
    let evolver_id = evolver.card_id.clone();
    let (mut r, slot) = field_runner(evolver, base.clone());

    // The action mask must offer the digivolve (hand 0 → field `slot`).
    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 1.0,
        "{evolver_id}: mask must offer digivolve over {} (Lv.{:?} {:?})",
        base.card_id, base.level, base.colors
    );

    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        ok,
        "{evolver_id}: digivolve over {} must succeed (printed circle matches)",
        base.card_id
    );
    assert_eq!(
        r.game.players[0].battle_area[slot]
            .top_card()
            .card_id(&r.game.card_data),
        evolver_id,
        "{evolver_id}: evolver lands on top"
    );
    assert_eq!(
        mem_before - r.game.memory,
        cost,
        "{evolver_id}: printed circle cost {cost} is paid"
    );
}

/// Assert `evolver` can NOT digivolve over `base` (no printed circle or alt
/// path matches) — both at the mask layer and at the action layer.
fn assert_field_digivolve_rejected(evolver: CardData, base: CardData) {
    let evolver_id = evolver.card_id.clone();
    let (mut r, slot) = field_runner(evolver, base.clone());

    let mask = build_action_mask(&r.game, 0);
    let action = encode_digivolve(0, slot as u16) as usize;
    assert_eq!(
        mask[action], 0.0,
        "{evolver_id}: mask must NOT offer digivolve over {} (Lv.{:?} {:?})",
        base.card_id, base.level, base.colors
    );

    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    assert!(
        !ok,
        "{evolver_id}: digivolve over {} must be rejected (no matching circle)",
        base.card_id
    );
    assert_eq!(r.game.memory, mem_before, "{evolver_id}: no memory paid");
    assert_eq!(
        r.game.players[0].hand.len(),
        1,
        "{evolver_id}: card stays in hand"
    );
}

// ─── Bug 1: BT25-052 Logimon — Green/Red, circles Green Lv.3: 3 + Red Lv.3: 3 ──

/// The reported failure mode: digivolving over the OFF-PRIMARY (Red) colour.
/// Before the official-DB circle recovery only the primary Green circle
/// existed, so this exact digivolve was wrongly rejected.
#[test]
fn bt25_052_digivolves_over_off_primary_red_lv3_base() {
    on_big_stack(|| {
        assert_field_digivolve_ok(
            production_card("BT25-052"),
            plain_base("RED-BASE", 3, CardColor::Red),
            3,
        );
    });
}

#[test]
fn bt25_052_digivolves_over_primary_green_lv3_base() {
    on_big_stack(|| {
        assert_field_digivolve_ok(
            production_card("BT25-052"),
            plain_base("GREEN-BASE", 3, CardColor::Green),
            3,
        );
    });
}

/// Negative control: a colour on NEITHER printed circle (and no [Standard
/// App] trait for the alt box) stays illegal.
#[test]
fn bt25_052_rejected_over_unmatched_blue_lv3_base() {
    on_big_stack(|| {
        assert_field_digivolve_rejected(
            production_card("BT25-052"),
            plain_base("BLUE-BASE", 3, CardColor::Blue),
        );
    });
}

// ─── Bug 1: BT25-056 Bootmon — Green/Yellow, circles Green Lv.4: 4 + Yellow Lv.4: 4 ──

/// The reported failure mode for BT25-056: the off-primary (Yellow) circle.
#[test]
fn bt25_056_digivolves_over_off_primary_yellow_lv4_base() {
    on_big_stack(|| {
        assert_field_digivolve_ok(
            production_card("BT25-056"),
            plain_base("YELLOW-BASE", 4, CardColor::Yellow),
            4,
        );
    });
}

#[test]
fn bt25_056_digivolves_over_primary_green_lv4_base() {
    on_big_stack(|| {
        assert_field_digivolve_ok(
            production_card("BT25-056"),
            plain_base("GREEN-BASE", 4, CardColor::Green),
            4,
        );
    });
}

#[test]
fn bt25_056_rejected_over_unmatched_red_lv4_base() {
    on_big_stack(|| {
        assert_field_digivolve_rejected(
            production_card("BT25-056"),
            plain_base("RED-BASE", 4, CardColor::Red),
        );
    });
}

// ─── Bug 2: ST1-01 Koromon — red Lv.2 in the breeding area ────────────────────

/// A synthetic Lv.3 whose only printed circle is `color` Lv.2: 0 — the
/// minimal evolver for the breeding-colour gate.
fn lv3_with_circle(id: &str, color: CardColor, circle_color_code: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(2000);
    c.play_cost = 2;
    c.colors = vec![color];
    c.evo_costs = vec![EvoCost {
        card_color: circle_color_code,
        level: 2,
        memory_cost: 0,
    }];
    c
}

/// Stage production ST1-01 Koromon into P0's breeding area with `evolver` in
/// hand; returns the runner in Main phase.
fn breeding_runner(evolver: CardData) -> DebugRunner {
    let evolver_id = evolver.card_id.clone();
    let mut r = DebugRunner::builder()
        .add_card(production_card("ST1-01"))
        .add_card(evolver)
        .hand(0, &[&evolver_id])
        .memory(10)
        .start();

    // Place ST1-01 into the breeding area directly (bypass hatch machinery —
    // same staging as `cost_hooks/cost_reduction_fn.rs`).
    {
        let game = r.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "ST1-01")
            .expect("ST1-01 staged in card data");
        let ci = game.next_card_index();
        let card = CardSource::new(data_idx, 0, ci);
        let turn = game.turn_count;
        game.players[0].breeding_area = Some(Permanent::new(card, turn));
    }
    r.game_mut().enter_main_phase();
    r
}

/// The real ST1 line: ST1-03 Agumon prints "Red Lv.2: 0" and Koromon is red —
/// the breeding digivolve is legal and free.
#[test]
fn st1_01_breeding_accepts_matching_red_lv3_production_agumon() {
    on_big_stack(|| {
        let agumon = production_card("ST1-03");
        assert!(
            agumon
                .evo_costs
                .iter()
                .any(|ec| ec.card_color == 0 && ec.level == 2),
            "ST1-03 production data must print the Red Lv.2 circle; got {:?}",
            agumon.evo_costs
        );
        let mut r = breeding_runner(agumon);

        // Mask must offer the breeding digivolve...
        let mask = build_action_mask(&r.game, 0);
        let action = encode_digivolve(0, BREEDING_TARGET) as usize;
        assert_eq!(
            mask[action], 1.0,
            "mask offers ST1-03 digivolve onto breeding ST1-01"
        );

        // ...and the action succeeds, free of charge.
        let mem_before = r.game.memory;
        let ok = r
            .game_mut()
            .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
        assert!(ok, "red Agumon digivolves onto red Koromon in breeding");
        assert_eq!(r.game.memory, mem_before, "printed circle costs 0");
        let breeding = r.game.players[0].breeding_area.as_ref().unwrap();
        assert_eq!(
            breeding.top_card().card_id(&r.game.card_data),
            "ST1-03",
            "Agumon on top of the breeding stack"
        );
    });
}

/// The reported failure mode: the egg-Digimon's colour requirement must be
/// ENFORCED. A Lv.3 whose only circle is "Blue Lv.2: 0" cannot digivolve
/// onto the red Koromon — at the legality predicate, the mask, and the action.
#[test]
fn st1_01_breeding_rejects_off_color_blue_lv3() {
    on_big_stack(|| {
        // Blue-circle evolver (circle colour code 1 = Blue).
        let blue = lv3_with_circle("BLUE-EVOLVER", CardColor::Blue, 1);
        let mut r = breeding_runner(blue);

        // Legality predicate (shared by mask + Blast counter path).
        let card = r.game.players[0].hand[0].clone();
        let breeding = r.game.players[0].breeding_area.clone().unwrap();
        assert!(
            !r.game.can_digivolve(&card, &breeding),
            "blue-circle Lv.3 must NOT satisfy red Koromon's colour"
        );

        // Mask must not offer it.
        let mask = build_action_mask(&r.game, 0);
        let action = encode_digivolve(0, BREEDING_TARGET) as usize;
        assert_eq!(
            mask[action], 0.0,
            "mask must NOT offer the off-colour breeding digivolve"
        );

        // Action layer rejects it.
        let mem_before = r.game.memory;
        let ok = r
            .game_mut()
            .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
        assert!(!ok, "off-colour breeding digivolve must be rejected");
        assert_eq!(r.game.memory, mem_before, "no memory paid");
        let breeding = r.game.players[0].breeding_area.as_ref().unwrap();
        assert_eq!(
            breeding.card_sources.len(),
            1,
            "breeding stack unchanged (Koromon alone)"
        );
    });
}

/// Same-colour control for the synthetic evolver shape: a "Red Lv.2: 0"
/// circle IS accepted — proving the rejection above is the colour gate, not
/// an artifact of the synthetic card.
#[test]
fn st1_01_breeding_accepts_matching_synthetic_red_lv3() {
    on_big_stack(|| {
        let red = lv3_with_circle("RED-EVOLVER", CardColor::Red, 0);
        let mut r = breeding_runner(red);
        let ok = r
            .game_mut()
            .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
        assert!(ok, "red-circle Lv.3 digivolves onto red Koromon");
    });
}

// ─── Printed circle colours across the pool (bug scan of 1,224 logged games, 2026-11-21) ──
//
// Two data/spec faults, one in each direction:
//
// * RAINBOW circles lost. A rainbow circle (official EN DB "Multicolor N from
//   Lv.X", JP 「全」 = all, DCGO one EvoCosts row per colour) lets the card
//   digivolve from a Digimon of that level of ANY colour. cards.json held it as
//   one red row — or, for Omnimon: Merciful Mode (EX13-077), as no row at all —
//   so in the logs EX13-077 was never offered its Lv.6 digivolve (0 of 566
//   opportunities) and Kimeramon (BT8-084) could not digivolve from a blue or
//   purple Lv.4.
// * Colour-specific circles authored colour-free. 35 YAML specs wrote the
//   printed circle as a bare `from: { level_eq: N }` alt-path, which is a real
//   route from a base of any colour: Dimetromon (P-189) and Cyclonemon
//   (BT24-011), both printed "Red Lv.3: 2", digivolved onto the white Ukkomon
//   (BT16-082).
//
// Rules: CR 4.3 §2-3-5-1/-2 (the printed digivolution requirement is the
// colour and level of the card digivolved from, plus the cost), §8-1-3-1 (the
// player picks one requirement and a card "that meets the requirement").
// These tests run the PRODUCTION card data through the mask and the action.

/// Digivolve production `evolver_id` from hand over a plain single-colour
/// Lv.`level` base: `Ok(memory paid)` when the mask offers it and the action
/// puts the card on top, `Err(why)` otherwise.
fn probe_field_digivolve(
    data: &std::collections::HashMap<String, CardData>,
    evolver_id: &str,
    level: u8,
    color: CardColor,
) -> Result<i16, String> {
    let evolver = data
        .get(evolver_id)
        .unwrap_or_else(|| panic!("{evolver_id} present in production cards.json"))
        .clone();
    let (mut r, slot) = field_runner(evolver, plain_base("PLAIN-BASE", level, color));
    let mask = build_action_mask(&r.game, 0);
    let offered = mask[encode_digivolve(0, slot as u16) as usize] == 1.0;
    let mem_before = r.game.memory;
    let ok = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
    if offered != ok {
        return Err(format!("mask offered = {offered} but the action returned {ok}"));
    }
    if !ok {
        return Err("no digivolve route".to_string());
    }
    let top = r.game.players[0].battle_area[slot]
        .top_card()
        .card_id(&r.game.card_data)
        .to_string();
    if top != evolver_id {
        // More than one cost applies: the engine parks a "which digivolution
        // cost do you pay?" choice instead of digivolving.
        return Err(format!(
            "the digivolve did not complete (top card is still {top}; a cost choice is pending)"
        ));
    }
    Ok(mem_before - r.game.memory)
}

#[test]
fn rainbow_circle_cards_digivolve_from_every_colour_at_the_printed_cost() {
    on_big_stack(|| {
        let data = digimon_engine::deck_tools::full_card_data();
        let mut failures: Vec<String> = Vec::new();
        for (id, level, cost) in RAINBOW_CIRCLES {
            for color in ALL_COLOURS {
                match probe_field_digivolve(&data, id, *level, color) {
                    Ok(paid) if paid == *cost => {}
                    other => failures.push(format!(
                        "{id} over a {color:?} Lv.{level}: expected the printed cost {cost}, got {other:?}"
                    )),
                }
            }
            // The circle is any COLOUR, not any level.
            for wrong_level in [level - 1, level + 1] {
                if let Ok(paid) = probe_field_digivolve(&data, id, wrong_level, CardColor::Red) {
                    failures.push(format!(
                        "{id} over a red Lv.{wrong_level}: the Lv.{level} circle must not apply, \
                         but the digivolve was legal for {paid}"
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} rainbow-circle digivolve fault(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    });
}

#[test]
fn colour_specific_circles_take_only_their_printed_colours_at_the_printed_cost() {
    on_big_stack(|| {
        let data = digimon_engine::deck_tools::full_card_data();
        let mut failures: Vec<String> = Vec::new();
        for (id, level, cost, printed) in COLOURED_CIRCLES {
            for color in ALL_COLOURS {
                let got = probe_field_digivolve(&data, id, *level, color);
                if printed.contains(&color) {
                    if got != Ok(*cost) {
                        failures.push(format!(
                            "{id} over a {color:?} Lv.{level}: expected the printed cost {cost}, got {got:?}"
                        ));
                    }
                } else if let Ok(paid) = got {
                    failures.push(format!(
                        "{id} over a {color:?} Lv.{level}: the card prints only {printed:?} \
                         circles, but the digivolve was legal for {paid}"
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} colour-specific circle fault(s):\n{}",
            failures.len(),
            failures.join("\n")
        );
    });
}

/// Production twin of the 2026-11-22 spec completion: the 99 cards whose
/// specs authored one colour of a printed split circle. A spec's `alt_paths`
/// are live in production too (`collect_dsl_alt_digivolve_routes` adds them
/// beside the `cards.json` circles, and equal routes collapse into one), so
/// the claim "production already had every colour, the specs only caught up"
/// is checked here, on the production card data: each card digivolves over
/// exactly its printed colours, at the printed cost, with no cost choice.
///
/// Mastemon (EX6-029) is the one card that does not: `cards.json` lists a
/// yellow|purple Lv.5 / cost 5 pair beside the printed cost 4, which the card
/// face does not print, so production asks which cost to pay. That is a data
/// error this test pins rather than hides (see `BASELINE_PHANTOM_ROWS` in
/// `tests/alt_path_printed_cost_guard.rs`): when the rows are removed the
/// assertion on the cost choice fails and the exception has to go.
#[test]
fn completed_split_circles_take_exactly_their_printed_colours_in_production() {
    use printed_circles::SPLIT_CIRCLES_COMPLETED;
    const COST_CHOICE_FROM_A_ROW_THE_CARD_DOES_NOT_PRINT: &[&str] = &["EX6-029"];
    on_big_stack(|| {
        let data = digimon_engine::deck_tools::full_card_data();
        let mut failures: Vec<String> = Vec::new();
        for (id, level, cost, printed) in SPLIT_CIRCLES_COMPLETED {
            let known_bad = COST_CHOICE_FROM_A_ROW_THE_CARD_DOES_NOT_PRINT.contains(id);
            for color in ALL_COLOURS {
                if known_bad && printed.contains(&color) {
                    // The action pauses on a two-option "which digivolution
                    // cost do you pay?" choice (the printed 4 and the phantom
                    // 5) instead of digivolving.
                    let evolver = data.get(*id).expect("in production cards.json").clone();
                    let (mut r, slot) =
                        field_runner(evolver, plain_base("PLAIN-BASE", *level, color));
                    let proceeded = r.game.digivolve_from_hand(0, 0, slot, PlaySource::ByHand);
                    let options = r
                        .pending_selection_view()
                        .and_then(|view| view.effect_choices.as_ref().map(|c| c.len()));
                    assert!(
                        !proceeded && options == Some(2),
                        "{id} over a {color:?} Lv.{level}: expected the cost choice its \
                         phantom cards.json row causes, got proceeded = {proceeded}, options = \
                         {options:?}. If the row left cards.json, delete this exception."
                    );
                    continue;
                }
                let got = probe_field_digivolve(&data, id, *level, color);
                if printed.contains(&color) {
                    if got != Ok(*cost) {
                        failures.push(format!(
                            "{id} over a {color:?} Lv.{level}: expected the printed cost {cost}, got {got:?}"
                        ));
                    }
                } else if let Ok(paid) = got {
                    failures.push(format!(
                        "{id} over a {color:?} Lv.{level}: the card prints only {printed:?} \
                         circles, but the digivolve was legal for {paid}"
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} split-circle fault(s) in production:\n{}",
            failures.len(),
            failures.join("\n")
        );
    });
}

/// Stage production `base_id` in P0's breeding area with production
/// `evolver_id` in hand; returns the runner in Main phase.
fn production_breeding_runner(base_id: &str, evolver_id: &str) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(production_card(base_id))
        .add_card(production_card(evolver_id))
        .hand(0, &[evolver_id])
        .memory(10)
        .start();
    {
        let game = r.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == base_id)
            .expect("base staged in card data");
        let ci = game.next_card_index();
        let card = CardSource::new(data_idx, 0, ci);
        let turn = game.turn_count;
        game.players[0].breeding_area = Some(Permanent::new(card, turn));
    }
    r.game_mut().enter_main_phase();
    r
}

/// The logged illegal digivolve: Dimetromon (P-189), printed "Red Lv.3: 2",
/// onto the white Lv.3 Ukkomon (BT16-082) in the breeding area
/// (glowing-dawn--medusamon pair-001-0 step 7; light-hybrid--medusamon
/// pair-001-1 step 48). Ukkomon is white with no trait any route asks for, so
/// it meets no requirement of the card (CR 4.3 §8-1-3-1).
#[test]
fn dimetromon_p_189_cannot_digivolve_onto_the_white_ukkomon_in_breeding() {
    on_big_stack(|| {
        let mut r = production_breeding_runner("BT16-082", "P-189");
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(
            mask[encode_digivolve(0, BREEDING_TARGET) as usize],
            0.0,
            "mask must not offer Dimetromon (P-189) onto the white Ukkomon (BT16-082)"
        );
        let mem_before = r.game.memory;
        let ok = r
            .game_mut()
            .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
        assert!(!ok, "a red Lv.3 circle does not accept a white Lv.3");
        assert_eq!(r.game.memory, mem_before, "no memory paid");
    });
}

/// Control for the test above: the same card onto a RED Lv.3 in breeding —
/// Elizamon (BT21-008) — is legal for the printed 2.
#[test]
fn dimetromon_p_189_digivolves_onto_a_red_lv3_in_breeding_for_two() {
    on_big_stack(|| {
        let mut r = production_breeding_runner("BT21-008", "P-189");
        let mask = build_action_mask(&r.game, 0);
        assert_eq!(
            mask[encode_digivolve(0, BREEDING_TARGET) as usize],
            1.0,
            "mask offers Dimetromon (P-189) onto the red Elizamon (BT21-008)"
        );
        let mem_before = r.game.memory;
        let ok = r
            .game_mut()
            .digivolve_from_hand_onto_breeding(0, 0, PlaySource::ByHand);
        assert!(ok, "red Lv.3 circle accepts a red Lv.3");
        assert_eq!(mem_before - r.game.memory, 2, "printed cost 2 is paid");
    });
}

/// The other logged case: Cyclonemon (BT24-011), printed "Red Lv.3: 2" plus
/// "[Digivolve] Lv.3 w/[TS] trait: Cost 2", was offered onto the white Ukkomon
/// (BT16-082) on the field (data-squad-ravemon--medusamon pair-002-1 steps
/// 12-13). Ukkomon is neither red nor [TS].
#[test]
fn cyclonemon_bt24_011_cannot_digivolve_onto_the_white_ukkomon() {
    on_big_stack(|| {
        assert_field_digivolve_rejected(production_card("BT24-011"), production_card("BT16-082"));
    });
}

/// Omnimon: Merciful Mode (EX13-077) over a Lv.6 that is not named [Omnimon]:
/// the printed rainbow "Lv.6: 6" circle applies (adventure--dynasmon
/// pair-000-1 step 43 had Dynasmon (BT23-035) on the field and no digivolve
/// action).
#[test]
fn omnimon_merciful_mode_ex13_077_digivolves_over_dynasmon_for_six() {
    on_big_stack(|| {
        assert_field_digivolve_ok(production_card("EX13-077"), production_card("BT23-035"), 6);
    });
}

/// Kimeramon (BT8-084) over the blue Shellmon (EX12-026): the logged blocked
/// digivolve (data-squad-ravemon--toho-braves pair-002-1 step 66).
#[test]
fn kimeramon_bt8_084_digivolves_over_the_blue_shellmon_for_four() {
    on_big_stack(|| {
        assert_field_digivolve_ok(production_card("BT8-084"), production_card("EX12-026"), 4);
    });
}
