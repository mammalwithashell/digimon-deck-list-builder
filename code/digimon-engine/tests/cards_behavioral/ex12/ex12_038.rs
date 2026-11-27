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

// ═════════════════════════ Kokuwamon (EX12-038) ═════════════════════════

fn kokuwamon_runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-038")
        .expect("EX12-038 YAML loads")
        .add_card(me("MUTANT", CardColor::Yellow, 3, &["Mutant"]))
        .add_card(me("ME-ONLY", CardColor::Yellow, 3, &["ME"]))
        .add_card(me("MACH", CardColor::Yellow, 3, &["Machine"]))
        .add_card(me("DRAW", CardColor::Yellow, 3, &["Beast"]))
        .add_card(me("OPP", CardColor::Red, 4, &["Beast"]))
        .hand(0, hand)
        .deck(0, &["DRAW"; 6])
        .deck(1, &["DRAW"; 6])
        .security(1, &["DRAW"; 3])
        .memory(10)
        .start()
}

#[test]
fn ex12_038_on_play_trashes_a_mutant_or_me_card_to_draw_2() {
    let mut r = kokuwamon_runner(&["EX12-038", "MUTANT", "MACH"]);
    let slot = hand_slot(&r, "EX12-038");
    assert!(r.play(0, slot).is_some());
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    let view = r.pending_selection_view().unwrap();
    assert_eq!(
        view.valid_action_ids.iter().filter(|&&a| a != PASS).count(),
        1,
        "[Machine] is not [Mutant]/[ME]: only MUTANT qualifies"
    );
    select_hand_card(&mut r, 0, "MUTANT");
    r.auto_resolve().expect("resolve Kokuwamon On Play");
    assert!(trash_contains(&r, 0, "MUTANT"));
    assert_eq!(r.hand_size(0), 1 + 2, "MACH kept + 2 drawn");
}

#[test]
fn ex12_038_on_play_declined_or_without_a_match_draws_nothing() {
    let mut r = kokuwamon_runner(&["EX12-038", "ME-ONLY"]);
    let slot = hand_slot(&r, "EX12-038");
    assert!(r.play(0, slot).is_some());
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), 6);

    let mut r = kokuwamon_runner(&["EX12-038", "MACH"]);
    let slot = hand_slot(&r, "EX12-038");
    assert!(r.play(0, slot).is_some());
    let _ = r.auto_resolve();
    assert_eq!(r.deck_size(0), 6, "no [Mutant]/[ME] card in hand");
}

#[test]
fn ex12_038_inherited_when_attacking_gives_minus_2000_dp_for_the_turn() {
    let mut r = kokuwamon_runner(&[]);
    let carrier = r.place_stack(0, &["EX12-038", "MACH"]);
    let victim = r.place_on_field(1, "OPP", Some(0));
    let before = r.effective_dp(victim).unwrap();
    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(victim), Some(before - 2000));
}

