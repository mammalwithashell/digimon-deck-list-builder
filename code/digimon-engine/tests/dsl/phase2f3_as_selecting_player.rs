//! Phase 2f3 Task 2 — `AsSelectingPlayer` step lowering.
//!
//! `AsSelectingPlayer { of, body }` sets the selecting-player override for
//! the duration of `body`, restoring it on synchronous completion. On park
//! (a select inside `body` parks), the override persists across the parked-
//! callback boundary via Task 1's `EffectContext::new_with_override`.
//!
//! Outer-tail siblings (steps that follow the `AsSelectingPlayer` step at
//! the same level) must NOT inherit the override — `drain_dsl_outer_tail`
//! clears the override before draining.

use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

/// Helper: build a runner with a hand source on P0 and one permanent on
/// each player's field. Returns the runner and the source-card handle.
fn fixture_with_permanents() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .hand(0, &["SRC"])
        .build();
    runner.place_on_field(0, "D1", None);
    runner.place_on_field(1, "D2", None);
    runner
}

/// `AsSelectingPlayer { of: Opponent, body: [select_own_permanent] }`
/// — the prompt's `selecting_player` should be P1 (opponent), not P0.
#[test]
fn as_selecting_player_routes_prompt_to_opponent() {
    let mut runner = fixture_with_permanents();
    let src_card = runner.game.players[0].hand[0].handle();

    let body = vec![CompiledStep::SelectOwnPermanent {
        then: vec![],
        filter: CompiledPredicate::default(),
        bind_as: None,
        selector: None,
        prompt: "pick".to_string(),
        prompt_key: None,
        optional: false,
        continue_on_decline: false,
    }];
    let steps = vec![CompiledStep::AsSelectingPlayer {
        of: CompiledPlayerRef::Opponent,
        body,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("body's select_own_permanent should have parked a PendingSelection");
    assert_eq!(
        pending.selecting_player, 1,
        "AsSelectingPlayer with of=Opponent must route the prompt to P1"
    );
}

/// Two selects back-to-back inside the body. After P1 resolves the first
/// (under the override), the second select should ALSO park with
/// `selecting_player = 1` — the override must persist across the parked-
/// callback boundary (Task 1's `new_with_override` mechanism).
#[test]
fn as_selecting_player_chained_selects_all_route_to_override() {
    let mut runner = fixture_with_permanents();
    // Give P1 a hand card so the second SelectHand has a non-empty
    // candidate list.
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "D2")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[1]
        .hand
        .push(digimon_engine::card_source::CardSource::new(idx, 1, next));

    let src_card = runner.game.players[0].hand[0].handle();

    let body = vec![
        CompiledStep::SelectOwnPermanent {
            then: vec![],
            filter: CompiledPredicate::default(),
            bind_as: None,
            selector: None,
            prompt: "pick perm".to_string(),
            prompt_key: None,
            optional: false,
            continue_on_decline: false,
        },
        CompiledStep::SelectHand {
            then: vec![],
            of: CompiledPlayerRef::Opponent,
            filter: CompiledPredicate::default(),
            bind_as: None,
            prompt: "pick hand".to_string(),
            prompt_key: None,
            optional: false,
            cost: false,
        },
    ];
    let steps = vec![CompiledStep::AsSelectingPlayer {
        of: CompiledPlayerRef::Opponent,
        body,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("first select should be parked");
    assert_eq!(
        pending.selecting_player, 1,
        "first body select must route to override (P1)"
    );
    let action = pending.valid_action_ids[0];

    runner
        .game
        .resolve_selection(1, action)
        .expect("P1 should be able to resolve the first parked selection");

    let pending2 = runner
        .game
        .pending_selection
        .as_ref()
        .expect("second select should be parked after first resolves");
    assert_eq!(
        pending2.selecting_player, 1,
        "second body select must ALSO route to override — Task 1's \
         new_with_override must carry the override across the callback boundary"
    );
}

/// Synchronous body: gain_memory(2) inside AsSelectingPlayer. The override
/// is None before, body runs synchronously, override is None after.
#[test]
fn as_selecting_player_synchronous_body_restores_previous_override() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![CompiledStep::AsSelectingPlayer {
        of: CompiledPlayerRef::Opponent,
        body: vec![CompiledStep::GainMemory(2)],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
        assert_eq!(
            ctx.override_selecting_player(),
            None,
            "synchronous body must restore the previous override (None)"
        );
    }
    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "synchronous body should still execute"
    );
}

/// `AsSelectingPlayer { of: You, body: [select_own_permanent] }` —
/// `You` resolves to the controller (P0), so `selecting_player == 0`.
/// This confirms `resolve_player` is wired to `CompiledPlayerRef::You`.
#[test]
fn as_selecting_player_with_self_as_of_routes_to_controller() {
    let mut runner = fixture_with_permanents();
    let src_card = runner.game.players[0].hand[0].handle();

    let body = vec![CompiledStep::SelectOwnPermanent {
        then: vec![],
        filter: CompiledPredicate::default(),
        bind_as: None,
        selector: None,
        prompt: "pick".to_string(),
        prompt_key: None,
        optional: false,
        continue_on_decline: false,
    }];
    let steps = vec![CompiledStep::AsSelectingPlayer {
        of: CompiledPlayerRef::You,
        body,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("body's select_own_permanent should have parked a PendingSelection");
    assert_eq!(
        pending.selecting_player, 0,
        "AsSelectingPlayer with of=You must route the prompt to the controller (P0)"
    );
}

/// `[AsSelectingPlayer { of: Opp, body: [select] }, gain_memory(1)]` —
/// after P1 resolves the body's select, the outer-tail's gain_memory must
/// run. Verifies (a) outer-tail propagation still works for AsSelectingPlayer,
/// and (b) the gain_memory effect lands.
#[test]
fn as_selecting_player_parked_body_drains_outer_tail() {
    let mut runner = fixture_with_permanents();
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let body = vec![CompiledStep::SelectOwnPermanent {
        then: vec![],
        filter: CompiledPredicate::default(),
        bind_as: None,
        selector: None,
        prompt: "pick".to_string(),
        prompt_key: None,
        optional: false,
        continue_on_decline: false,
    }];
    let steps = vec![
        CompiledStep::AsSelectingPlayer {
            of: CompiledPlayerRef::Opponent,
            body,
        },
        CompiledStep::GainMemory(1),
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("body's select_own_permanent should have parked");
    assert_eq!(pending.selecting_player, 1);
    let action = pending.valid_action_ids[0];

    runner
        .game
        .resolve_selection(1, action)
        .expect("P1 should be able to resolve the parked selection");

    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "outer-tail GainMemory(1) must run after body's select resolves"
    );
}

/// Regression guard: an outer-tail select sibling must NOT inherit the
/// `AsSelectingPlayer` override from a preceding step.
///
/// Steps: `[AsSelectingPlayer { of: Opponent, body: [select_own_perm] },
///          select_own_perm]`
///
/// The body's parked select routes to P1 (the override). When P1 resolves
/// it, the parked-callback boundary reconstructs `cb_ctx` with
/// `override = Some(1)` (Task 1's `new_with_override`), the body completes,
/// and `drain_dsl_outer_tail` runs the outer-tail steps. Without the leak
/// fix, `cb_ctx`'s override would still be `Some(1)`, so the outer-tail's
/// `select_own_permanent` would route to P1 again. With the fix
/// (`drain_dsl_outer_tail` calls `set_override_selecting_player(None)`
/// before running the tail), the second select must route to the
/// controller (P0).
#[test]
fn as_selecting_player_outer_tail_select_does_not_inherit_override() {
    let mut runner = fixture_with_permanents();
    let src_card = runner.game.players[0].hand[0].handle();

    let body = vec![CompiledStep::SelectOwnPermanent {
        then: vec![],
        filter: CompiledPredicate::default(),
        bind_as: None,
        selector: None,
        prompt: "body pick".to_string(),
        prompt_key: None,
        optional: false,
        continue_on_decline: false,
    }];
    let steps = vec![
        CompiledStep::AsSelectingPlayer {
            of: CompiledPlayerRef::Opponent,
            body,
        },
        CompiledStep::SelectOwnPermanent {
            then: vec![],
            filter: CompiledPredicate::default(),
            bind_as: None,
            selector: None,
            prompt: "outer-tail pick".to_string(),
            prompt_key: None,
            optional: false,
            continue_on_decline: false,
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // First: body's select parks under the override (P1).
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("body's select_own_permanent should have parked");
    assert_eq!(
        pending.selecting_player, 1,
        "body select must route to override (P1)"
    );
    let body_action = pending.valid_action_ids[0];

    runner
        .game
        .resolve_selection(1, body_action)
        .expect("P1 should resolve the body's parked selection");

    // Second: after the body's callback completes, drain_dsl_outer_tail
    // installs the outer-tail's select_own_permanent. It MUST route to the
    // controller (P0) — not the override (P1) — because drain_dsl_outer_tail
    // clears the override before running the parked tail.
    let pending2 = runner
        .game
        .pending_selection
        .as_ref()
        .expect("outer-tail select_own_permanent should have parked");
    assert_eq!(
        pending2.selecting_player, 0,
        "outer-tail select must route to controller (P0), NOT inherit the \
         AsSelectingPlayer override — drain_dsl_outer_tail must clear the \
         override before running parked tail steps"
    );
    let tail_action = pending2.valid_action_ids[0];

    runner
        .game
        .resolve_selection(0, tail_action)
        .expect("P0 (controller) should resolve the outer-tail selection");

    assert!(
        runner.game.pending_selection.is_none(),
        "all selections should be resolved"
    );
}
