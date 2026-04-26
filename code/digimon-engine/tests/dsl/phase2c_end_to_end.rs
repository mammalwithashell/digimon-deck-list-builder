//! Phase 2c end-to-end: select_opponent_permanent → If { your_turn } → delete_permanent.
//!
//! Proves the Phase 2b continuation dispatcher still works when the parked tail
//! contains an `If` branch that mutates the selected permanent.
//!
//! Step list:
//!   SelectOpponentPermanent { bind_as: "tgt", filter: {}, prompt: "...", optional: false }
//!   If { condition: your_turn == true,
//!        then:        [DeletePermanent { target: Named("tgt") }],
//!        else_branch: [AddDpModifier { target: Named("tgt"), value: 3000, expiry: "EndOfTurn" }] }

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledModifierValue, CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

/// Full round-trip:
///   1. Player 0 is the turn player.
///   2. Player 1 has a Digimon on the field (the target).
///   3. `run_steps` parks on the SelectOpponentPermanent.
///   4. We resolve the selection by picking Player 1's permanent.
///   5. The callback fires the tail: If { your_turn=true, then: DeletePermanent }.
///   6. `your_turn` is true (P0 is the turn player and P0 owns the effect).
///   7. Assert: Player 1's battle_area is empty.
#[test]
fn select_then_if_your_turn_deletes_opponent_permanent() {
    // ── Build the game ────────────────────────────────────────────────────────
    // We need:
    //   - P0: a hand card to act as the "source" of the effect.
    //   - P1: a Digimon on the field (the target of the selection).
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("TGT", "TGT"))
        .hand(0, &["SRC"])
        .hand(1, &["TGT"])
        .build();

    // Place TGT on Player 1's field.
    let _tgt_handle = runner.place_on_field(1, "TGT", None);
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        1,
        "Player 1 should have one permanent before selection"
    );

    // Confirm turn player is 0 (DebugRunner always starts with P0 as turn player).
    let turn_player = runner.game.turn_player();
    assert_eq!(turn_player, 0, "P0 should be the turn player");

    // Source card: P0's hand card.
    let src_card = runner.game.players[0].hand[0].handle();

    // ── Build the step list ───────────────────────────────────────────────────
    let condition = CompiledPredicate {
        your_turn: Some(true),
        ..CompiledPredicate::default()
    };
    let steps = vec![
        CompiledStep::SelectOpponentPermanent {
            filter: CompiledPredicate::default(),
            bind_as: Some("tgt".to_string()),
            prompt: "Choose an opponent's Digimon to delete".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::If {
            condition,
            then: vec![CompiledStep::DeletePermanent {
                target: CompiledBindingRef::Named("tgt".to_string()),
            }],
            else_branch: vec![CompiledStep::AddDpModifier {
                target: CompiledBindingRef::Named("tgt".to_string()),
                value: CompiledModifierValue::Literal(3000),
                expiry: "EndOfTurn".to_string(),
            }],
        },
    ];

    // ── Run steps (parks on SelectOpponentPermanent) ──────────────────────────
    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, turn_player);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // A PendingSelection should be installed with exactly one valid action
    // (Player 1's only permanent).
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("SelectOpponentPermanent should have installed a PendingSelection");
        assert_eq!(
            pending.valid_action_ids.len(),
            1,
            "one opponent permanent → one valid action ID"
        );
        (pending.valid_action_ids[0], pending.selecting_player)
    };

    // ── Resolve the selection ─────────────────────────────────────────────────
    // This fires the callback with the chosen PermanentHandle, which runs the
    // tail: If { your_turn=true } → DeletePermanent { target: Named("tgt") }.
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection should succeed");

    // ── Post-conditions ───────────────────────────────────────────────────────
    assert!(
        runner.game.pending_selection.is_none(),
        "pending_selection should be cleared after resolution"
    );
    assert_eq!(
        runner.game.players[1].battle_area.len(),
        0,
        "Player 1's permanent should have been deleted (then-branch ran because your_turn=true)"
    );
    // P0's battle_area should be unaffected.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        0,
        "Player 0's battle_area should be empty (not the target)"
    );
}

/// Else-branch test: when the context player (P1) is NOT the turn player (P0),
/// `your_turn` evaluates to false and the else-branch fires (`AddDpModifier`
/// on P0's permanent) instead of deleting it.
///
/// This exercises the same continuation pipeline but through the else-branch,
/// confirming both arms of the `If` step compose correctly with the selection.
#[test]
fn select_then_if_opponents_turn_buffs_opponent_permanent() {
    // ── Build the game ────────────────────────────────────────────────────────
    // P0 is the turn player (default). P1's triggered effect fires on P0's turn.
    // Context player = 1 (the effect owner), turn_player = 0.
    // `your_turn` predicate: ctx.player (1) == turn_player() (0) → false.
    // → else_branch (AddDpModifier) fires.
    //
    // SelectOpponentPermanent from P1's perspective targets P0's permanents.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC2", "SRC2"))
        .add_card(make_test_card("TGT2", "TGT2"))
        .hand(0, &["SRC2"])
        .hand(1, &["TGT2"])
        .build();

    // Place SRC2 on P0's field — P1 will target this via SelectOpponentPermanent.
    runner.place_on_field(0, "SRC2", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let p0_base_dp = runner
        .game
        .effective_dp(runner.perm_handle(0, 0))
        .expect("SRC2 has base DP");

    // P1's hand card is the effect source (context player = 1).
    let p1_src = runner.game.players[1].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectOpponentPermanent {
            filter: CompiledPredicate::default(),
            bind_as: Some("tgt".to_string()),
            prompt: "Choose an opponent Digimon".to_string(),
            prompt_key: None,
            optional: false,
        },
        CompiledStep::If {
            condition: CompiledPredicate {
                your_turn: Some(true),
                ..CompiledPredicate::default()
            },
            then: vec![CompiledStep::DeletePermanent {
                target: CompiledBindingRef::Named("tgt".to_string()),
            }],
            else_branch: vec![CompiledStep::AddDpModifier {
                target: CompiledBindingRef::Named("tgt".to_string()),
                value: CompiledModifierValue::Literal(3000),
                expiry: "EndOfTurn".to_string(),
            }],
        },
    ];

    // ── Run steps (parks on SelectOpponentPermanent) ──────────────────────────
    {
        // Context player = 1, turn player = 0 → your_turn is false → else_branch fires.
        let mut ctx = EffectContext::new(&mut runner.game, p1_src, None, 1);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // ── Resolve the selection ─────────────────────────────────────────────────
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("SelectOpponentPermanent installed PendingSelection");
        assert_eq!(pending.valid_action_ids.len(), 1, "one P0 permanent selectable");
        (pending.valid_action_ids[0], pending.selecting_player)
    };

    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve_selection");

    // ── Post-conditions ───────────────────────────────────────────────────────
    assert!(runner.game.pending_selection.is_none(), "selection cleared");

    // Else-branch fired: P0's permanent should still be on the field (not deleted),
    // but its DP should be +3000 higher.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "P0's permanent should NOT be deleted (else-branch ran AddDpModifier)"
    );
    let after_dp = runner
        .game
        .effective_dp(runner.perm_handle(0, 0))
        .expect("P0 permanent still on field");
    assert_eq!(
        after_dp,
        p0_base_dp + 3000,
        "else-branch should have applied +3000 DP modifier to P0's permanent"
    );
}
