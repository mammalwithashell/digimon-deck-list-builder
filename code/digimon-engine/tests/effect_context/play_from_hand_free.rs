//! Phase 2f1 Task 2 — `EffectContext::play_from_hand_free` plays a card from
//! the controller's hand into the battle area without subtracting memory.
//!
//! ## Audit finding: thin alias over `play_from_hand_with_cost`
//!
//! `Game::play_from_hand_with_cost(player, hand_index, CostDelta::Free)` already
//! covers "play to battle area, do not change memory":
//!   - `CostDelta::Free.resolve(_) == 0` → `effective_cost = 0` →
//!     `pay_memory(0)` is a no-op (memory unchanged).
//!   - OnPlay + OnEnterFieldAnyone triggers fire as normal at the end of the
//!     path.
//!
//! The DSL needs a call-site convenience: card scripts saying "play this
//! without paying its memory cost" want a primitive that doesn't force them to
//! spell out `CostDelta::Free`.

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

#[test]
fn play_from_hand_free_does_not_subtract_memory() {
    // Build a runner with a single test card, place it in P0's hand, and
    // start the game so OnPlay / battle-area machinery is live.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-001", "PlayFreeTest"))
        .hand(0, &["TST-001"])
        .memory(3)
        .start();

    // Ensure preconditions match what the test asserts.
    assert_eq!(runner.game.memory, 3, "precondition: memory == 3");
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "precondition: P0 hand has 1 card"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "precondition: P0 battle area empty"
    );

    let src_handle = runner.game.players[0].hand[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        let result = ctx.play_from_hand_free(0, 0);
        assert!(
            result.is_some(),
            "play_from_hand_free should succeed for a valid hand index"
        );
    }

    // Memory must NOT have changed — printed cost (3) must NOT be deducted.
    assert_eq!(
        runner.game.memory, 3,
        "play_from_hand_free must not change memory"
    );

    // The card must now be a permanent in P0's battle area, identifiable by
    // the same `CardHandle` that was in hand before the call.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let perm = &runner.game.players[0].battle_area[0];
    let top: &CardSource = perm.top_card();
    assert_eq!(
        top.handle(),
        src_handle,
        "the same CardSource handle is now on the battle area"
    );

    // Hand is empty.
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "card was removed from hand"
    );
}

#[test]
fn play_from_hand_free_returns_none_for_invalid_hand_index() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-001", "PlayFreeTest"))
        .hand(0, &["TST-001"])
        .memory(3)
        .start();

    let src_handle = runner.game.players[0].hand[0].handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.play_from_hand_free(0, 99) // out-of-bounds hand index
    };

    assert!(result.is_none(), "out-of-bounds hand index returns None");
    assert_eq!(runner.game.memory, 3, "memory untouched on failed play");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "no permanent created on failed play"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "card stays in hand on failed play"
    );
}
