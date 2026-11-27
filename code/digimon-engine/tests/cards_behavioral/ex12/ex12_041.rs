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

// ═════════════════════════ Thundermon (EX12-041) ═════════════════════════

fn thundermon_runner(hand: &[&str]) -> DebugRunner {
    let mut opt = me("ME-OPTION", CardColor::Black, 3, &["ME"]);
    opt.card_kind = digimon_engine::enums::CardKind::Option;
    opt.level = None;
    opt.dp = None;
    opt.play_cost = 3;
    DebugRunner::builder()
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("ME-4", CardColor::Black, 4, &["ME"]))
        .add_card(me("MUTANT-3", CardColor::Black, 3, &["Mutant"]))
        .add_card(me("PLAIN-3", CardColor::Black, 3, &["Beast"]))
        .add_card(opt)
        .hand(0, hand)
        .memory(10)
        .start()
}

#[test]
fn ex12_041_main_plays_a_mutant_or_me_card_for_2_less() {
    let mut r = thundermon_runner(&["ME-4", "PLAIN-3"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    let field_before = r.battle_area_size(0);
    let memory_before = r.memory();
    assert!(r.game.activate_field_main(0, t.index as usize));
    select_hand_card(&mut r, 0, "ME-4");
    r.auto_resolve().expect("resolve Thundermon Main");
    assert_eq!(r.battle_area_size(0), field_before + 1);
    assert_eq!(memory_before - r.memory(), 3, "play cost 5 reduced by 2");
}

#[test]
fn ex12_041_main_never_offers_a_card_without_the_mutant_or_me_trait() {
    let mut r = thundermon_runner(&["PLAIN-3"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    assert!(
        !r.game.activate_field_main(0, t.index as usize),
        "no [Mutant]/[ME] card in hand: the [Main] is not usable"
    );
}

fn thundermon_main_action(field_index: u8) -> usize {
    (FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN)
        as usize
}

/// Once-per-turn uses the Thundermon at `field_index` has recorded this turn
/// (a refunded use leaves no entry).
fn thundermon_uses(r: &DebugRunner, field_index: u8) -> u32 {
    r.game.players[0].battle_area[field_index as usize]
        .effect_activations
        .values()
        .map(|&n| n as u32)
        .sum()
}

/// Thundermon (EX12-041) "[Main] [Once Per Turn] You may play or use 1 card
/// ...": declared, then the pick passed while a [Mutant] card was selectable.
/// Owner ruling 2026-11-24 "Match dcgo on the main use": DCGO EX12_041.cs:180
/// `if (!isUsed) activateClass.RemoveUse();` (isUsed is set only when a card
/// is picked, :96) gives the use back. This reverses the 2026-10-25 record
/// and commit 4606e6b4d, which spent it under CR 4.3 15-14-1-4 /
/// 15-8-4-1/-2; the owner chose DCGO parity over that reading. Nothing else
/// changed, so the main-declaration loop guard withholds the identical
/// declaration. A second Thundermon has its own use (CR 4.3 15-14-1-3).
#[test]
fn ex12_041_main_declared_then_pick_passed_keeps_the_once_per_turn() {
    let mut r = thundermon_runner(&["MUTANT-3"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    let other = r.place_on_field(0, "EX12-041", Some(0));
    let memory_before = r.memory();
    assert_eq!(build_action_mask(&r.game, 0)[thundermon_main_action(t.index)], 1.0);
    assert!(r.game.activate_field_main(0, t.index as usize));
    assert_eq!(thundermon_uses(&r, t.index), 1, "the declaration records the use");
    r.execute_action(0, PASS).expect("pass the pick");
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none(), "the effect ended");
    assert_eq!(hand_ids(&r, 0), vec!["MUTANT-3"], "nothing was played");
    assert_eq!(r.memory(), memory_before, "no cost was paid");
    assert_eq!(thundermon_uses(&r, t.index), 0, "DCGO RemoveUse: the passed pick keeps the use");
    assert_eq!(
        build_action_mask(&r.game, 0)[thundermon_main_action(t.index)],
        0.0,
        "loop guard: the identical declaration is withheld while nothing changed"
    );
    assert_eq!(
        build_action_mask(&r.game, 0)[thundermon_main_action(other.index)],
        1.0,
        "CR 4.3 15-14-1-3: the other Thundermon still has its own use"
    );
}

/// The kept use is real: after a state change the [Main] is offered again,
/// a card is played at cost -2, and that spends the use for the turn.
#[test]
fn ex12_041_main_pass_then_state_change_redeclares_and_spends() {
    let mut r = thundermon_runner(&["MUTANT-3"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    let bit = thundermon_main_action(t.index);
    assert!(r.game.activate_field_main(0, t.index as usize));
    r.execute_action(0, PASS).expect("pass the pick");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert_eq!(build_action_mask(&r.game, 0)[bit], 0.0);
    r.game.memory -= 1;
    assert_eq!(build_action_mask(&r.game, 0)[bit], 1.0, "the state changed: offered again");
    let memory_before = r.memory();
    r.game.decode_action(bit as u16, 0);
    select_hand_card(&mut r, 0, "MUTANT-3");
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(!hand_contains(&r, 0, "MUTANT-3"), "the picked card was played");
    assert_eq!(memory_before - r.memory(), 1, "play cost 3 reduced by 2");
    assert_eq!(thundermon_uses(&r, t.index), 1);
    r.game.memory -= 1;
    assert_eq!(build_action_mask(&r.game, 0)[bit], 0.0, "spent for the turn");
}

/// Thundermon (EX12-041): declaring the [Main] and playing a card spends the
/// [Once Per Turn] (DCGO EX12_041.cs:96 sets isUsed before the play; CR 4.3
/// 15-14-1-1/-4).
#[test]
fn ex12_041_main_pick_chosen_spends_the_once_per_turn() {
    let mut r = thundermon_runner(&["MUTANT-3", "ME-4"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    assert!(r.game.activate_field_main(0, t.index as usize));
    select_hand_card(&mut r, 0, "MUTANT-3");
    r.auto_resolve().expect("resolve Thundermon Main");
    assert!(!hand_contains(&r, 0, "MUTANT-3"), "the picked card was played");
    assert_eq!(thundermon_uses(&r, t.index), 1);
    assert_eq!(
        build_action_mask(&r.game, 0)[thundermon_main_action(t.index)],
        0.0,
        "a card was played: the [Once Per Turn] is used although ME-4 is still playable"
    );
    assert!(!r.game.activate_field_main(0, t.index as usize));
}

#[test]
fn ex12_041_main_can_use_an_me_option_for_2_less() {
    let mut r = thundermon_runner(&["ME-OPTION"]);
    let t = r.place_on_field(0, "EX12-041", Some(0));
    let memory_before = r.memory();
    assert!(r.game.activate_field_main(0, t.index as usize));
    select_hand_card(&mut r, 0, "ME-OPTION");
    let _ = r.auto_resolve();
    assert_eq!(memory_before - r.memory(), 1, "use cost 3 reduced by 2");
}

#[test]
fn ex12_041_digivolves_from_a_level_3_me_digimon_for_2() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("RED-ME-3", CardColor::Red, 3, &["ME"]))
        .add_card(me("RED-PLAIN-3", CardColor::Red, 3, &["Beast"]))
        .hand(0, &["EX12-041", "EX12-041"])
        .memory(5)
        .start();
    assert_me_alt_path(&r, "EX12-041", 3, 2);
    let me_base = r.place_on_field(0, "RED-ME-3", Some(0));
    let plain_base = r.place_on_field(0, "RED-PLAIN-3", Some(1));
    assert_eq!(digivolve_cost(&mut r, "EX12-041", me_base.index as usize), Some(2));
    r.auto_resolve().ok();
    assert_eq!(digivolve_cost(&mut r, "EX12-041", plain_base.index as usize), None);
}

#[test]
fn ex12_041_inherited_when_attacking_gives_minus_2000_dp_for_the_turn() {
    let mut r = DebugRunner::builder()
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("CARRIER", CardColor::Yellow, 4, &["Mutant"]))
        .add_card(me("OPP", CardColor::Red, 4, &["Beast"]))
        .deck(0, &["OPP"; 4])
        .deck(1, &["OPP"; 4])
        .security(1, &["OPP"; 3])
        .start();
    let carrier = r.place_stack(0, &["EX12-041", "CARRIER"]);
    let victim = r.place_on_field(1, "OPP", Some(0));
    let before = r.effective_dp(victim).unwrap();
    r.attack_player(carrier, 1, false);
    let _ = r.auto_resolve();
    assert_eq!(r.effective_dp(victim), Some(before - 2000));
}

#[test]
fn ex12_041_rule_name_includes_mamemon_and_is_found_by_the_kokuwamon_reveal() {
    // Kokuwamon (EX13-046) [On Play] adds "1 card with [Mamemon] in its text":
    // Thundermon (EX12-041) prints `(Rule) Name: Treated as including
    // [Mamemon]` and must be selectable in that bucket.
    let deck = deck_with_top(&["EX12-041", "PLAIN", "FILL"], "FILL", 4);
    let mut r = DebugRunner::builder()
        .dsl_card("EX13-046")
        .expect("EX13-046 YAML loads")
        .dsl_card("EX12-041")
        .expect("EX12-041 YAML loads")
        .add_card(me("PLAIN", CardColor::Black, 4, &["Beast"]))
        .add_card(me("FILL", CardColor::Black, 3, &["Beast"]))
        .hand(0, &["EX13-046"])
        .deck(0, &deck)
        .deck(1, &["FILL"; 6])
        .memory(5)
        .start();
    let slot = hand_slot(&r, "EX13-046");
    assert!(r.play(0, slot).is_some());
    assert_eq!(offered_reveal_ids(&r), vec!["EX12-041".to_string()]);
    pick_revealed(&mut r, "EX12-041");
    let _ = r.auto_resolve();
    assert!(hand_contains(&r, 0, "EX12-041"));
}
