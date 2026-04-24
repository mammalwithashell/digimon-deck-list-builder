//! Phase 2c — control-flow step lowering (Optional + If).

use digimon_dsl::compiled::{CompiledPredicate, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

/// `Optional(body)` always runs the inner body in Phase 2c. The opt-out UX
/// lands in Phase 2d alongside `ScheduleDelayed`.
#[test]
fn optional_runs_inner_body() {
    let card = make_test_card("T-OPT", "T-OPT");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-OPT"])
        .build();

    // A hand-card handle is sufficient — the body (GainMemory) doesn't
    // interact with any permanent.
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![CompiledStep::Optional(vec![
        CompiledStep::GainMemory(2),
    ])];
    let mut bindings = Bindings::new();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "Optional body should always run in Phase 2c: memory should have increased by 2"
    );
}

// ── If tests ──────────────────────────────────────────────────────────────────

/// `If { condition: your_turn=true, then: GainMemory(3), else: LoseMemory(3) }`
/// where the context player IS the turn player — then-branch should run.
#[test]
fn if_true_runs_then_branch() {
    let card = make_test_card("T-IF-TRUE", "T-IF-TRUE");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-IF-TRUE"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let turn_player = runner.game.turn_player();
    let memory_before = runner.game.memory;

    let condition = CompiledPredicate {
        your_turn: Some(true),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::If {
        condition,
        then: vec![CompiledStep::GainMemory(3)],
        else_branch: vec![CompiledStep::LoseMemory(3)],
    }];
    let mut bindings = Bindings::new();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, turn_player);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 3,
        "If condition is true (your_turn) — then-branch (GainMemory 3) should have run"
    );
}

/// `If { condition: opponents_turn=true, then: GainMemory(100), else: GainMemory(1) }`
/// where the context player IS the turn player (so opponents_turn is false) — else-branch runs.
#[test]
fn if_false_runs_else_branch() {
    let card = make_test_card("T-IF-FALSE", "T-IF-FALSE");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-IF-FALSE"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let turn_player = runner.game.turn_player();
    let memory_before = runner.game.memory;

    // opponents_turn: true — but the context player IS the turn player, so this is false.
    let condition = CompiledPredicate {
        opponents_turn: Some(true),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::If {
        condition,
        then: vec![CompiledStep::GainMemory(100)],
        else_branch: vec![CompiledStep::GainMemory(1)],
    }];
    let mut bindings = Bindings::new();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, turn_player);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "If condition is false (opponents_turn but we are turn player) — else-branch (GainMemory 1) should have run"
    );
}

/// `If { condition: false, then: GainMemory(5), else_branch: [] }` — empty else is a no-op.
#[test]
fn if_with_empty_else_false_branch_is_noop() {
    let card = make_test_card("T-IF-NOOP", "T-IF-NOOP");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-IF-NOOP"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();
    let turn_player = runner.game.turn_player();
    let memory_before = runner.game.memory;

    // opponents_turn: true but we pass the turn player — predicate is false.
    let condition = CompiledPredicate {
        opponents_turn: Some(true),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::If {
        condition,
        then: vec![CompiledStep::GainMemory(5)],
        else_branch: vec![],
    }];
    let mut bindings = Bindings::new();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, turn_player);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before,
        "If condition is false with empty else-branch — memory should be unchanged"
    );
}
