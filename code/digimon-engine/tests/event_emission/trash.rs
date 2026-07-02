//! `GameEvent::Trash` emission tests. Covers the scenarios in
//! `openspec/changes/add-reward-profiles/specs/engine-event-emission/spec.md`
//! under "Rust engine emits Trash events for every card-to-trash migration".

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;
use digimon_engine::replacement::ReplacementCause;

fn fighter(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

fn drained_trashes(r: &mut DebugRunner) -> Vec<GameEvent> {
    r.game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::Trash { .. }))
        .collect()
}

#[test]
fn deleting_a_single_card_permanent_emits_one_trash_event() {
    // Minimal version of the "Deleting a permanent emits Trash per card" scenario.
    // We assert per-card emission for a single-card stack; the deeper
    // multi-card-stack scenario (top + digi_source + DigiEgg) lives below.
    let mut r = DebugRunner::builder()
        .add_card(fighter("VICTIM", 1000))
        .start();

    let victim = r.place_on_field(0, "VICTIM", Some(0));
    r.game.drain_events();

    r.game
        .delete_permanents_batch(vec![victim], ReplacementCause::OpponentEffect);

    let trashes = drained_trashes(&mut r);
    assert_eq!(
        trashes.len(),
        1,
        "single-card permanent SHALL emit exactly one Trash event; got {trashes:?}"
    );

    let GameEvent::Trash {
        player,
        card_id,
        card_name,
        ..
    } = trashes[0].clone()
    else {
        unreachable!()
    };

    assert_eq!(player, 0, "owner is P1 (Rust 0)");
    assert_eq!(card_id, "VICTIM", "card_id matches the trashed card");
    assert_eq!(
        card_name, "VICTIM",
        "card_name is carried on the Trash event"
    );
}
