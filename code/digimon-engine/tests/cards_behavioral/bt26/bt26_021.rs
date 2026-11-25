//! BT26-021 Gekomon — Lv.4 Blue/Purple Digimon (Amphibian / Titan / TS).
//!
//! [On Play] [When Digivolving] 1 of your [TS] trait Digimon's attack target
//! can't change for the turn.
//! [Main] [Once Per Turn] You may play 1 [TS] trait Tamer card from your trash
//! with the cost reduced by 2.
//! Inherited: [All Turns] [Once Per Turn] When a Digimon attacks, by trashing
//! 1 card in your hand, trash the bottom 2 digivolution cards of 1 of your
//! opponent's Digimon.
//!
//! DCGO: BT26/Blue/BT26_021.cs.

use super::support::*;
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, ModifierType};

const CARD_ID: &str = "BT26-021";

fn setup() -> DebugRunner {
    let mut ts_tamer = tamer("TS-T", "TS Tamer", CardColor::Purple, &["TS"]);
    ts_tamer.play_cost = 5;
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-021")
        .add_card(filler("FILLER"))
        .add_card(ts_tamer)
        .add_card(tamer("OTHER-T", "Other Tamer", CardColor::Purple, &[]))
        .add_card(digimon("TS-D", "TS Digi", CardColor::Blue, 4, 5, &["TS"]))
        .add_card(digimon("NON-TS", "Plain", CardColor::Blue, 4, 5, &[]))
        .add_card(digimon("OPP", "Opp", CardColor::Red, 4, 5, &[]))
        .add_card(digimon("CARRIER", "Carrier", CardColor::Blue, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

fn main_bit(field_index: usize) -> u16 {
    FIELD_EFFECT_START + field_index as u16 * EFFECTS_PER_PERMANENT + FIELD_EFFECT_SLOT_FOR_MAIN
}

fn main_legal(r: &DebugRunner) -> bool {
    build_action_mask(&r.game, 0)[main_bit(0) as usize] > 0.0
}

#[test]
fn bt26_021_traits_include_rule_titan() {
    let r = setup();
    let c = r.compiled_card(CARD_ID).unwrap();
    for t in ["Amphibian", "Titan", "TS"] {
        assert!(c.traits.contains(&t.to_string()), "{t}");
    }
}

#[test]
fn bt26_021_on_play_locks_own_ts_attack_target() {
    let mut r = setup();
    let g = r.place_on_field(0, CARD_ID, Some(0));
    let plain = r.place_on_field(0, "NON-TS", Some(0));
    let ts = r.place_on_field(0, "TS-D", Some(0));
    fire(&mut r, EffectTiming::OnPlay, g);
    // Mandatory pick over own [TS] Digimon only: Gekomon itself + TS-D.
    let v = r.pending_selection_view().expect("own [TS] pick");
    assert!(!v.is_optional);
    let picks = v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != digimon_engine::action::space::PASS)
        .count();
    assert_eq!(picks, 2, "only [TS] Digimon");
    // Pick the TS-D Digimon (last candidate).
    let a = *v
        .valid_action_ids
        .iter()
        .filter(|&&a| a != digimon_engine::action::space::PASS)
        .last()
        .unwrap();
    r.execute_action(0, a).unwrap();
    let _ = r.auto_resolve();
    let locked: Vec<_> = [g, plain, ts]
        .into_iter()
        .filter(|h| {
            r.modifiers()
                .has(*h, ModifierType::CannotSwitchAttackTarget)
        })
        .collect();
    assert_eq!(locked.len(), 1);
    assert_ne!(locked[0], plain);
}

#[test]
fn bt26_021_main_plays_ts_tamer_from_trash_reduced_once_per_turn() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "TS-T");
    push_trash(&mut r, 0, "OTHER-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    let mem = r.memory();
    assert!(main_legal(&r), "[Main] offered");
    r.game.decode_action(main_bit(0), 0);
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional, "may");
    assert_eq!(
        v.valid_action_ids
            .iter()
            .filter(|&&a| a != digimon_engine::action::space::PASS)
            .count(),
        2,
        "only [TS] Tamers"
    );
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"TS-T".to_string()));
    assert_eq!(r.memory(), mem - 3, "cost 5 reduced by 2");
    // Once per turn: a chosen Tamer spends the use (DCGO BT26_021.cs:138
    // sets isUsed before the play), so the [Main] is no longer offered.
    assert_eq!(uses_spent(&r, 0), 1);
    assert!(!main_legal(&r), "[Main] already used this turn");
    assert_eq!(
        trash_ids(&r, 0).iter().filter(|c| *c == "TS-T").count(),
        1,
        "second TS Tamer stays in trash"
    );
}

