#![allow(unused_imports)]
use digimon_dsl::compiled::{CompiledAltPathKind, CompiledCost, CompiledPredicate};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, SEL_REVEAL_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, Keyword, PlaySource};
use digimon_engine::selection::SelectionKind;

use super::me_machine_puppet::*;
use super::support::{
    digimon, hand_contains, hand_ids, select_hand_card, trash_contains, DebugRunner,
};

// ═════════════════════════ Guardromon (EX12-054) ═════════════════════════

fn guardromon_runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-054")
        .expect("EX12-054 YAML loads")
        .add_card(me("MACH", CardColor::Black, 3, &["Machine"]))
        .add_card(me("CYB", CardColor::Black, 3, &["Cyborg"]))
        .add_card(me("ME-ONLY", CardColor::Black, 3, &["ME"]))
        .add_card(me("MUTANT", CardColor::Black, 3, &["Mutant"]))
        .add_card(me("PLAIN", CardColor::Black, 3, &["Beast"]))
        .add_card(me("DRAW", CardColor::Black, 3, &["Beast"]))
        .hand(0, hand)
        .deck(0, &["DRAW"; 6])
        .memory(10)
        .start()
}

#[test]
fn ex12_054_on_play_trashes_a_machine_cyborg_or_me_card_to_draw_2() {
    let mut r = guardromon_runner(&["EX12-054", "CYB", "PLAIN", "MUTANT"]);
    let slot = hand_slot(&r, "EX12-054");
    assert!(r.play(0, slot).is_some());
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    // [Machine]/[Cyborg]/[ME] only: [Mutant] and plain cards are not offered.
    let view = r.pending_selection_view().unwrap();
    let offered: usize = view.valid_action_ids.iter().filter(|&&a| a != PASS).count();
    assert_eq!(offered, 1, "only the [Cyborg] card qualifies");
    select_hand_card(&mut r, 0, "CYB");
    r.auto_resolve().expect("resolve Guardromon On Play");
    assert!(trash_contains(&r, 0, "CYB"));
    assert_eq!(r.hand_size(0), 2 + 2, "PLAIN + MUTANT kept, drew 2");
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex12_054_declining_the_trash_draws_nothing() {
    let mut r = guardromon_runner(&["EX12-054", "ME-ONLY"]);
    let slot = hand_slot(&r, "EX12-054");
    assert!(r.play(0, slot).is_some());
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    r.execute_action(0, PASS).expect("decline the optional trash");
    let _ = r.auto_resolve();
    assert!(hand_contains(&r, 0, "ME-ONLY"));
    assert_eq!(r.deck_size(0), 6, "no draw without the trash");
}

#[test]
fn ex12_054_no_qualifying_hand_card_skips_the_effect() {
    let mut r = guardromon_runner(&["EX12-054", "PLAIN", "MUTANT"]);
    let slot = hand_slot(&r, "EX12-054");
    assert!(r.play(0, slot).is_some());
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 2);
    assert_eq!(r.deck_size(0), 6);
}

#[test]
fn ex12_054_when_digivolving_trigger_and_me_alt_path() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-054")
        .expect("EX12-054 YAML loads")
        .add_card(me("RED-ME-3", CardColor::Red, 3, &["ME"]))
        .add_card(me("MACH", CardColor::Black, 3, &["Machine"]))
        .add_card(me("DRAW", CardColor::Black, 3, &["Beast"]))
        .hand(0, &["EX12-054", "MACH"])
        .deck(0, &["DRAW"; 4])
        .memory(5)
        .start();
    assert_me_alt_path(&r, "EX12-054", 3, 2);
    let base = r.place_on_field(0, "RED-ME-3", Some(0));
    assert_eq!(digivolve_cost(&mut r, "EX12-054", base.index as usize), Some(2));
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand), "[When Digivolving] cost pick");
    select_hand_card(&mut r, 0, "MACH");
    r.auto_resolve().expect("resolve Guardromon When Digivolving");
    assert!(trash_contains(&r, 0, "MACH"));
    assert!(r.deck_size(0) <= 2, "<Draw 2> drew after the trash (plus the digivolution draw)");
}

#[test]
fn ex12_054_has_blocker() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-054")
        .expect("EX12-054 YAML loads")
        .start();
    let g = r.place_on_field(0, "EX12-054", Some(0));
    assert!(r.game.has_keyword(g, Keyword::Blocker));
}

