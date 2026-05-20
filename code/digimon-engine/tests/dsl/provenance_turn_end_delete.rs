//! Task 11 (PUPPETS-G003 substrate): provenance-bound turn-end self-delete.
//!
//! An effect that plays a Digimon and must delete *that specific permanent*
//! at turn end ("At turn end, delete the Digimon this effect played") schedules
//! the deletion via `EffectContext::schedule_delete_at_end_of_turn`. The
//! deletion is keyed to a stable `ProvenanceToken` (the played card's
//! identity), not a battle-area index, so it survives index shifts. The drain
//! runs in `fire_end_of_your_turn`.
//!
//! These tests exercise the substrate directly (engine API, no card YAML):
//!   (a) normal cleanup — the scheduled permanent is deleted at turn end.
//!   (b) index-shift — a lower-index permanent is deleted before turn end;
//!       the cleanup still targets the right (now-shifted) permanent and
//!       leaves an unrelated permanent alone.
//!   (c) no-op — the scheduled permanent already left; the drain deletes
//!       nothing (and does not over-delete a substitute).

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::CardKind;
use digimon_engine::replacement::ReplacementCause;

fn digimon(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(3000);
    card
}

fn field_ids(runner: &DebugRunner, player: usize) -> Vec<String> {
    runner.game.players[player]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect()
}

/// (a) A scheduled provenance deletion fires at the end of the controller's
/// turn, deleting exactly the scheduled permanent.
#[test]
fn provenance_delete_fires_at_turn_end() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("SRC"))
        .add_card(digimon("PLAYED"))
        .add_card(digimon("BYSTANDER"))
        .hand(0, &["SRC"])
        .memory(5)
        .build();

    let played = runner.place_on_field(0, "PLAYED", Some(0));
    runner.place_on_field(0, "BYSTANDER", Some(0));
    let src = runner.game.players[0].hand[0].handle();

    // Schedule the deletion of PLAYED at turn end.
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        ctx.schedule_delete_at_end_of_turn(played);
    }
    assert_eq!(
        runner.game.scheduled_provenance_deletions.len(),
        1,
        "one provenance deletion must be queued"
    );

    runner.game.enter_main_phase();
    runner.end_turn();
    let _ = runner.auto_resolve();

    assert!(
        !field_ids(&runner, 0).contains(&"PLAYED".to_string()),
        "the scheduled permanent must be deleted at turn end"
    );
    assert!(
        field_ids(&runner, 0).contains(&"BYSTANDER".to_string()),
        "an unrelated permanent must NOT be deleted"
    );
    assert!(
        runner.game.scheduled_provenance_deletions.is_empty(),
        "the queue must be drained after turn end"
    );
}

/// (b) The deletion is keyed to a stable identity: deleting a lower-index
/// permanent before turn end shifts the target's index, but the cleanup still
/// hits the right permanent.
#[test]
fn provenance_delete_survives_index_shift() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("SRC"))
        .add_card(digimon("LOW"))
        .add_card(digimon("PLAYED"))
        .add_card(digimon("UNRELATED"))
        .hand(0, &["SRC"])
        .memory(5)
        .build();

    let _low = runner.place_on_field(0, "LOW", Some(0));
    let played = runner.place_on_field(0, "PLAYED", Some(0));
    let src = runner.game.players[0].hand[0].handle();

    // Capture PLAYED's stable card identity.
    let played_card_index = runner.game.players[0].battle_area[played.index as usize]
        .top_card()
        .card_index;

    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        ctx.schedule_delete_at_end_of_turn(played);
    }

    // Delete LOW (index 0) — PLAYED's index shifts from 1 down to 0.
    let low_handle = runner.perm_handle(0, 0);
    runner
        .game
        .delete_permanent_with_cause(low_handle, ReplacementCause::OwnEffect);
    let _ = runner.auto_resolve();
    // Place an unrelated permanent — it must NOT be the deletion target.
    runner.place_on_field(0, "UNRELATED", Some(0));

    runner.game.enter_main_phase();
    runner.end_turn();
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_index == played_card_index),
        "the scheduled permanent must be deleted by stable identity despite the shift"
    );
    assert!(
        field_ids(&runner, 0).contains(&"UNRELATED".to_string()),
        "the unrelated permanent must survive — the token must not over-match"
    );
}

/// (c) If the scheduled permanent already left the battle area, the drain is a
/// silent no-op — it must not delete a substitute permanent.
#[test]
fn provenance_delete_is_noop_if_target_already_left() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon("SRC"))
        .add_card(digimon("PLAYED"))
        .add_card(digimon("BYSTANDER"))
        .hand(0, &["SRC"])
        .memory(5)
        .build();

    let played = runner.place_on_field(0, "PLAYED", Some(0));
    runner.place_on_field(0, "BYSTANDER", Some(0));
    let src = runner.game.players[0].hand[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        ctx.schedule_delete_at_end_of_turn(played);
    }

    // PLAYED leaves the battle area before turn end.
    let played_handle = runner.perm_handle(0, played.index as usize);
    runner
        .game
        .delete_permanent_with_cause(played_handle, ReplacementCause::OwnEffect);
    let _ = runner.auto_resolve();

    runner.game.enter_main_phase();
    runner.end_turn();
    let _ = runner.auto_resolve();

    // The drain found nothing to delete — BYSTANDER must be untouched.
    assert!(
        field_ids(&runner, 0).contains(&"BYSTANDER".to_string()),
        "the no-op cleanup must not delete a substitute permanent"
    );
    assert!(
        runner.game.scheduled_provenance_deletions.is_empty(),
        "the queue must be drained even when every entry no-ops"
    );
}
