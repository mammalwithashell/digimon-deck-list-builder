//! Phase 2f1 Task 3a — `EffectContext::play_from_security` plays the top
//! card of a player's security stack into the battle area without
//! subtracting memory.
//!
//! Card text precedent: BT12-091 et al. — "play the top card of your
//! security stack". Distinct from the existing
//! `EffectContext::play_pending_security()` (the security-skill replay path
//! that consumes `Game.pending_security` during attack-time security check).
//!
//! ## Audit finding: thin wrapper around `play_from_hand_with_cost`
//!
//! `Game::play_from_hand_with_cost(player, hand_index, CostDelta::Free)`
//! already covers "play to battle area, do not change memory" with full
//! OnPlay + OnEnterFieldAnyone trigger plumbing. This primitive pops the top
//! of `player.security`, parks it at the end of `player.hand`, and routes
//! through that existing path with `CostDelta::Free`. Hand-transit is the
//! cheapest correct placement strategy that reuses the established trigger
//! flow without duplicating the placement body — see the doc comment on
//! `play_from_security` in `effect_context/mod.rs` for the rationale.

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

#[test]
fn play_from_security_does_not_subtract_memory() {
    // Build a runner with a single test card, place it on top of P0's
    // security stack, and start the game so OnPlay / battle-area machinery
    // is live.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BT2-001", "PlayFromSecurityTest"))
        .security(0, &["BT2-001"])
        .memory(3)
        .start();

    // Preconditions.
    assert_eq!(runner.game.memory, 3, "precondition: memory == 3");
    assert_eq!(
        runner.game.players[0].security.len(),
        1,
        "precondition: P0 security has 1 card"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "precondition: P0 battle area empty"
    );

    let src_handle = runner.game.players[0].security[0].handle();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        let result = ctx.play_from_security(0);
        assert!(
            result.is_some(),
            "play_from_security should succeed for a non-empty security stack"
        );
    }

    // Memory must NOT have changed — printed cost (default 3) must NOT be
    // deducted.
    assert_eq!(
        runner.game.memory, 3,
        "play_from_security must not change memory"
    );

    // Security stack is now empty.
    assert_eq!(
        runner.game.players[0].security.len(),
        0,
        "card was removed from security"
    );

    // The card must now be a permanent in P0's battle area, identifiable by
    // the same `CardHandle` that was on top of security before the call.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let perm = &runner.game.players[0].battle_area[0];
    let top: &CardSource = perm.top_card();
    assert_eq!(
        top.handle(),
        src_handle,
        "the same CardSource handle is now on the battle area"
    );

    // The card must NOT remain in face_up_security after the play (it's no
    // longer in the security zone — keep the bookkeeping clean even though
    // this card was face-down to begin with).
    let face_up = &runner.game.players[0].face_up_security;
    assert!(
        !face_up.contains(&top.card_index),
        "popped card must not linger in face_up_security after play"
    );
}

#[test]
fn play_from_security_returns_none_for_empty_security() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BT2-001", "PlayFromSecurityTest"))
        .memory(3)
        .start();

    assert_eq!(
        runner.game.players[0].security.len(),
        0,
        "precondition: P0 security empty"
    );

    let src_handle = {
        // No real source card — fabricate a synthetic handle for the
        // EffectContext (the primitive doesn't dereference it on the
        // empty-security path, so any handle works).
        digimon_engine::card_source::CardHandle(0)
    };

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.play_from_security(0)
    };

    assert!(
        result.is_none(),
        "empty security stack returns None"
    );
    assert_eq!(runner.game.memory, 3, "memory untouched on empty-security");
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "no permanent created on empty-security"
    );
}

#[test]
fn play_from_security_clears_face_up_flag_when_top_was_face_up() {
    // Setup: top of P0 security is marked face-up. After play_from_security,
    // the entry must be removed from `face_up_security` because the card is
    // no longer in the security zone.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("BT2-001", "PlayFromSecurityFaceUpTest"))
        .security(0, &["BT2-001"])
        .memory(3)
        .start();

    let src_handle = runner.game.players[0].security[0].handle();
    let card_index = runner.game.players[0].security[0].card_index;
    runner.game.players[0]
        .face_up_security
        .insert(card_index);

    assert!(
        runner.game.players[0]
            .face_up_security
            .contains(&card_index),
        "precondition: top of security is marked face-up"
    );

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        let result = ctx.play_from_security(0);
        assert!(result.is_some(), "play succeeds");
    }

    assert!(
        !runner.game.players[0]
            .face_up_security
            .contains(&card_index),
        "face_up_security entry must be cleared after play_from_security"
    );
}
