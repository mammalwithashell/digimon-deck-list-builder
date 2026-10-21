//! Chapter 15-14 "Effect Icons" (Comprehensive Rules Ver. 4.3): the
//! [X Per Turn] count (§15-14-1-1, §15-14-1-4).
//!
//! §15-14-1-4: "When a player chooses to activate an [X Per Turn] effect, it
//! will count toward 1 use of [X Per Turn], even if the effect's processing
//! can't be executed." Official Q&A Q1818 (BT9-018, "You may delete that
//! Digimon"): "If you choose not to, the effect's [Once Per Turn] requirement
//! won't be used up, either."
//!
//! Every test asserts the RULE, never the engine's current output.
//!
//! Fixture cards: `fixtures/common/*.yaml` + `fixtures/ch15_14/*.yaml`.
//! `SEC_MAY_DEL`'s whole effect is one "you may", folded into its pick (no
//! outer confirm). `SEC_COST_DEL` is a triggered effect with an optional
//! processing condition (a cost): declining the cost of an activated effect
//! still spends the use (owner ruling #20; BT24-026 / P-250).

use crate::fixtures;
use digimon_engine::action::space::PASS;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;
use digimon_engine::trigger_context::EventCause;

/// P0 has `card` on the field and `hand0` in hand; P1 has two plain Digimon.
fn board(card: &str, hand0: &[&str]) -> DebugRunner {
    let mut r = fixtures::builder(&[fixtures::ch15_14_cards()])
        .hand(0, hand0)
        .deck(0, &["VANILLA_LV3"; 6])
        .deck(1, &["VANILLA_LV3"; 6])
        .memory(5)
        .start();
    r.place_on_field(0, card, Some(0));
    r.place_on_field(1, "VANILLA_LV3", Some(0));
    r.place_on_field(1, "VANILLA_LV3", Some(0));
    r
}

/// P0's security stack is removed from (the trigger event).
fn remove_p0_security(r: &mut DebugRunner) {
    r.game.enqueue_triggered(
        EffectTiming::OnOwnSecurityRemoved,
        TriggerSource::SecurityRemoved {
            affected_player: 0,
            observer_player: 0,
            source_player: 1,
            card: CardHandle(0),
            cause: EventCause::SecurityRemoval,
        },
    );
    r.game.drain_effect_queue();
}

/// Take the pending pick's first non-PASS action (P0's).
fn accept(r: &mut DebugRunner) {
    let sel = r.pending_selection().expect("a pick is offered");
    assert_eq!(sel.selecting_player, 0, "{sel:?}");
    let act = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("a non-PASS choice");
    r.execute_action(0, act).expect("accept");
    r.game.drain_effect_queue();
}

/// PASS the pending (optional) pick.
fn pass(r: &mut DebugRunner) {
    let sel = r.pending_selection().expect("a pick is offered");
    assert!(sel.is_optional, "PASS is legal on an optional pick: {sel:?}");
    r.execute_action(sel.selecting_player, PASS).expect("pass");
    r.game.drain_effect_queue();
}

/// Declining a "you may" effect is choosing not to activate it, so its
/// [Once Per Turn] is not used up and the next event the same turn offers it
/// again (§15-14-1-4; Q1818).
#[test]
fn declined_you_may_effect_keeps_its_once_per_turn() {
    cite!(
        "15-14-1-4",
        "0ec475393b7da51a023fdc14b305d8c33160c89fce5d469544b9040c676a117f"
    );
    let mut r = board("SEC_MAY_DEL", &[]);
    remove_p0_security(&mut r);
    pass(&mut r);
    assert_eq!(r.game.players[1].battle_area.len(), 2, "declined: nothing deleted");

    remove_p0_security(&mut r);
    assert!(
        r.pending_selection().is_some(),
        "the declined [Once Per Turn] effect is offered again the same turn"
    );
    accept(&mut r);
    assert_eq!(r.game.players[1].battle_area.len(), 1, "the second offer deletes");
}

/// Choosing to activate counts the use: after the effect has resolved, the
/// next event the same turn does not offer it (§15-14-1-4, §15-14-1-2).
#[test]
fn activated_you_may_effect_spends_its_once_per_turn() {
    cite!(
        "15-14-1-4",
        "0ec475393b7da51a023fdc14b305d8c33160c89fce5d469544b9040c676a117f"
    );
    let mut r = board("SEC_MAY_DEL", &[]);
    remove_p0_security(&mut r);
    accept(&mut r);
    assert_eq!(r.game.players[1].battle_area.len(), 1);

    remove_p0_security(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "an activated [Once Per Turn] effect is not offered again the same turn"
    );
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

/// A triggered effect whose optional processing condition (a cost) is
/// declined has still been activated: the use is spent "even if the effect's
/// processing can't be executed" (§15-14-1-4; owner ruling #20).
#[test]
fn declined_optional_cost_of_an_activated_effect_spends_its_once_per_turn() {
    cite!(
        "15-14-1-4",
        "0ec475393b7da51a023fdc14b305d8c33160c89fce5d469544b9040c676a117f"
    );
    let mut r = board("SEC_COST_DEL", &["VANILLA_LV3", "VANILLA_LV3"]);
    remove_p0_security(&mut r);
    pass(&mut r);
    assert_eq!(r.game.players[0].hand.len(), 2, "the cost was not paid");
    assert_eq!(r.game.players[1].battle_area.len(), 2);

    remove_p0_security(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "the effect was activated, so its [Once Per Turn] is spent"
    );
}
