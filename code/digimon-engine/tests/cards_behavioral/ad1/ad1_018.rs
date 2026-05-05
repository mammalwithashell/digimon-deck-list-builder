//! AD1-018 LordKnightmon

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ad1_018_has_on_play_immunity_and_inherited_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("AD1-018")
        .expect("AD1-018 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("AD1-018").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
    )));
}

#[ignore = "pending: G-PLAY-COST-TEXT-COUNT-REDUCTION — play cost reduction needs trash cards whose text mentions Knightmon/Lucemon"]
#[test]
fn ad1_018_reduces_play_cost_with_four_matching_text_cards_in_trash() {}
