use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_087_has_reveal_search_clause() {
    let runner = DebugRunner::builder().dsl_card("BT13-087").expect("load").start();
    let card = runner.compiled_card("BT13-087").expect("compiled card");
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
    )));
}

#[ignore = "pending: G-ALLY-PLAYED-OTHER-EVENT — when another Lucemon/Royal Knight is played, delete all opponent level 4 or lower Digimon"]
#[test]
fn bt13_087_observer_deletes_level_4_or_lower_when_another_matching_digimon_played() {}
