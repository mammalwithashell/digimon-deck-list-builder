//! AD1-004 WarGreymon

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ad1_004_has_keywords_and_end_turn_attack_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-004")
        .expect("AD1-004 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("AD1-004").expect("compiled card");

    let keyword_count = card
        .effects
        .iter()
        .filter(|clause| {
            matches!(
                clause,
                CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
            )
        })
        .count();
    assert_eq!(keyword_count, 2);
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::EndOfYourTurn)
    )));
}

#[ignore = "pending: G-FORMULA-SOURCE-DP — delete opponent Digimon with DP <= this Digimon's effective DP"]
#[test]
fn ad1_004_deletes_opponent_digimon_at_or_below_self_dp() {}
