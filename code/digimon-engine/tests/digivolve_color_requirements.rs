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
