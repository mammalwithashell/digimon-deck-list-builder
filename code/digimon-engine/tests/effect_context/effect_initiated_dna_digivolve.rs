//! Phase 2f1 Task 3c — `EffectContext::effect_initiated_dna_digivolve`
//! merges TWO existing battle-area permanents into a single permanent topped
//! with a card from hand. Card-text precedent: BT5-085 Omnimon-style
//! "DNA digivolve from-effect".
//!
//! Reference implementation: `EffectContext::effect_initiated_digivolve`
//! covers single-target effect-initiated digivolves. The DNA variant
//! consumes both source permanents and stacks their card_sources
//! underneath the new top from hand. Stacking order:
//!   target_a.card_sources ++ target_b.card_sources ++ [from_hand]
//! (target_a's stack first, target_b's stack second, then the new top).
//!
//! No shared `Game::dna_digivolve_from_hand_inner` exists today — the
//! user-action `initiate_dna_digivolve` path stubs execution as
//! `TODO(dna-digivolve-execute)`. The primitive therefore performs the
//! merge inline; the user-action path can adopt the same logic later.

use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use std::sync::Arc;

#[test]
fn effect_initiated_dna_digivolve_merges_two_permanents_with_hand_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    // Place two source permanents on P0's battle area.
    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);

    // Snapshot pre-call invariants.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        2,
        "precondition: P0 has 2 source permanents"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "precondition: P0 has 1 hand card (the DNA result)"
    );
    assert_eq!(runner.game.memory, 5, "precondition: memory == 5");

    // Capture handles before mutation.
    let source_a_handle = runner.game.players[0].battle_area[handle_a.index as usize]
        .top_card()
        .handle();
    let source_b_handle = runner.game.players[0].battle_area[handle_b.index as usize]
        .top_card()
        .handle();
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    // Drive the new primitive. cost=0 with ignore_requirements=true: memory
    // unchanged, no color/level/material-affinity validation.
    let src_handle = hand_card_handle;
    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)
    };

    assert!(
        result.is_some(),
        "effect_initiated_dna_digivolve should succeed for valid handles"
    );

    // Both source permanents are consumed and merged into ONE permanent.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "two source permanents should be replaced by one merged permanent"
    );

    // Result card was consumed from hand.
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "the DNA result card should be removed from hand"
    );

    // Memory unchanged when cost=0 (irrespective of ignore_requirements).
    assert_eq!(
        runner.game.memory, 5,
        "memory must be unchanged when cost=0"
    );

    // Inspect the merged permanent.
    let merged = &runner.game.players[0].battle_area[0];

    // Top of stack is the DNA result card from hand.
    assert_eq!(
        merged.top_card().handle(),
        hand_card_handle,
        "top of merged stack must be the result card from hand"
    );

    // Stack must contain BOTH source handles plus the new top — at least
    // 3 sources total (each source perm contributed exactly its own top card,
    // since they were placed via place_on_field with stack_size=1).
    assert_eq!(
        merged.card_sources.len(),
        3,
        "merged stack must have 3 sources: source_a, source_b, result"
    );

    let source_handles: Vec<_> = merged.card_sources.iter().map(|c| c.handle()).collect();
    assert!(
        source_handles.contains(&source_a_handle),
        "merged stack must contain source A as material"
    );
    assert!(
        source_handles.contains(&source_b_handle),
        "merged stack must contain source B as material"
    );
    assert!(
        source_handles.contains(&hand_card_handle),
        "merged stack must contain the result card on top"
    );

    // Returned handle points to the merged permanent.
    let new_handle = result.unwrap();
    assert_eq!(
        new_handle.player, 0,
        "merged permanent is on P0's battle area"
    );
    assert_eq!(
        new_handle.index as usize, 0,
        "merged permanent occupies the first slot"
    );
}

