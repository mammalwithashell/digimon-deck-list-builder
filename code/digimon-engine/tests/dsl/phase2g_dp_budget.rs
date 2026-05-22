//! Phase 2g: DSL DP-budget selections bind opponent permanents for later steps.

use digimon_dsl::compiled::{CompiledFormula, CompiledStep};
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::{run_steps, RunOutcome};
use digimon_engine::effect_context::EffectContext;

#[test]
fn dsl_select_dp_budget_binds_opponent_permanents() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("TARGET", "Target"))
        .start();
    let p0 = 0;
    let p1 = 1;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.force_base_dp("TARGET", 4000);
    let target = runner.place_on_field(p1, "TARGET", Some(0));

    let steps = vec![CompiledStep::SelectOpponentDpBudget {
        dp_budget: CompiledFormula::Literal(5000),
        min_picks: 1,
        bind_as: Some("targets".to_string()),
        prompt: "Choose opponents".to_string(),
        then: vec![CompiledStep::DeleteBoundPermanents {
            binding: "targets".to_string(),
        }],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    runner
        .game
        .resolve_selection(p0, encode_attack(0, target.index as u16))
        .expect("pick target");

    assert!(
        runner.game.player(p1).battle_area.is_empty(),
        "target deleted after bound tail"
    );
}
