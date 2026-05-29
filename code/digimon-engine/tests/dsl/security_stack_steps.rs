use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{encode_attack, PASS, SEL_MY_SECURITY_START};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;
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

#[test]
fn trash_top_security_cost_gates_triggered_tail() {
    let yaml = r#"
card: TEST-SECURITY-COST
name: Security Cost
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - if_trash_top_security_cost:
          of: you
          then:
            - draw: { of: you, count: 1 }
            - gain_memory: 1
"#;
    let mut payable = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("DRAW", "Draw"))
        .security(0, &["SECURITY"])
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = payable.place_on_field(0, "TEST-SECURITY-COST", Some(0));

    payable.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    payable.game.drain_effect_queue();

    assert_eq!(payable.security_count(0), 0, "top security cost was paid");
    assert!(
        zone_ids(&payable.game.players[0].trash, &payable.game.card_data)
            .contains(&"SECURITY".to_string())
    );
    assert!(
        zone_ids(&payable.game.players[0].hand, &payable.game.card_data)
            .contains(&"DRAW".to_string())
    );
    assert_eq!(payable.memory(), 1, "tail resolves after paid cost");

    let mut unpayable = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = unpayable.place_on_field(0, "TEST-SECURITY-COST", Some(0));

    unpayable.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    unpayable.game.drain_effect_queue();

    assert_eq!(unpayable.security_count(0), 0);
    assert!(
        unpayable.game.players[0].hand.is_empty(),
        "tail must not draw when the cost cannot be paid"
    );
    assert_eq!(unpayable.memory(), 0, "tail must not gain memory");
}

#[test]
fn owner_security_placement_cost_gates_tail_on_success() {
    let cost_effect = r#"
card: TEST-PLACEMENT-COST
name: Placement Cost
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - select_own_permanent:
          bind_as: picked
          filter: { name_is: "Target" }
          prompt: "Place Target into security"
      - if_place_permanent_on_owners_security_cost:
          target: picked
          position: top
          face: down
          then:
            - draw: { of: you, count: 1 }
            - gain_memory: 1
"#;
    let mut success = DebugRunner::builder()
        .from_dsl_yaml(cost_effect)
        .expect("placement cost YAML loads")
        .add_card(make_test_card("TARGET", "Target"))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = success.place_on_field(0, "TEST-PLACEMENT-COST", Some(0));
    let target = success.place_on_field(0, "TARGET", Some(0));

    success.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    success.game.drain_effect_queue();
    let view = success
        .pending_selection_view()
        .expect("placement cost target choice");
    assert!(view
        .valid_action_ids
        .contains(&encode_attack(target.player as u16, target.index as u16)));
    success
        .execute_action(
            view.selecting_player,
            encode_attack(target.player as u16, target.index as u16),
        )
        .expect("choose placement target");

    assert_eq!(success.security_count(0), 1);
    assert!(
        zone_ids(&success.game.players[0].hand, &success.game.card_data)
            .contains(&"DRAW".to_string())
    );
    assert_eq!(
        success.memory(),
        1,
        "tail resolves after successful placement"
    );

    let cancel_target = r#"
card: TARGET
name: Target
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 2000
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    process:
      - cancel_replacement: {}
"#;
    let mut prevented = DebugRunner::builder()
        .from_dsl_yaml(cost_effect)
        .expect("placement cost YAML loads")
        .from_dsl_yaml(cancel_target)
        .expect("cancel target YAML loads")
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = prevented.place_on_field(0, "TEST-PLACEMENT-COST", Some(0));
    let target = prevented.place_on_field(0, "TARGET", Some(0));

    prevented.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    prevented.game.drain_effect_queue();
    let view = prevented
        .pending_selection_view()
        .expect("placement cost target choice");
    prevented
        .execute_action(
            view.selecting_player,
            encode_attack(target.player as u16, target.index as u16),
        )
        .expect("choose placement target");

    assert_eq!(
        prevented.security_count(0),
        0,
        "replacement cancellation prevents the placement cost"
    );
    assert!(
        prevented.game.players[0].hand.is_empty(),
        "tail must not draw when placement cost fails"
    );
    assert_eq!(prevented.memory(), 0, "tail must not gain memory");
}

