//! Phase 2f1 Task 4 — play / digivolve / placement step lowerings.
//!
//! This test drives `CompiledStep::PlayFromHand` with a `Free` cost-delta
//! through the synchronous step dispatcher and asserts the wiring works:
//! - Memory is unchanged (Free → 0 cost).
//! - The hand index is consumed (hand shrinks by 1).
//! - The card lands on the battle area.

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledCostDelta, CompiledPlayerRef, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_step;
use digimon_engine::effect_context::EffectContext;

#[test]
fn play_from_hand_step_with_free_cost_delta_consumes_hand_and_keeps_memory() {
    // Setup: P0 hand has 2 cards. TST-A is the in-hand "source" of the
    // effect (we use its handle as `source_card` in the EffectContext);
    // TST-B is the card we want to play via `PlayFromHand`. Memory pre-
    // funded but irrelevant — Free cost-delta means no memory deduction.
    let card_a = make_test_card("TST-A", "A");
    let card_b = make_test_card("TST-B", "B");
    let mut runner = DebugRunner::builder()
        .add_card(card_a)
        .add_card(card_b)
        .hand(0, &["TST-A", "TST-B"])
        .memory(5)
        .start();

    let memory_before = runner.game.memory;
    let hand_before = runner.game.players[0].hand.len();
    let battle_before = runner.game.players[0].battle_area.len();

    // The "casting" card is TST-A at hand index 0 (the source of the effect).
    // Because we're going to drop the card at hand index 1 (TST-B), the
    // source's index doesn't shift. Source of EffectContext is just used
    // for trigger fan-out; we use TST-A's handle so the source is in a real
    // zone and not floating.
    let src_card = runner.game.players[0].hand[0].handle();

    // Bind "idx" → HandIndex(1) so PlayFromHand drops TST-B.
    let mut bindings = Bindings::new();
    bindings.insert_hand_index("idx", 1);

    let step = CompiledStep::PlayFromHand {
        of: CompiledPlayerRef::You,
        hand_index: CompiledBindingRef::Named("idx".into()),
        cost_delta: Some(CompiledCostDelta::Free),
    };

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_step(&step, &mut ctx, &mut bindings);
    }

    // Memory unchanged — Free resolves to 0.
    assert_eq!(
        runner.game.memory,
        memory_before,
        "memory should be unchanged with CostDelta::Free"
    );
    // Hand shrinks by 1.
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before - 1,
        "hand should shrink by 1 (TST-B left for battle area)"
    );
    // Battle area gains 1 permanent.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        battle_before + 1,
        "battle area should gain 1 permanent (TST-B)"
    );
}
