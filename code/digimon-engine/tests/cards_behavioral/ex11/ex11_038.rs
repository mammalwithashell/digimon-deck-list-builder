//! EX11-038 Sunarizamon
//!
//! This pass covers the inherited source-trash draw clause. The face-up
//! [When Moving]/[On Play] hand-or-source trash cost is a separate pass.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .build()
}

#[test]
fn ex11_038_has_inherited_source_trash_draw_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX11-038")
        .expect("EX11-038 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
    )));
}

#[test]
fn ex11_038_source_trash_draws_one() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());
    let filler = make_test_card("FILLER", "Filler");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-038")
        .expect("EX11-038 YAML parses and compiles")
        .add_card(host)
        .add_card(filler)
        .deck(0, &["FILLER", "FILLER"])
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "EX11-038");
    let hand_before = runner.hand_size(0);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.hand_size(0), hand_before + 1);
}