#[test]
fn optional_security_cost_gate_can_be_declined_without_paying_cost() {
    let yaml = r#"
card: TEST-OPTIONAL-SECURITY-COST
name: Optional Security Cost
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    optional: true
    process:
      - if_trash_top_security_cost:
          of: you
          then:
            - draw: { of: you, count: 1 }
            - gain_memory: 1
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("SECURITY", "Security"))
        .add_card(make_test_card("DRAW", "Draw"))
        .security(0, &["SECURITY"])
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TEST-OPTIONAL-SECURITY-COST", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_is_optional(),
        "optional cost effect must expose decline"
    );
    let view = runner.pending_selection_view().expect("optional prompt");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline optional security cost effect");

    assert_eq!(runner.security_count(0), 1, "decline must not pay the cost");
    assert!(
        runner.game.players[0].hand.is_empty(),
        "decline must not resolve the gated tail"
    );
    assert_eq!(runner.memory(), 0);
}

#[test]
fn top_or_bottom_security_cost_offers_both_choices_and_gates_tail() {
    let yaml = r#"
card: TEST-TOP-BOTTOM-SECURITY-COST
name: Top Bottom Security Cost
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - if_trash_top_or_bottom_security_cost:
          of: you
          prompt: "Trash top or bottom security"
          then:
            - draw: { of: you, count: 1 }
            - gain_memory: 1
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("DRAW", "Draw"))
        .security(0, &["BOTTOM", "TOP"])
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TEST-TOP-BOTTOM-SECURITY-COST", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("top/bottom security cost choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    assert!(
        view.is_optional,
        "cost payment must be declinable instead of hidden"
    );
    assert_eq!(view.valid_action_ids.len(), 2);
    let labels: Vec<String> = view
        .effect_choices
        .as_ref()
        .expect("effect-choice labels")
        .iter()
        .map(|choice| choice.label.clone())
        .collect();
    assert_eq!(
        labels,
        vec![
            "Trash top security".to_string(),
            "Trash bottom security".to_string()
        ]
    );

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pay top security cost");

    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0]
            .security
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "BOTTOM",
        "choosing top should trash the last/top security card"
    );
    assert!(
        zone_ids(&runner.game.players[0].trash, &runner.game.card_data)
            .contains(&"TOP".to_string())
    );
    assert!(
        zone_ids(&runner.game.players[0].hand, &runner.game.card_data)
            .contains(&"DRAW".to_string())
    );
    assert_eq!(runner.memory(), 1, "tail resolves only after paid cost");
}

#[test]
fn top_or_bottom_security_cost_handles_one_empty_decline_and_prevented_paths() {
    let yaml = r#"
card: TEST-TOP-BOTTOM-SECURITY-COST-EDGE
name: Top Bottom Security Cost Edge
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - if_trash_top_or_bottom_security_cost:
          of: you
          prompt: "Trash top or bottom security"
          then:
            - draw: { of: you, count: 1 }
            - gain_memory: 1
"#;

    let mut one_card = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("ONLY", "Only"))
        .add_card(make_test_card("DRAW", "Draw"))
        .security(0, &["ONLY"])
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = one_card.place_on_field(0, "TEST-TOP-BOTTOM-SECURITY-COST-EDGE", Some(0));
    one_card.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    one_card.game.drain_effect_queue();
    assert!(
        one_card.pending_selection().is_none(),
        "a one-card stack has no meaningful top/bottom choice"
    );
    assert_eq!(one_card.security_count(0), 0);
    assert!(
        zone_ids(&one_card.game.players[0].hand, &one_card.game.card_data)
            .contains(&"DRAW".to_string())
    );
    assert_eq!(one_card.memory(), 1);

    let mut empty = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = empty.place_on_field(0, "TEST-TOP-BOTTOM-SECURITY-COST-EDGE", Some(0));
    empty.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    empty.game.drain_effect_queue();
    assert!(empty.pending_selection().is_none());
    assert!(
        empty.game.players[0].hand.is_empty(),
        "empty security means the cost is unpayable and the tail is skipped"
    );
    assert_eq!(empty.memory(), 0);

    let mut declined = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("DRAW", "Draw"))
        .security(0, &["BOTTOM", "TOP"])
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = declined.place_on_field(0, "TEST-TOP-BOTTOM-SECURITY-COST-EDGE", Some(0));
    declined.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    declined.game.drain_effect_queue();
    let view = declined
        .pending_selection_view()
        .expect("optional top/bottom cost choice");
    declined
        .execute_action(view.selecting_player, PASS)
        .expect("decline top/bottom security cost");
    assert_eq!(declined.security_count(0), 2);
    assert!(
        declined.game.players[0].hand.is_empty(),
        "declining the cost must not resolve the gated tail"
    );
    assert_eq!(declined.memory(), 0);

    let cancel_security = r#"
