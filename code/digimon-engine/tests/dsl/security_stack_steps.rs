use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{PASS, SEL_MY_SECURITY_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::{EffectTiming, TriggerSource};

fn security_steps_yaml() -> &'static str {
    r#"
card: TEST-SECURITY-STEPS
name: Security Steps
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    process:
      - may_add_top_security_to_hand: { of: you }
      - if:
          condition: { security_count_lte: 0 }
          then:
            - recover: { of: you, count: 1 }
"#
}

#[test]
fn dsl_may_add_top_security_to_hand_then_recover_models_bt24_031_inherited_shape() {
    let yaml = security_steps_yaml();
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("security step YAML compiles");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("security steps compile as a triggered inherited effect");
    };
    assert_eq!(triggered.scope, CompiledScope::Inherited);
    assert_eq!(triggered.when, vec![CompiledTiming::WhenAttacking]);
    assert!(triggered.once_per_turn);
    assert!(!triggered.optional);
    assert!(matches!(
        triggered.process.first(),
        Some(CompiledStep::MayAddTopSecurityToHand { .. })
    ));
    let Some(CompiledStep::If { then, .. }) = triggered.process.get(1) else {
        panic!("second step should conditionally recover");
    };
    assert!(matches!(
        then.as_slice(),
        [CompiledStep::Recover { count: 1, .. }]
    ));
}

#[test]
fn may_add_top_security_accepts_then_runs_tail_recovery() {
    let yaml = security_steps_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["TEST-SECURITY-STEPS", "CARRIER"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    assert!(
        runner.pending_is_optional(),
        "only the top-security add should be optional"
    );
    let view = runner.pending_selection_view().expect("security prompt");
    assert_eq!(view.valid_action_ids, vec![SEL_MY_SECURITY_START]);
    runner
        .execute_action(0, SEL_MY_SECURITY_START)
        .expect("accept top security add");

    assert_eq!(
        runner.security_count(0),
        1,
        "security removed then recovered"
    );
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "SECURITY"));
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER"
    );
}

#[test]
fn may_add_top_security_decline_still_runs_tail_without_moving_security() {
    let yaml = security_steps_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["SECURITY"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["TEST-SECURITY-STEPS", "CARRIER"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    assert!(runner.pending_is_optional());
    runner
        .execute_action(0, PASS)
        .expect("decline security add");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(!hand_ids.contains(&"SECURITY".to_string()));
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "SECURITY"
    );
}

#[test]
fn may_add_top_security_with_empty_security_continues_to_recovery_without_prompt() {
    let yaml = security_steps_yaml();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["TEST-SECURITY-STEPS", "CARRIER"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "no security means there is no optional choice to present"
    );
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "RECOVER"
    );
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}
