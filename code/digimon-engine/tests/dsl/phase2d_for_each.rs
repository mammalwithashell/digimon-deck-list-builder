//! Phase 2d Task 4: ForEach iterates over a battle-area predicate scan
//! and runs the body per match.

use digimon_dsl::compiled::{CompiledCardKind, CompiledPlayerRef, CompiledPredicate, CompiledStep};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn for_each_runs_body_per_battle_area_match() {
    // Two test digimon on P0's field. ForEach { kind: digimon }: gain_memory(1).
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .hand(0, &["SRC", "D1", "D2"])
        .build();

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(0, "D2", None);
    assert_eq!(runner.game.players[0].battle_area.len(), 2);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::ForEach {
        over: pred,
        bind_as: "tgt".to_string(),
        body: vec![CompiledStep::GainMemory(1)],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "ForEach should have run gain_memory(1) once per matching permanent (2 of them)"
    );
}

/// Predicate that matches nothing → body runs zero times → memory and
/// game state are untouched. Pins down the empty-scan corner.
#[test]
fn for_each_no_matches_runs_body_zero_times() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .hand(0, &["SRC", "D1"])
        .build();

    runner.place_on_field(0, "D1", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    // Predicate matches Tamers; battle area has only digimon.
    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Tamer),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::ForEach {
        over: pred,
        bind_as: "tgt".to_string(),
        body: vec![CompiledStep::GainMemory(1)],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory, memory_before,
        "ForEach with zero matches must not run the body"
    );
}

/// Two players each have a digimon. Default scan visits both players
/// (P0 then P1 by `permanent_scan::scan` ordering). Asserts the body
/// fires twice — once per player — when the predicate is unconstrained
/// by `owner`.
#[test]
fn for_each_scans_across_both_players() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .hand(0, &["SRC", "D1"])
        .hand(1, &["D2"])
        .build();

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(1, "D2", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::ForEach {
        over: pred,
        bind_as: "tgt".to_string(),
        body: vec![CompiledStep::GainMemory(1)],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "Unscoped ForEach should fire once per battle-area permanent across both players"
    );
}

/// Same setup as the cross-player test, but constrains the predicate to
/// `owner: You`. Confirms `permanent_scan::scan` actually applies the
/// `owner` filter via the existing predicate evaluator (i.e. the scan
/// helper isn't accidentally bypassing predicate fields).
#[test]
fn for_each_owner_you_only_matches_self_permanents() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("D1", "D1"))
        .add_card(make_test_card("D2", "D2"))
        .hand(0, &["SRC", "D1"])
        .hand(1, &["D2"])
        .build();

    runner.place_on_field(0, "D1", None);
    runner.place_on_field(1, "D2", None);

    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let pred = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        owner: Some(CompiledPlayerRef::You),
        ..CompiledPredicate::default()
    };
    let steps = vec![CompiledStep::ForEach {
        over: pred,
        bind_as: "tgt".to_string(),
        body: vec![CompiledStep::GainMemory(1)],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "owner: You should restrict the scan to P0's permanent (1 match), not P1's"
    );
}
