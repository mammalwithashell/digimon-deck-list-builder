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
use digimon_engine::permanent::PermanentHandle;
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
        "15-14-1-2",
        "98e456796774965369540240091e980a3f1dfb06729aa1c2c7a05f0e1c2ec170"
    );
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

/// Owner-decided 2026-10-21 (mechanics-gaps W13; the PDF is silent; DCGO
/// `BT25_028.cs` `if (!executed) activateClass.RemoveUse()`): a
/// "you may A. Then, you may B." [Once Per Turn] effect whose halves are both
/// declined executed nothing, so its use is not spent and the next event the
/// same turn offers it again.
#[test]
fn then_may_effect_with_both_halves_declined_keeps_its_once_per_turn() {
    cite!(
        "15-14-1-4",
        "0ec475393b7da51a023fdc14b305d8c33160c89fce5d469544b9040c676a117f"
    );
    let mut r = board("SEC_THEN_MAY", &[]);
    remove_p0_security(&mut r);
    pass(&mut r); // decline "delete"
    pass(&mut r); // decline "return"
    assert_eq!(r.game.players[1].battle_area.len(), 2, "nothing executed");

    remove_p0_security(&mut r);
    assert!(
        r.pending_selection().is_some(),
        "both halves declined: the [Once Per Turn] is not spent"
    );
}

/// The same effect is spent as soon as either half executes (here the first;
/// the "Then" half is declined).
#[test]
fn then_may_effect_with_one_half_executed_spends_its_once_per_turn() {
    cite!(
        "15-14-1-4",
        "0ec475393b7da51a023fdc14b305d8c33160c89fce5d469544b9040c676a117f"
    );
    let mut r = board("SEC_THEN_MAY", &[]);
    remove_p0_security(&mut r);
    accept(&mut r); // delete one
    pass(&mut r); // decline "return"
    assert_eq!(r.game.players[1].battle_area.len(), 1, "the first half executed");

    remove_p0_security(&mut r);
    assert!(
        r.pending_selection().is_none(),
        "a half executed: the [Once Per Turn] is spent"
    );
}

/// P0: two `LV5_PLAIN` attackers (they survive their security battles, so the
/// second handle stays valid) and a face-up `SECZONE_MAY_DEL` as its only
/// security card; P1: two `VANILLA_LV3` and three security cards.
fn security_zone_board() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = fixtures::builder(&[fixtures::ch15_14_cards()])
        .security(0, &["SECZONE_MAY_DEL"])
        .security(1, &["VANILLA_LV3"; 3])
        .deck(0, &["VANILLA_LV3"; 6])
        .deck(1, &["VANILLA_LV3"; 6])
        .memory(5)
        .start();
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    let a1 = r.place_on_field(0, "LV5_PLAIN", Some(0));
    let a2 = r.place_on_field(0, "LV5_PLAIN", Some(0));
    r.place_on_field(1, "VANILLA_LV3", Some(0));
    r.place_on_field(1, "VANILLA_LV3", Some(0));
    (r, a1, a2)
}

/// §15-14-1-1 / §15-14-1-2 apply to a `{Security}` effect too: its card is not
/// on a permanent, but its [Once Per Turn] is still counted, so after it
/// activated once the next attack the same turn does not offer it.
#[test]
fn security_zone_once_per_turn_effect_is_used_once_per_turn() {
    cite!(
        "15-14-1-2",
        "98e456796774965369540240091e980a3f1dfb06729aa1c2c7a05f0e1c2ec170"
    );
    let (mut r, a1, a2) = security_zone_board();
    r.attack_player(a1, 1, false);
    accept(&mut r);
    settle(&mut r);
    assert_eq!(r.game.players[1].battle_area.len(), 1, "the first activation deleted");

    assert_eq!(r.game.players[0].battle_area.len(), 2, "both attackers remain");
    r.attack_player(a2, 1, false);
    assert!(
        r.game.players[0].battle_area[a2.index as usize].is_suspended,
        "the second attack was declared"
    );
    assert!(
        r.pending_selection().is_none(),
        "the {{Security}} [Once Per Turn] effect is not offered a second time: {:?}",
        r.pending_selection()
    );
}

/// Declining the `{Security}` "you may" keeps the use (§15-14-1-4; Q1818),
/// so the next attack the same turn offers it again.
#[test]
fn declined_security_zone_you_may_effect_keeps_its_once_per_turn() {
    cite!(
        "15-14-1-4",
        "0ec475393b7da51a023fdc14b305d8c33160c89fce5d469544b9040c676a117f"
    );
    let (mut r, a1, a2) = security_zone_board();
    r.attack_player(a1, 1, false);
    pass(&mut r);
    settle(&mut r);
    assert_eq!(r.game.players[1].battle_area.len(), 2);

    r.attack_player(a2, 1, false);
    assert!(
        r.pending_selection().is_some(),
        "the declined {{Security}} effect is offered again the same turn"
    );
}

/// Decline whatever optional prompt is left (the attack's own windows).
fn settle(r: &mut DebugRunner) {
    for _ in 0..8 {
        let Some(sel) = r.pending_selection() else { return };
        let player = sel.selecting_player;
        if sel.is_optional {
            r.execute_action(player, PASS).expect("pass");
        } else {
            let act = sel.valid_action_ids[0];
            r.execute_action(player, act).expect("first pick");
        }
    }
}