#[test]
fn bt26_021_main_needs_ts_tamer_in_trash() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "OTHER-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    assert!(!main_legal(&r), "no [TS] Tamer in trash → not activatable");
    assert!(field_ids(&r, 0).len() == 1);
}

/// The carrier's OWN attack as the attack declaration dispatches it: the
/// printed "When a Digimon attacks" has no "another", so the carrier itself
/// is an observer of the `OnAllyAttack` batch (DCGO `AttackProcess.cs` stacks
/// one `EffectTiming.OnAllyAttack` batch with no attacker exclusion).
fn carrier_attacks(r: &mut DebugRunner, carrier: digimon_engine::permanent::PermanentHandle) {
    let card = r.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .handle();
    r.game.enqueue_triggered(
        EffectTiming::OnAllyAttack,
        digimon_engine::selection::TriggerSource::PlayerBattleAreaAttack {
            player: 0,
            attacker: carrier,
            card,
        },
    );
    r.game.drain_effect_queue();
}

/// Through the REAL attack declaration the carrier's own attack offers the
/// inherited effect exactly ONCE (the clause is not also enqueued through a
/// carrier-scoped `OnAttack` entry, which would double the prompt).
#[test]
fn bt26_021_real_own_attack_offers_the_inherited_effect_once() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = opp_with_sources(&mut r);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    r.attack_digimon(carrier, opp, false);
    assert!(r.pending_is_optional(), "inherited trigger offered on own attack");
    r.decline_optional_trigger().unwrap();
    assert!(
        r.pending_selection_view().is_none(),
        "one entry only: declining leaves no second offer"
    );
}

fn opp_with_sources(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    r.place_stack(1, &["FILLER", "FILLER", "FILLER", "OPP"])
}

#[test]
fn bt26_021_inherited_any_attack_trash_hand_strips_bottom_two() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = opp_with_sources(&mut r);
    push_hand(&mut r, 0, "FILLER");
    // The OPPONENT's Digimon attacks → observed via on_opponent_attack.
    r.game.enqueue_triggered(
        EffectTiming::OnOpponentAttack,
        digimon_engine::selection::TriggerSource::PlayerBattleAreaAttack {
            player: 0,
            attacker: opp,
            card: r.game.players[1].battle_area[opp.index as usize]
                .top_card()
                .handle(),
        },
    );
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional(), "by trashing — optional cost");
    r.accept_optional_trigger().unwrap();
    drain_first(&mut r);
    assert_eq!(r.hand_size(0), 0, "hand card trashed as the cost");
    assert_eq!(sources(&r, opp), 1, "bottom 2 of 3 sources trashed");
    let _ = carrier;
}

