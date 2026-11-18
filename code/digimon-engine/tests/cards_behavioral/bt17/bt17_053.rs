//! BT17-053 -- synthetic-board behavioural tests for the Keramon line
//! (dcgo-replay hunt). Fixtures: `keramon_line_support`.
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::keramon_line_support::*;
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, ModifierType};
use digimon_engine::permanent::PermanentHandle;

#[test]
fn bt17_053_opponents_turn_level5_play_digivolves_into_infermon_free_ignoring_requirements() {
    for seat in 0..=1u8 {
        let mut r = keramon_runner(seat);
        let keramon = r.place_on_field(seat, "BT17-053", Some(0));
        let infermon = r.add_to_hand(seat, "BT5-067");
        let opp = r.place_on_field(1 - seat, "OPP-LV5", None);
        r.fire_play_event_triggers(1 - seat, opp.index as usize, false, false);
        assert!(r.pending_is_optional(), "[Opponent's Turn] ... you may (seat {seat})");
        assert_eq!(r.pending_selection().unwrap().selecting_player, seat);
        r.accept_optional_trigger().unwrap();
        let _ = r.auto_resolve();
        assert_eq!(top_id(&r, keramon), "BT5-067", "digivolved into the [Infermon] in hand");
        // The digivolve itself draws 1 (the deck is INERT); the Infermon left the hand.
        assert_eq!(hand_ids(&r, seat), vec!["INERT"]);
        assert_eq!(r.memory(), 6, "without paying the cost");
    }
}

#[test]
fn bt17_053_level4_opponent_play_does_not_trigger() {
    let mut r = keramon_runner(0);
    let keramon = r.place_on_field(0, "BT17-053", Some(0));
    r.add_to_hand(0, "BT5-067");
    let opp = r.place_on_field(1, "OPP-LV4", None);
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(top_id(&r, keramon), "BT17-053");
}

#[test]
fn bt17_053_does_not_trigger_on_its_owners_own_turn() {
    let mut r = keramon_runner(0);
    r.game.turn_player_idx = 0;
    r.game.memory_pair = (0, 1);
    let keramon = r.place_on_field(0, "BT17-053", Some(0));
    r.add_to_hand(0, "BT5-067");
    let opp = r.place_on_field(1, "OPP-LV5", None);
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    assert!(r.pending_selection().is_none());
    assert_eq!(top_id(&r, keramon), "BT17-053");
}

#[test]
fn bt17_053_without_infermon_in_hand_does_nothing() {
    let mut r = keramon_runner(0);
    r.place_on_field(0, "BT17-053", Some(0));
    let opp = r.place_on_field(1, "OPP-LV5", None);
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt17_053_can_be_declined() {
    let mut r = keramon_runner(0);
    let keramon = r.place_on_field(0, "BT17-053", Some(0));
    r.add_to_hand(0, "BT5-067");
    let opp = r.place_on_field(1, "OPP-LV5", None);
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    r.decline_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, keramon), "BT17-053");
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt17_053_inherited_on_deletion_needs_the_unidentified_trait() {
    // Source Keramon under a host WITH [Unidentified] (BT22-053) -> token.
    let mut r = keramon_runner(0);
    let host = r.place_stack(0, &["BT17-053", "BT22-053"]);
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(token_on_field(&r, 0).len(), 1);

    // ... under a host WITHOUT it (plain Lv.3) -> nothing.
    let mut r = keramon_runner(0);
    let host = r.place_stack(0, &["BT17-053", "BLACK3"]);
    r.game.delete_permanent_with_effects(host);
    r.game.drain_effect_queue();
    let _ = r.auto_resolve();
    assert!(token_on_field(&r, 0).is_empty(), "host had no [Unidentified] trait");
}

// ----------------------------------------- BT17-053: opponent digivolution
#[test]
fn bt17_053_triggers_when_an_opponents_digimon_digivolves_into_level_5() {
    use digimon_engine::card_data::EvoCost;
    let mut up5 = lv("OPP-UP5", "Opp Up Five", 5);
    up5.evo_costs = vec![EvoCost { card_color: 0, level: 4, memory_cost: 1 }];
    let mut up4 = lv("OPP-UP4", "Opp Up Four", 4);
    up4.evo_costs = vec![EvoCost { card_color: 0, level: 3, memory_cost: 1 }];
    let r = base()
        .add_card(up5)
        .add_card(up4)
        .dsl_card("BT5-067")
        .unwrap()
        .dsl_card("BT17-053")
        .unwrap()
        .build();
    // seat 0 owns the Keramon; seat 1 has the turn.
    let mut r = into_main(r, 1, 6);
    let keramon = r.place_on_field(0, "BT17-053", Some(0));
    r.add_to_hand(0, "BT5-067");

    // A Lv.3 -> Lv.4 digivolution does not qualify ...
    let host3 = r.place_on_field(1, "INERT", Some(0));
    digivolve_via_action(&mut r, 1, "OPP-UP4", host3);
    assert!(r.pending_selection().is_none(), "level 4 result: no trigger");
    assert_eq!(top_id(&r, keramon), "BT17-053");

    // ... a Lv.4 -> Lv.5 one does.
    let host4 = r.place_on_field(1, "OPP-UP4", Some(0));
    digivolve_via_action(&mut r, 1, "OPP-UP5", host4);
    assert!(r.pending_is_optional(), "an opponent's Digimon digivolved into level 5");
    assert_eq!(r.pending_selection().unwrap().selecting_player, 0);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, keramon), "BT5-067");
}

#[test]
fn bt17_053_digivolve_into_infermon_does_not_need_the_normal_requirements_or_cost() {
    // BT5-067 needs Black Lv.4 normally; the Keramon is Lv.3. It is also free.
    let mut r = keramon_runner(0);
    let keramon = r.place_on_field(0, "BT17-053", Some(0));
    r.add_to_hand(0, "BT5-067");
    let opp = r.place_on_field(1, "OPP-LV5", None);
    let mem = r.memory();
    r.fire_play_event_triggers(1, opp.index as usize, false, false);
    r.accept_optional_trigger().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, keramon), "BT5-067");
    assert_eq!(r.memory(), mem);
}