#[test]
fn effect_initiated_dna_digivolve_preserves_stacked_materials_under_top() {
    // Verify that when the source permanents already have multi-card stacks,
    // both stacks are preserved underneath the new top in deterministic order.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-A-BASE", "DnaSourceABase"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    // Build P0 perm A with two sources (base + top).
    let handle_a = runner.place_on_field(0, "TST-A-BASE", None);
    let data_idx_top_a = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TST-A")
        .expect("TST-A registered");
    let card_index = runner.game.next_card_index();
    let top_src_a = CardSource::new(data_idx_top_a, 0, card_index);
    let top_a_handle = top_src_a.handle();
    runner.game.players[0].battle_area[0]
        .card_sources
        .push(top_src_a);
    let base_a_handle = runner.game.players[0].battle_area[0].card_sources[0].handle();

    // Build P0 perm B with one source.
    let handle_b = runner.place_on_field(0, "TST-B", None);
    let source_b_handle = runner.game.players[0].battle_area[handle_b.index as usize]
        .top_card()
        .handle();

    // Verify pre-conditions on stacks.
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 2);
    assert_eq!(runner.game.players[0].battle_area[1].stack_size(), 1);

    let hand_card_handle = runner.game.players[0].hand[0].handle();
    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)
    };
    assert!(result.is_some(), "primitive succeeded");

    // After the merge: 1 permanent on P0's battle area, with stack of size 4
    // (base_a, top_a, source_b, result) in that order.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let merged = &runner.game.players[0].battle_area[0];
    assert_eq!(
        merged.card_sources.len(),
        4,
        "stack must concatenate target_a's stack, target_b's stack, then top"
    );

    let handles: Vec<_> = merged.card_sources.iter().map(|c| c.handle()).collect();
    // CHOSEN-NOT-CANONICAL contract: order is target_a's stack first,
    // target_b's second, then hand top. When the user-action
    // initiate_dna_digivolve lands (TODO(dna-digivolve-execute) at
    // game_actions.rs:2198), update both this test and that path together.
    assert_eq!(handles[0], base_a_handle, "card_sources[0] = target_a base");
    assert_eq!(handles[1], top_a_handle, "card_sources[1] = target_a top");
    assert_eq!(
        handles[2], source_b_handle,
        "card_sources[2] = target_b's only source"
    );
    assert_eq!(
        handles[3], hand_card_handle,
        "card_sources[3] = new top from hand"
    );
}

#[test]
fn effect_initiated_dna_digivolve_returns_none_when_targets_equal() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        // Same handle for both targets — defensive: must reject.
        ctx.effect_initiated_dna_digivolve(handle_a, handle_a, hand_card_handle, 0, true)
    };
    assert!(
        result.is_none(),
        "must reject when target_a == target_b (would consume same permanent twice)"
    );
    // No mutations.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(runner.game.memory, 5);
}

#[test]
fn effect_initiated_dna_digivolve_returns_none_for_invalid_target() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    // Bogus second target: out-of-range index.
    let bogus_b = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 99,
    };

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, bogus_b, hand_card_handle, 0, true)
    };
    assert!(result.is_none(), "must reject when target_b is invalid");
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(runner.game.memory, 5);
}

#[test]
fn effect_initiated_dna_digivolve_returns_none_when_hand_card_missing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);

    // Hand is empty — fabricate a handle that doesn't exist anywhere.
    let bogus_card = digimon_engine::card_source::CardHandle(9999);

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, bogus_card, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, bogus_card, 0, true)
    };
    assert!(
        result.is_none(),
        "must reject when from_hand card is not in any player's hand"
    );
    // Battle area unchanged.
    assert_eq!(runner.game.players[0].battle_area.len(), 2);
    assert_eq!(runner.game.memory, 5);
}

/// A `CardEffect` that grants +1 memory whenever an `OnDnaDigivolve` trigger
/// fires for the permanent it's attached to. The test below uses memory delta
/// as the observable side effect — same pattern as `DigivolveObsMem` in
/// `timing_dispatch.rs`.
struct DnaDigivolveObsMem;
impl CardEffect for DnaDigivolveObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_dna_digivolve(card)
            .name("+1 on dna digivolve")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn effect_initiated_dna_digivolve_fires_on_dna_digivolve_trigger() {
    // Place TST-A and TST-B on P0's battle area, with TST-DNA-RESULT in hand.
    // Register an OnDnaDigivolve effect against TST-DNA-RESULT that grants +1
    // memory when fired. After the merge, TST-DNA-RESULT sits on top of the
    // merged permanent — so its OnDnaDigivolve effect is in scope regardless
    // of whether Task 4 fires the trigger globally (PlayerBattleArea) or
    // locally (Permanent(merged_handle)).
    //
    // This test FAILS today: `EffectContext::effect_initiated_dna_digivolve`
    // does not yet enqueue OnDnaDigivolve. Task 4 will wire it.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();
    runner.register_effect("TST-DNA-RESULT", Arc::new(DnaDigivolveObsMem));

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    let before = runner.game.memory;

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)
    };
    assert!(
        result.is_some(),
        "precondition: effect_initiated_dna_digivolve should succeed for valid handles"
    );

    let after = runner.game.memory;
    assert_eq!(
        after - before,
        1,
        "OnDnaDigivolve must fire exactly once on the merged permanent \
         (memory before={}, after={}, expected delta=+1)",
        before,
        after
    );
}