#[test]
fn bt26_021_inherited_once_per_turn_shared_across_attackers() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = opp_with_sources(&mut r);
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    // The carrier itself attacks.
    carrier_attacks(&mut r, carrier);
    r.accept_optional_trigger().unwrap();
    drain_first(&mut r);
    assert_eq!(sources(&r, opp), 1);
    // A second attack (by anything) the same turn: no trigger.
    carrier_attacks(&mut r, carrier);
    assert!(r.pending_selection_view().is_none(), "once per turn");
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_021_inherited_decline_keeps_hand_and_use() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let opp = opp_with_sources(&mut r);
    push_hand(&mut r, 0, "FILLER");
    carrier_attacks(&mut r, carrier);
    r.decline_optional_trigger().unwrap();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(sources(&r, opp), 3);
    carrier_attacks(&mut r, carrier);
    assert!(
        r.pending_selection_view().is_some(),
        "not consumed by a decline"
    );
}

#[test]
fn bt26_021_inherited_needs_a_hand_card() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let _opp = opp_with_sources(&mut r);
    carrier_attacks(&mut r, carrier);
    assert!(r.pending_selection_view().is_none());
}

/// Once-per-turn uses the Gekomon at `field_index` has recorded this turn
/// (a refunded use leaves no entry).
fn uses_spent(r: &DebugRunner, field_index: usize) -> u32 {
    r.game.players[0].battle_area[field_index]
        .effect_activations
        .values()
        .map(|&n| n as u32)
        .sum()
}

/// Declared, then the [TS] Tamer pick passed: the use is given back. Owner
/// ruling 2026-11-24 "Match dcgo on the main use": DCGO BT26_021.cs:184
/// `if (!isUsed) activateClass.RemoveUse();` (isUsed is set only when a card
/// is chosen, :136-138). The owner chose DCGO parity over the CR 4.3
/// 15-14-1-4 / 15-8-4-1/-2 reading. Nothing else changed, so the
/// main-declaration loop guard withholds the identical declaration.
#[test]
fn bt26_021_main_pass_refunds_once_per_turn() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-T");
    r.game.enter_main_phase();
    let _ = r.auto_resolve();
    let memory = r.memory();
    let hand = hand_ids(&r, 0);
    let trash = trash_ids(&r, 0);
    r.game.decode_action(main_bit(0), 0);
    assert_eq!(uses_spent(&r, 0), 1, "the declaration records the use");
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert_eq!(field_ids(&r, 0).len(), 2, "nothing played");
    assert_eq!(trash_ids(&r, 0), trash, "the [TS] Tamer stays in trash");
    assert_eq!(hand_ids(&r, 0), hand);
    assert_eq!(r.memory(), memory);
    assert_eq!(uses_spent(&r, 0), 0, "DCGO RemoveUse: the passed pick keeps the use");
    assert!(
        !main_legal(&r),
        "loop guard: the identical declaration is withheld while nothing changed"
    );
    assert!(
        build_action_mask(&r.game, 0)[main_bit(1) as usize] > 0.0,
        "another copy retains its independent use"
    );
}

/// The kept use is real: after a state change the [Main] is offered again,
/// the Tamer is played at cost -2, and that spends the use for the turn.
#[test]
fn bt26_021_main_pass_then_state_change_redeclares_and_spends() {
    let mut r = setup();
    r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "TS-T");
    r.game.enter_main_phase();
    assert!(r.pending_selection().is_none(), "no decision is left");
    r.game.decode_action(main_bit(0), 0);
    pass(&mut r, 0);
    assert!(!main_legal(&r));
    push_hand(&mut r, 0, "FILLER");
    assert!(main_legal(&r), "the state changed: the kept use is offered again");
    let memory = r.memory();
    r.game.decode_action(main_bit(0), 0);
    let tamer_pick = r.pending_selection_view().expect("the [TS] Tamer pick");
    r.execute_action(0, tamer_pick.valid_action_ids[0]).unwrap();
    assert!(r.pending_selection().is_none(), "no decision is left");
    assert!(field_ids(&r, 0).contains(&"TS-T".to_string()));
    assert_eq!(r.memory(), memory - 3, "cost 5 reduced by 2");
    assert_eq!(uses_spent(&r, 0), 1);
    push_trash(&mut r, 0, "TS-T");
    assert!(!main_legal(&r), "spent for the turn, whatever else changes");
}
