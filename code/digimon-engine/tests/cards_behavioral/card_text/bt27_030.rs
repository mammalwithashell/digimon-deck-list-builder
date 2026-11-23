//! Source: `data/card_bundles/BT27-030.md` (printed text and official Q&A).
//!
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] You may return
//! 1 of your opponent's level 4 or lower Digimon to the hand. Then, by trashing
//! 2 bottom face-down cards from under any of your Tamers, you may play or use
//! 1 [DATA SQUAD] trait card with a play or use cost of 5 or less from your
//! hand without paying the cost.
//! Inherited: [All Turns] [Once Per Turn] When this Digimon with [Gaogamon] in
//! its name or the [DATA SQUAD] trait would leave the battle area, by trashing
//! the bottom face-down card from under any of your Tamers, it doesn't leave.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-030";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(tamer("T", "Tamer", &["DATA SQUAD"]))
        .add_card(mon("SQ5", "Squad", 3, 5, &["DATA SQUAD"]))
        .add_card(mon("SQ6", "Squad6", 3, 6, &["DATA SQUAD"]))
        .add_card(mon("PLAIN", "Plain", 3, 3, &[]))
        .add_card(mon("L4", "Four", 4, 5, &[]))
        .add_card(mon("L5", "Five", 5, 7, &[]))
        .add_card(mon("GAO", "Gaogamon", 4, 5, &[]))
        .add_card(mon("SQUADMON", "Squadmon", 4, 5, &["DATA SQUAD"]))
        .add_card(mon("FD", "Fd", 3, 3, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .hand(0, hand)
        .memory(8)
        .start()
}

fn pay(r: &mut DebugRunner) {
    choose_branch(r, 0);
    pay_tamer_stock(r);
}

#[test]
fn on_play_bounces_a_level_four_or_lower_then_pays_two_face_down_cards_to_play_data_squad_free() {
    let mut r = runner(&[CARD, "SQ5", "SQ6", "PLAIN"]);
    let t = stocked(&mut r, "T", &["FD", "FD", "FD"]);
    let l4 = r.place_on_field(1, "L4", Some(0));
    r.place_on_field(1, "L5", Some(1));
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("optional bounce");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["L4"], "level 5 is not a target");
    r.choose_prompt_card("L4", None).unwrap();
    assert!(hand_ids(&r, 1).contains(&"L4".to_string()));
    let _ = l4;
    pay(&mut r);
    assert_eq!(source_ids(&r, t).len(), 1, "2 bottom face-down cards trashed");
    let v = r.pending_selection_view().expect("optional free play");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["SQ5"], "cost 6 and non-DATA SQUAD excluded");
    let mem = r.memory();
    r.choose_prompt_card("SQ5", None).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"SQ5".to_string()));
    assert_eq!(r.memory(), mem, "played without paying the cost");
}

#[test]
fn the_then_cost_is_independent_of_the_bounce_and_may_be_declined() {
    let mut r = runner(&[CARD, "SQ5"]);
    let t = stocked(&mut r, "T", &["FD", "FD"]);
    r.place_on_field(1, "L4", Some(0));
    play_hand(&mut r, CARD);
    r.decline_prompt().unwrap();
    assert_eq!(field_ids(&r, 1), vec!["L4"]);
    let v = r.pending_selection_view().expect("cost choice still offered");
    assert_eq!(v.effect_choices.as_ref().map(|c| c.len()), Some(2));
    choose_branch(&mut r, 1);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t).len(), 2, "nothing trashed");
    assert_eq!(hand_ids(&r, 0), vec!["SQ5"]);
}

#[test]
fn the_cost_may_be_paid_and_the_play_declined() {
    let mut r = runner(&[CARD, "SQ5"]);
    let t = stocked(&mut r, "T", &["FD", "FD"]);
    play_hand(&mut r, CARD);
    pay(&mut r);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert!(source_ids(&r, t).is_empty());
    assert_eq!(hand_ids(&r, 0), vec!["SQ5"]);
}

#[test]
fn fewer_than_two_face_down_cards_offers_no_cost() {
    let mut r = runner(&[CARD, "SQ5"]);
    stocked(&mut r, "T", &["FD"]);
    play_hand(&mut r, CARD);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert_eq!(hand_ids(&r, 0), vec!["SQ5"]);
}

#[test]
fn when_attacking_fires_once_per_turn() {
    let mut r = runner(&[]);
    let me = r.place_on_field(0, CARD, Some(0)).index;
    r.place_on_field(1, "L4", Some(0));
    attack_security(&mut r, me);
    let v = r.pending_selection_view().expect("When Attacking bounce");
    assert!(v.is_optional);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    r.game.players[0].battle_area[me as usize].is_suspended = false;
    attack_security(&mut r, me);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none(), "Once Per Turn");
}

fn carrier(r: &mut DebugRunner, top: &str) -> PermanentHandle {
    r.place_stack(0, &[CARD, top])
}

#[test]
fn inherited_keeps_a_gaogamon_named_carrier_by_trashing_one_face_down_card() {
    let mut r = runner(&[]);
    let t = stocked(&mut r, "T", &["FD", "FD"]);
    let me = carrier(&mut r, "GAO");
    let opp = r.place_on_field(1, "L4", Some(0));
    effect_bounce(&mut r, 1, opp, me);
    let v = r.pending_selection_view().expect("optional replacement");
    assert!(v.is_optional);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    pay_tamer_stock(&mut r);
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, me), "GAO");
    assert_eq!(source_ids(&r, t).len(), 1);
}

#[test]
fn inherited_covers_a_data_squad_carrier_and_is_once_per_turn() {
    let mut r = runner(&[]);
    stocked(&mut r, "T", &["FD", "FD"]);
    let me = carrier(&mut r, "SQUADMON");
    let opp = r.place_on_field(1, "L4", Some(0));
    effect_bounce(&mut r, 1, opp, me);
    let v = r.pending_selection_view().expect("optional replacement");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    pay_tamer_stock(&mut r);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"SQUADMON".to_string()));
    effect_bounce(&mut r, 1, opp, me);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"SQUADMON".to_string()), "second leave this turn");
}

#[test]
fn inherited_ignores_other_carriers_and_needs_a_face_down_card() {
    let mut r = runner(&[]);
    stocked(&mut r, "T", &["FD"]);
    let me = carrier(&mut r, "L4");
    let opp = r.place_on_field(1, "L4", Some(0));
    effect_bounce(&mut r, 1, opp, me);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"L4".to_string()));

    let mut r = runner(&[]);
    stocked(&mut r, "T", &[]);
    let me = carrier(&mut r, "GAO");
    let opp = r.place_on_field(1, "L4", Some(0));
    effect_bounce(&mut r, 1, opp, me);
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 0).contains(&"GAO".to_string()));
}
