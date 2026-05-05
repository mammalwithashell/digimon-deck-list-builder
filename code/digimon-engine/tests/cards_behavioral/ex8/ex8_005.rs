//! EX8-005 Tumblemon
//!
//! Printed inherited text: When effects trash this card from a [Mineral] or
//! [Rock] trait Digimon's digivolution cards, gain 1 memory.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-005")
        .expect("EX8-005 YAML parses and compiles")
        .build()
}

#[test]
fn ex8_005_has_inherited_source_trash_gain_memory_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX8-005")
        .expect("EX8-005 compiled card present");

    let inherited = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed] =>
        {
            Some(triggered)
        }
        _ => None,
    });

    assert!(
        inherited.is_some(),
        "EX8-005 must compile inherited source-trash timing"
    );
}

#[test]
fn ex8_005_gains_memory_when_exact_source_is_trashed_from_rock_host() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());

    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-005")
        .expect("EX8-005 YAML parses and compiles")
        .add_card(host)
        .memory(0)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "EX8-005");
    let memory_before = runner.memory();

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "EX8-005 inherited source-trash effect must gain 1 memory"
    );
}
