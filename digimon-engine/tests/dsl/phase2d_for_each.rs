//! Phase 2d Task 4: ForEach iterates over a battle-area predicate scan
//! and runs the body per match.

use digimon_dsl::compiled::{CompiledCardKind, CompiledPredicate, CompiledStep};
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
