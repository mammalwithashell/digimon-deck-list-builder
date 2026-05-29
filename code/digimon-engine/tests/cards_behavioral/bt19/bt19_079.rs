use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

#[test]
fn bt19_079_sets_memory_to_three_at_start_of_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT19-079")
        .expect("BT19-079 YAML loads")
        .memory(1)
        .start();
    let taiki = runner.place_on_field(0, "BT19-079", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(taiki),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.game.memory, 3);
}

#[test]
fn bt19_079_declares_under_tamer_digixros_material_routing_cost_modifier() {
    let runner = DebugRunner::builder()
        .dsl_card("BT19-079")
        .expect("BT19-079 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT19-079").expect("BT19-079 compiles");

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            when_any_ally_played: Some(predicate),
            optional: true,
            pay_cost,
            ..
        }) if format!("{predicate:?}").contains("Xros Heart")
            && format!("{predicate:?}").contains("digixros")
            && pay_cost.iter().any(|step| format!("{step:?}").contains("AllowDigixrosMaterialZone"))
    )));

    assert!(compiled.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnSecurity)
    )));
}