card: GUARD
name: Guard
kind: digimon
level: 3
color: [yellow]
cost: 3
dp: 2000
effects:
  - kind: replacement
    trigger: when_would_be_trashed
    active_when:
      replacement_subject_is_mine: true
    process:
      - cancel_replacement: {}
"#;
    let mut prevented = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("test YAML loads")
        .from_dsl_yaml(cancel_security)
        .expect("cancel security YAML loads")
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("TOP", "Top"))
        .add_card(make_test_card("DRAW", "Draw"))
        .security(0, &["BOTTOM", "TOP"])
        .deck(0, &["DRAW"])
        .memory(0)
        .start();
    let carrier = prevented.place_on_field(0, "TEST-TOP-BOTTOM-SECURITY-COST-EDGE", Some(0));
    prevented.place_on_field(0, "GUARD", Some(0));
    prevented.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    prevented.game.drain_effect_queue();
    let view = prevented
        .pending_selection_view()
        .expect("optional top/bottom cost choice");
    prevented
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("choose prevented top security cost");

    assert_eq!(
        prevented.security_count(0),
        2,
        "replacement cancellation prevents the cost from being paid"
    );
    assert!(
        prevented.game.players[0].hand.is_empty(),
        "prevented cost must not resolve the gated tail"
    );
    assert_eq!(prevented.memory(), 0);
}

