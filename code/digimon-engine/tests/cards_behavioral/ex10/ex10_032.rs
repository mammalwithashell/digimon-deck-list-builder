//! EX10-032 Proganomon
//!
//! This pass covers the inherited source-trash De-Digivolve clause. The
//! hand-main and face-up source-trash buff clauses are separate passes.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .build()
}

#[test]
fn ex10_032_has_inherited_source_trash_dedigivolve_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX10-032")
        .expect("EX10-032 compiled card present");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.scope == CompiledScope::Inherited
                && triggered.when == vec![CompiledTiming::OnDigivolutionCardTrashed]
    )));
}

#[test]
fn ex10_032_source_trash_dedigivolves_one_opponent_digimon() {
    let mut host = make_test_card("ROCK-HOST", "Rock Host");
    host.traits.push("Rock".to_string());

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-032")
        .expect("EX10-032 YAML parses and compiles")
        .dsl_card("BT24-011")
        .expect("BT24-011 YAML parses and compiles")
        .dsl_card("BT21-024")
        .expect("BT21-024 YAML parses and compiles")
        .add_card(host)
        .build();

    let host = runner.place_on_field(0, "ROCK-HOST", None);
    let source = runner.push_source(host, "EX10-032");
    let opponent = runner.place_stack(1, &["BT24-011", "BT21-024"]);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    runner
        .auto_resolve()
        .expect("EX10-032 de-digivolve target resolves");

    assert_eq!(
        runner.game.player(1).battle_area[opponent.index as usize]
            .card_sources
            .len(),
        1
    );
}
