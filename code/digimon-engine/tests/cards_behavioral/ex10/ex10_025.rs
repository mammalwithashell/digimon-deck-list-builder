//! EX10-025 Sunarizamon
//!
//! This pass covers the inherited source-trash clause. The [On Play] source
//! placement clause is left for a later source-placement authoring pass.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::SelectionKind;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .build()
}

#[test]
fn ex10_025_has_inherited_source_trash_delete_clause() {
    let runner = runner();
    let card = runner
        .compiled_card("EX10-025")
        .expect("EX10-025 compiled card present");

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
        "EX10-025 must compile inherited source-trash timing"
    );
}

#[test]
fn ex10_025_source_trash_deletes_only_opponent_digimon_cost_four_or_less() {
    let mut host = make_test_card("MINERAL-HOST", "Mineral Host");
    host.traits.push("Mineral".to_string());
    let mut cheap = make_test_card("CHEAP4", "Cheap Cost 4");
    cheap.play_cost = 4;
    let mut expensive = make_test_card("EXPENSIVE5", "Expensive Cost 5");
    expensive.play_cost = 5;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-025")
        .expect("EX10-025 YAML parses and compiles")
        .add_card(host)
        .add_card(cheap)
        .add_card(expensive)
        .build();

    let host = runner.place_on_field(0, "MINERAL-HOST", None);
    let source = runner.push_source(host, "EX10-025");
    runner.place_on_field(1, "CHEAP4", None);
    runner.place_on_field(1, "EXPENSIVE5", None);

    let top = runner.top_card(host);
    {
        let mut ctx = EffectContext::new(&mut runner.game, top, Some(host), 0);
        ctx.trash_card_source(host, source);
    }

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(
        runner.pending_action_count(),
        1,
        "only play-cost-4-or-less opponent Digimon should be selectable"
    );
    runner
        .auto_resolve()
        .expect("EX10-025 delete target resolves");

    let remaining: Vec<&str> = runner
        .game
        .player(1)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data))
        .collect();
    assert_eq!(remaining, vec!["EXPENSIVE5"]);
}