#[test]
fn selected_security_play_records_success_and_runs_card_local_tail() {
    let play_search_yaml = r#"
card: TEST-SECURITY-PLAY-SEARCH
name: Security Play Search
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - select_security:
          of: you
          bind_as: sec_digimon
          optional: true
          filter: { kind: digimon }
          prompt: "Play a Digimon from security"
      - if:
          condition: { binding_present: sec_digimon }
          then:
            - play_security_card: { of: you, card: sec_digimon }
            - if:
                condition: { effect_played_any_digimon: true }
                then:
                  - gain_memory: 1
"#;
    let played_yaml = r#"
card: SEC-DIGIMON
name: Security Digimon
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: on_play
    process:
      - gain_memory: 2
"#;
    let mut filler = make_test_card("FILLER", "Filler");
    filler.card_kind = CardKind::Option;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(play_search_yaml)
        .expect("security play search YAML loads")
        .from_dsl_yaml(played_yaml)
        .expect("played security Digimon YAML loads")
        .add_card(filler)
        .security(0, &["FILLER", "SEC-DIGIMON"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TEST-SECURITY-PLAY-SEARCH", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();

    let view = runner
        .pending_selection_view()
        .expect("selected-security play prompt");
    assert_eq!(
        view.valid_action_ids,
        vec![SEL_MY_SECURITY_START + 1],
        "only the Digimon security card should be selectable"
    );
    runner
        .execute_action(view.selecting_player, SEL_MY_SECURITY_START + 1)
        .expect("play selected security Digimon");

    assert_eq!(runner.security_count(0), 1, "selected card left security");
    assert!(runner.game.players[0].battle_area.iter().any(|permanent| {
        permanent.top_card().card_id(&runner.game.card_data) == "SEC-DIGIMON"
    }));
    assert_eq!(
        runner.memory(),
        3,
        "played card OnPlay (+2) and play-success tail (+1) must both resolve"
    );
}

#[test]
fn declined_selected_security_play_skips_play_success_tail() {
    let play_search_yaml = r#"
card: TEST-SECURITY-PLAY-DECLINE
name: Security Play Decline
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - select_security:
          of: you
          bind_as: sec_digimon
          optional: true
          filter: { kind: digimon }
          prompt: "Play a Digimon from security"
      - if:
          condition: { binding_present: sec_digimon }
          then:
            - play_security_card: { of: you, card: sec_digimon }
            - if:
                condition: { effect_played_any_digimon: true }
                then:
                  - gain_memory: 1
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(play_search_yaml)
        .expect("security play search YAML loads")
        .add_card(make_test_card("SEC-DIGIMON", "Security Digimon"))
        .security(0, &["SEC-DIGIMON"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TEST-SECURITY-PLAY-DECLINE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("selected-security play prompt");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline selected security play");

    assert_eq!(
        runner.security_count(0),
        1,
        "decline leaves security intact"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "only the carrier should remain in battle area"
    );
    assert_eq!(runner.memory(), 0, "play-success tail must not resolve");
}

#[test]
fn selected_security_digivolve_records_success_and_runs_tail() {
    let yaml = r#"
card: TEST-SECURITY-DIGIVOLVE
name: Security Digivolve
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - select_security:
          of: you
          bind_as: sec_evo
          optional: true
          filter: { kind: digimon }
          prompt: "Digivolve from security"
      - if:
          condition: { binding_present: sec_evo }
          then:
            - effect_initiated_digivolve:
                target: source
                source: sec_evo
                cost: free
                ignore_requirements: true
            - if:
                condition: { effect_digivolved_any_digimon: true }
                then:
                  - gain_memory: 1
      - shuffle_security: { of: you }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("security digivolve YAML loads")
        .add_card(make_test_card("BOTTOM", "Bottom"))
        .add_card(make_test_card("EVO", "Evo"))
        .add_card(make_test_card("TOP", "Top"))
        .security(0, &["BOTTOM", "EVO", "TOP"])
        .memory(0)
        .start();
    let source = runner.place_on_field(0, "TEST-SECURITY-DIGIVOLVE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("security prompt");
    assert!(
        view.valid_action_ids.contains(&(SEL_MY_SECURITY_START + 1)),
        "the middle security card must be selectable"
    );
    runner
        .execute_action(view.selecting_player, SEL_MY_SECURITY_START + 1)
        .expect("choose security digivolve card");

    let stack = &runner.game.players[0].battle_area[source.index as usize];
    assert_eq!(stack.top_card().card_id(&runner.game.card_data), "EVO");
    let security = zone_ids(&runner.game.players[0].security, &runner.game.card_data);
    assert_eq!(
        sorted_ids(&security),
        vec!["BOTTOM", "TOP"],
        "the selected security card leaves and all unselected cards remain"
    );
    assert_eq!(
        runner.memory(),
        1,
        "effect_digivolved_any_digimon gates the card-local tail"
    );
}

#[test]
fn selected_security_search_shuffles_remaining_stack_after_pick() {
    let yaml = r#"
card: TEST-SECURITY-SEARCH-SHUFFLE-PICK
name: Security Search Shuffle Pick
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - select_security:
          of: you
          bind_as: picked
          optional: true
          filter: { name_is: "Pick" }
          prompt: "Pick a security card"
      - if:
          condition: { binding_present: picked }
          then:
            - add_to_hand_from_security: { of: you, card: picked }
      - shuffle_security: { of: you }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("security search shuffle YAML loads")
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("PICK", "Pick"))
        .add_card(make_test_card("B", "B"))
        .add_card(make_test_card("C", "C"))
        .add_card(make_test_card("D", "D"))
        .add_card(make_test_card("E", "E"))
        .security(0, &["A", "PICK", "B", "C", "D", "E"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TEST-SECURITY-SEARCH-SHUFFLE-PICK", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("security search prompt");
    assert_eq!(view.valid_action_ids, vec![SEL_MY_SECURITY_START + 1]);
    runner
        .execute_action(view.selecting_player, SEL_MY_SECURITY_START + 1)
        .expect("select security card and shuffle");

    let security = zone_ids(&runner.game.players[0].security, &runner.game.card_data);
    assert_eq!(
        sorted_ids(&security),
        vec!["A", "B", "C", "D", "E"],
        "shuffle keeps exactly the non-selected security cards"
    );
    assert_ne!(
        security,
        vec!["A", "B", "C", "D", "E"],
        "remaining searched security stack must be shuffled after selection"
    );
    assert!(
        zone_ids(&runner.game.players[0].hand, &runner.game.card_data)
            .contains(&"PICK".to_string())
    );
}

#[test]
fn declined_optional_security_search_still_shuffles_stack() {
    let yaml = r#"
card: TEST-SECURITY-SEARCH-SHUFFLE-DECLINE
name: Security Search Shuffle Decline
kind: digimon
color: [yellow]
level: 3
cost: 3
dp: 1000
effects:
  - when: start_of_your_main_phase
    process:
      - select_security:
          of: you
          bind_as: picked
          optional: true
          filter: { name_is: "Pick" }
          prompt: "Pick a security card"
      - if:
          condition: { binding_present: picked }
          then:
            - add_to_hand_from_security: { of: you, card: picked }
      - shuffle_security: { of: you }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("security search shuffle YAML loads")
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("PICK", "Pick"))
        .add_card(make_test_card("B", "B"))
        .add_card(make_test_card("C", "C"))
        .add_card(make_test_card("D", "D"))
        .add_card(make_test_card("E", "E"))
        .security(0, &["A", "PICK", "B", "C", "D", "E"])
        .memory(0)
        .start();
    let carrier = runner.place_on_field(0, "TEST-SECURITY-SEARCH-SHUFFLE-DECLINE", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
    let view = runner
        .pending_selection_view()
        .expect("security search prompt");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline search and shuffle");

    let security = zone_ids(&runner.game.players[0].security, &runner.game.card_data);
    assert_eq!(
        sorted_ids(&security),
        vec!["A", "B", "C", "D", "E", "PICK"],
        "decline keeps every searched security card"
    );
    assert_ne!(
        security,
        vec!["A", "PICK", "B", "C", "D", "E"],
        "searched security stack must still be shuffled after decline"
    );
    assert!(
        runner.game.players[0].hand.is_empty(),
        "decline must not process a card from security"
    );
}

#[test]
fn trash_top_security_count_formula_trashes_until_threshold() {
    let yaml = r#"
card: TEST-TRASH-SECURITY-THRESHOLD
name: Trash Security Threshold
kind: digimon
color: [purple]
level: 6
cost: 12
dp: 12000
effects:
  - when: start_of_your_main_phase
    process:
      - trash_top_security:
          of: opponent
          count:
            base: -4
            per:
              card_count_in_zone:
                zone: security
                of: opponent
            delta: 1
"#;
    let mut above = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("threshold YAML loads")
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("B", "B"))
        .add_card(make_test_card("C", "C"))
        .add_card(make_test_card("D", "D"))
        .add_card(make_test_card("E", "E"))
        .security(1, &["A", "B", "C", "D", "E"])
        .start();
    let source = above.place_on_field(0, "TEST-TRASH-SECURITY-THRESHOLD", Some(0));
    above.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(source),
    );
    above.game.drain_effect_queue();
    assert_eq!(above.security_count(1), 4);
    assert_eq!(
        zone_ids(&above.game.players[1].trash, &above.game.card_data),
        vec!["E"],
        "only the excess top security card is trashed"
    );

    let mut at_threshold = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("threshold YAML loads")
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("B", "B"))
        .add_card(make_test_card("C", "C"))
        .add_card(make_test_card("D", "D"))
        .security(1, &["A", "B", "C", "D"])
        .start();
    let source = at_threshold.place_on_field(0, "TEST-TRASH-SECURITY-THRESHOLD", Some(0));
    at_threshold.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(source),
    );
    at_threshold.game.drain_effect_queue();
    assert_eq!(at_threshold.security_count(1), 4);
    assert!(
        at_threshold.game.players[1].trash.is_empty(),
        "the formula clamps at zero when the player is already at threshold"
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

fn sorted_ids(ids: &[String]) -> Vec<&str> {
    let mut ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    ids.sort_unstable();
    ids
}
