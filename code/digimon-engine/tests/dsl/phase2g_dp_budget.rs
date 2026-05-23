//! Phase 2g: DSL DP-budget selections bind opponent permanents for later steps.

use digimon_dsl::compiled::{CompiledFormula, CompiledPredicate, CompiledStep};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS};
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
        filter: CompiledPredicate::default(),
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

#[test]
fn dsl_select_dp_budget_updates_remaining_budget_and_deletes_picked_targets() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("OPP-7K", "Seven"))
        .add_card(make_test_card("OPP-8K", "Eight"))
        .add_card(make_test_card("OPP-9K", "Nine"))
        .start();
    let p0 = 0;
    let p1 = 1;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.force_base_dp("OPP-7K", 7000);
    runner.force_base_dp("OPP-8K", 8000);
    runner.force_base_dp("OPP-9K", 9000);
    let seven = runner.place_on_field(p1, "OPP-7K", Some(0));
    let eight = runner.place_on_field(p1, "OPP-8K", Some(0));
    let nine = runner.place_on_field(p1, "OPP-9K", Some(0));

    let steps = vec![CompiledStep::SelectOpponentDpBudget {
        dp_budget: CompiledFormula::Literal(15000),
        min_picks: 1,
        filter: CompiledPredicate::default(),
        bind_as: Some("targets".to_string()),
        prompt: "Delete up to 15000 DP".to_string(),
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
        .resolve_selection(p0, encode_attack(0, seven.index as u16))
        .expect("pick 7000 DP target");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("budget selection continues after first pick");
    assert!(
        pending.valid_action_ids.contains(&PASS),
        "min_picks satisfied, so PASS can finish the selection"
    );
    assert!(
        pending
            .valid_action_ids
            .contains(&encode_attack(0, eight.index as u16)),
        "8000 DP target remains legal after 7000 DP was picked from a 15000 DP budget"
    );
    assert!(
        !pending
            .valid_action_ids
            .contains(&encode_attack(0, nine.index as u16)),
        "9000 DP target is illegal after 7000 DP was picked from a 15000 DP budget"
    );

    runner
        .game
        .resolve_selection(p0, PASS)
        .expect("finish budget selection");

    let remaining_ids: Vec<_> = runner
        .game
        .player(p1)
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        remaining_ids,
        vec!["OPP-8K".to_string(), "OPP-9K".to_string()]
    );
}

#[test]
fn dsl_select_dp_budget_min_pick_hides_pass_until_first_pick() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("OPP-7K", "Seven"))
        .start();
    let p0 = 0;
    let p1 = 1;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.force_base_dp("OPP-7K", 7000);
    runner.place_on_field(p1, "OPP-7K", Some(0));

    let steps = vec![CompiledStep::SelectOpponentDpBudget {
        dp_budget: CompiledFormula::Literal(15000),
        min_picks: 1,
        filter: CompiledPredicate::default(),
        bind_as: Some("targets".to_string()),
        prompt: "Delete up to 15000 DP".to_string(),
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

    let mask = build_action_mask(&runner.game, p0);
    assert_eq!(mask[PASS as usize], 0.0, "first pick is mandatory");
    assert!(runner.game.resolve_selection(p0, PASS).is_err());
}
