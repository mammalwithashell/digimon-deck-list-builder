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

// ═════════════════════════ ToyAgumon (EX12-008) ═════════════════════════

fn toy_runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX12-008")
        .expect("EX12-008 YAML loads")
        .add_card(me("PUPPET", CardColor::Red, 3, &["Puppet"]))
        .add_card(me("ME-ONLY", CardColor::Red, 3, &["ME"]))
        .add_card(me("PLAIN", CardColor::Red, 3, &["Beast"]))
        .add_card(me("DRAW", CardColor::Red, 3, &["Beast"]))
        .hand(0, hand)
        .deck(0, &["DRAW"; 4])
        .memory(2)
        .start()
}

#[test]
fn ex12_008_start_of_main_trashes_puppet_or_me_to_draw_and_gain_memory() {
    let mut r = toy_runner(&["PUPPET", "PLAIN"]);
    r.place_on_field(0, "EX12-008", Some(0));
    let memory_before = r.memory();
    r.game.enter_main_phase();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    select_hand_card(&mut r, 0, "PUPPET");
    r.auto_resolve().expect("resolve ToyAgumon Start of Main");
    assert!(trash_contains(&r, 0, "PUPPET"));
    assert!(hand_contains(&r, 0, "PLAIN"));
    assert_eq!(r.hand_size(0), 2, "PLAIN + 1 drawn");
    assert_eq!(r.deck_size(0), 3);
    assert_eq!(r.memory(), memory_before + 1);
}

#[test]
fn ex12_008_start_of_main_can_trash_an_me_card_and_can_be_declined() {
    let mut r = toy_runner(&["ME-ONLY"]);
    r.place_on_field(0, "EX12-008", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    r.execute_action(0, PASS).expect("decline");
    let _ = r.auto_resolve();
    assert!(hand_contains(&r, 0, "ME-ONLY"));
    assert_eq!(r.deck_size(0), 4);
}

#[test]
fn ex12_008_no_puppet_or_me_in_hand_does_nothing() {
    let mut r = toy_runner(&["PLAIN"]);
    r.place_on_field(0, "EX12-008", Some(0));
    r.game.enter_main_phase();
    assert_eq!(r.pending_kind(), None);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn ex12_008_digivolves_from_a_level_2_me_digimon_for_free() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-008")
        .expect("EX12-008 YAML loads")
        .add_card(me("BLUE-ME-2", CardColor::Blue, 2, &["ME"]))
        .add_card(me("BLUE-PLAIN-2", CardColor::Blue, 2, &["Beast"]))
        .hand(0, &["EX12-008", "EX12-008"])
        .memory(5)
        .start();
    assert_me_alt_path(&r, "EX12-008", 2, 0);
    let me_base = r.place_on_field(0, "BLUE-ME-2", Some(0));
    let plain_base = r.place_on_field(0, "BLUE-PLAIN-2", Some(1));
    assert_eq!(digivolve_cost(&mut r, "EX12-008", me_base.index as usize), Some(0));
    r.auto_resolve().ok();
    assert_eq!(digivolve_cost(&mut r, "EX12-008", plain_base.index as usize), None);
}

#[test]
fn ex12_008_inherited_grants_raid() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-008")
        .expect("EX12-008 YAML loads")
        .add_card(me("CARRIER", CardColor::Red, 4, &["Puppet"]))
        .start();
    let carrier = r.place_stack(0, &["EX12-008", "CARRIER"]);
    assert!(r.game.has_keyword(carrier, Keyword::Raid));
}

