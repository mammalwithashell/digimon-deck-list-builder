//! EX2-007 Mother D-Reaper — Digi-Egg, White, DP 15000.
//!
//! # Card text (official Bandai DB, data/card_bundles/EX2-007.md)
//!
//! ```text
//! [Main] [Once Per Turn] If you don't have another [Mother D-Reaper] in play,
//! place 1 of your [ADR-02 Searcher] from in play or from your hand under this
//! Digimon as its bottom digivolution card.
//! ```
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/EX2/White/EX2_007.cs`. The [Main]'s
//! `CanUseCondition` (:122-143) offers it only when no other [Mother D-Reaper]
//! Digimon is in play AND an [ADR-02 Searcher] can be chosen: a non-token own
//! battle-area permanent whose top card is named [ADR-02 Searcher]
//! (`CanSelectPermanentCondition`, :83-102) or a hand card so named
//! (`CanSelectCardCondition`, :104-120). Without one the [Main] is never
//! declared and no use is counted. Owner ruling 2026-11-24: "Generally let's
//! look to match dcgo".
//!
//! The full-card behaviour lives in `tests/ex2_007_mother_full_card.rs`; this
//! module covers the [Main]'s declarability gate.

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "EX2-007";
const SEARCHER: &str = "EX2-046";

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX2-007 YAML is in the embedded pack")
        .dsl_card(SEARCHER)
        .expect("EX2-046 YAML is in the embedded pack")
        .add_card(make_test_card("PAD", "Pad"))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
        .memory(5)
}

/// Mother D-Reaper alone on player 0's battle area, in the main phase.
fn mother(r: &mut DebugRunner) -> PermanentHandle {
    r.skip_mulligan();
    let h = r.place_stack(0, &[CARD_ID]);
    r.game.enter_main_phase();
    h
}

fn main_bit(h: PermanentHandle) -> usize {
    (FIELD_EFFECT_START
        + h.index as u16 * digimon_engine::action::space::EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN) as usize
}

fn offered(r: &DebugRunner, h: PermanentHandle) -> bool {
    build_action_mask(&r.game, 0)[main_bit(h)] == 1.0
}

fn uses(r: &DebugRunner, h: PermanentHandle) -> u32 {
    r.game.players[0].battle_area[h.index as usize]
        .effect_activations
        .values()
        .map(|&n| n as u32)
        .sum()
}

fn bottom_source(r: &DebugRunner, h: PermanentHandle) -> String {
    let p = &r.game.players[0].battle_area[h.index as usize];
    p.card_sources[0].card_id(&r.game.card_data).to_string()
}

#[test]
fn ex2_007_main_not_offered_without_a_searcher() {
    let mut r = builder().start();
    let m = mother(&mut r);
    assert!(!offered(&r, m), "DCGO CanUseCondition: no [ADR-02 Searcher] to choose");
    assert!(!r.game.activate_field_main(0, m.index as usize), "not declarable");
    assert_eq!(uses(&r, m), 0, "no use counted");
}

#[test]
fn ex2_007_main_not_offered_for_a_searcher_in_the_trash_only() {
    let mut r = builder().start();
    let m = mother(&mut r);
    r.inject_trash(0, SEARCHER);
    assert!(!offered(&r, m), "the trash is not \"in play or your hand\"");
}

#[test]
fn ex2_007_main_offered_and_places_a_searcher_from_the_hand() {
    let mut r = builder().hand(0, &[SEARCHER]).start();
    let m = mother(&mut r);
    assert!(offered(&r, m), "an [ADR-02 Searcher] in the hand");
    r.game.decode_action(main_bit(m) as u16, 0);
    let pick = *r
        .pending_selection_view()
        .expect("Searcher pick")
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("the Searcher");
    r.execute_action(0, pick).expect("place it");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(bottom_source(&r, m), SEARCHER, "placed as the bottom digivolution card");
    assert_eq!(uses(&r, m), 1);
    assert!(!offered(&r, m), "[Once Per Turn]");
}

#[test]
fn ex2_007_main_offered_and_places_a_searcher_from_the_battle_area() {
    let mut r = builder().start();
    let m = mother(&mut r);
    r.place_on_field(0, SEARCHER, Some(0));
    assert!(offered(&r, m), "an [ADR-02 Searcher] in play");
    r.game.decode_action(main_bit(m) as u16, 0);
    let pick = *r
        .pending_selection_view()
        .expect("Searcher pick")
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("the Searcher");
    r.execute_action(0, pick).expect("place it");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(r.game.players[0].battle_area.len(), 1, "the Searcher left the battle area");
    assert_eq!(bottom_source(&r, m), SEARCHER);
}
