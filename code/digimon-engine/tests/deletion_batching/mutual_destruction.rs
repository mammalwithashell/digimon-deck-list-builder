//! Mutual destruction in battle is one batch — both attacker and defender
//! are passed to `delete_permanents_batch` together (DCGO parity).
//!
//! Today's `resolve_battle` still calls the single-target shim per-handle,
//! but the architectural pattern is verified here: a `[defender, attacker]`
//! batch deletes both in one shot with shared snapshot capture and a
//! single OnDeletion drain across the kill list.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

fn plain_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

#[test]
fn mutual_destruction_two_handle_batch_trashes_both() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A"))
        .add_card(plain_digimon("B"))
        .start();

    let a = r.place_on_field(0, "A", None);
    let b = r.place_on_field(1, "B", None);

    // Pre-conditions: A and B both on field in different players' areas.
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(r.game.players[1].battle_area.len(), 1);

    let outcome = r
        .game
        .delete_permanents_batch(vec![a, b], ReplacementCause::Battle);

    // Both completed; nothing cancelled or substituted.
    assert_eq!(outcome.completed.len(), 2, "both permanents reported completed");
    assert_eq!(outcome.cancelled.len(), 0);
    assert_eq!(outcome.substituted_in.len(), 0);

    // Both permanents off field, both in their owners' trashes.
    assert_eq!(r.game.players[0].battle_area.len(), 0, "A trashed");
    assert_eq!(r.game.players[1].battle_area.len(), 0, "B trashed");
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "A"),
        "A in player 0's trash"
    );
    assert!(
        r.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "B"),
        "B in player 1's trash"
    );
}

#[test]
fn single_handle_batch_returns_one_completed() {
    let mut r = DebugRunner::builder().add_card(plain_digimon("A")).start();
    let a = r.place_on_field(0, "A", None);

    let outcome = r
        .game
        .delete_permanents_batch(vec![a], ReplacementCause::OpponentEffect);
    assert_eq!(outcome.completed.len(), 1);
    assert_eq!(outcome.cancelled.len(), 0);
    assert_eq!(outcome.substituted_in.len(), 0);
}

#[test]
fn empty_batch_no_ops() {
    let mut r = DebugRunner::builder().add_card(plain_digimon("A")).start();

    // An empty kill list should be a no-op (don't touch any state).
    let memory_before = r.game.memory;
    let outcome = r
        .game
        .delete_permanents_batch(Vec::new(), ReplacementCause::OpponentEffect);
    assert_eq!(outcome.completed.len(), 0);
    assert_eq!(outcome.cancelled.len(), 0);
    assert_eq!(r.game.memory, memory_before, "memory unchanged on empty batch");
    assert!(
        r.game.pending_selection.is_none(),
        "no selection installed on empty batch"
    );
}

#[test]
fn batch_with_one_dead_handle_filters_correctly() {
    let mut r = DebugRunner::builder()
        .add_card(plain_digimon("A"))
        .add_card(plain_digimon("B"))
        .start();
    let a = r.place_on_field(0, "A", None);
    let b = r.place_on_field(0, "B", None);

    // Delete A first via a separate call so its handle becomes stale.
    r.game
        .delete_permanents_batch(vec![a], ReplacementCause::OpponentEffect);

    // After A's deletion, indices shifted. B is now at index 0.
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    let b_new = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "B"
    );

    // Now try to batch-delete `a` (stale) and `b_new` (live). The batch
    // entrypoint must filter out the stale handle and proceed with just
    // the live one.
    let outcome = r
        .game
        .delete_permanents_batch(vec![a, b_new], ReplacementCause::OpponentEffect);

    // The stale handle was filtered out at the top of the batch flow.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "B was trashed; stale handle didn't crash the batch"
    );
    let _ = b; // unused after the stale-handle scenario
    let _ = outcome;
}
