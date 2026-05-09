use digimon_dsl::compiled::{
    CompiledAttackTargetSpec, CompiledBindingRef, CompiledClause, CompiledStep,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn may_attack_now_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-MAY-ATTACK-NOW
name: May Attack Now Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_digivolving
    process:
      - may_attack_now:
          attacker: this
          targets: player
          without_suspending: true
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let process = &clause.process;
    let CompiledStep::MayAttackNow {
        attacker,
        targets,
        without_suspending,
        optional,
        prompt,
        cost_upgrade,
    } = &process[0]
    else {
        panic!("expected may_attack_now step");
    };
    assert_eq!(*attacker, CompiledBindingRef::Source);
    assert_eq!(*targets, CompiledAttackTargetSpec::Player);
    assert!(*without_suspending);
    assert!(*optional);
    assert_eq!(prompt, &None);
    assert_eq!(cost_upgrade, &None);
}

#[test]
fn force_attack_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-FORCE-ATTACK
name: Force Attack Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_digivolving
    process:
      - force_attack:
          attacker: forced
          targets: player
          without_suspending: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::ForceAttack {
        attacker,
        targets,
        without_suspending,
        prompt,
        cost_upgrade,
    } = &clause.process[0]
    else {
        panic!("expected force_attack step");
    };
    assert_eq!(*attacker, CompiledBindingRef::Named("forced".to_string()));
    assert_eq!(*targets, CompiledAttackTargetSpec::Player);
    assert!(*without_suspending);
    assert_eq!(prompt, &None);
    assert_eq!(cost_upgrade, &None);
}

#[test]
fn may_attack_now_cost_upgrade_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-MAY-ATTACK-UPGRADE
name: May Attack Upgrade Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_digivolving
    process:
      - may_attack_now:
          attacker: this
          targets: any
          optional: true
          cost_upgrade:
            dp: 3000
            security_attack: 1
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::MayAttackNow { cost_upgrade, .. } = &clause.process[0] else {
        panic!("expected may_attack_now step");
    };
    let upgrade = cost_upgrade.expect("cost upgrade should compile");
    assert_eq!(upgrade.dp, 3000);
    assert_eq!(upgrade.security_attack, 1);
}

#[test]
fn cancel_attack_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-CANCEL-ATTACK
name: Cancel Attack Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - cancel_attack: {}
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::CancelAttack = &clause.process[0] else {
        panic!("expected cancel_attack step");
    };
}

#[test]
fn open_counter_window_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-OPEN-COUNTER
name: Open Counter Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - open_counter_window: {}
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::OpenCounterWindow = &clause.process[0] else {
        panic!("expected open_counter_window step");
    };
}

#[test]
fn redirect_attack_target_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-REDIRECT-ATTACK
name: Redirect Attack Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - redirect_attack_target: { new_target: redirect_target }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::RedirectAttackTarget {
        new_target,
        player,
        targets,
        optional,
        prompt,
    } = &clause.process[0]
    else {
        panic!("expected redirect_attack_target step");
    };
    assert_eq!(
        new_target.as_ref(),
        Some(&CompiledBindingRef::Named("redirect_target".to_string()))
    );
    assert_eq!(player, &None);
    assert_eq!(*targets, CompiledAttackTargetSpec::Any);
    assert!(!*optional);
    assert_eq!(prompt, &None);
}

#[test]
fn redirect_attack_target_prompt_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-REDIRECT-ATTACK-PROMPT
name: Redirect Attack Prompt Test
kind: tamer
color: [green]
cost: 4
effects:
  - when: on_ally_attack
    process:
      - redirect_attack_target:
          targets: any
          optional: true
          prompt: "Change attack target"
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::RedirectAttackTarget {
        new_target,
        player,
        targets,
        optional,
        prompt,
    } = &clause.process[0]
    else {
        panic!("expected redirect_attack_target step");
    };
    assert_eq!(new_target, &None);
    assert_eq!(player, &None);
    assert_eq!(*targets, CompiledAttackTargetSpec::Any);
    assert!(*optional);
    assert_eq!(prompt.as_deref(), Some("Change attack target"));
}

#[test]
fn redirect_attack_target_step_rewrites_active_attack_from_bound_selection() {
    let yaml = r#"
card: TEST-REDIRECT-ATTACK-RUNTIME
name: Redirect Attack Runtime Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - select_opponent_permanent:
          bind_as: redirect_target
          filter: { kind: digimon }
          prompt: "Change attack target"
      - redirect_attack_target: { new_target: redirect_target }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("DEF", "Original Defender"))
        .add_card(make_test_card("REDIR", "Redirect Defender"))
        .start();
    runner.force_base_dp("DEF", 1000);
    runner.force_base_dp("REDIR", 9000);
    let attacker = runner.place_on_field(0, "TEST-REDIRECT-ATTACK-RUNTIME", Some(0));
    let defender = runner.place_on_field(1, "DEF", Some(0));
    let redirect = runner.place_on_field(1, "REDIR", Some(0));

    let _ = runner.attack_digimon(attacker, defender, false);
    let action = encode_attack(0, redirect.index as u16);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("OnAttack selection should be pending");
    assert!(
        pending.valid_action_ids.contains(&action),
        "redirect target should be selectable"
    );

    runner
        .game
        .resolve_selection(pending.selecting_player, action)
        .expect("redirect selection resolves");

    assert!(
        runner.game.player(0).battle_area.is_empty(),
        "redirected battle should delete the 5000 DP attacker against the 9000 DP target"
    );
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        2,
        "original defender should survive because the attack was redirected"
    );
}

#[test]
fn cancel_attack_step_stops_battle_damage() {
    let yaml = r#"
card: TEST-CANCEL-ATTACK-RUNTIME
name: Cancel Attack Runtime Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - cancel_attack: {}
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("DEF", "Defender"))
        .start();
    runner.force_base_dp("DEF", 9000);
    let attacker = runner.place_on_field(0, "TEST-CANCEL-ATTACK-RUNTIME", Some(0));
    let defender = runner.place_on_field(1, "DEF", Some(0));

    let _ = runner.attack_digimon(attacker, defender, false);

    assert_eq!(
        runner.game.player(0).battle_area.len(),
        1,
        "attacker should survive because cancel_attack skips battle"
    );
    assert_eq!(
        runner.game.player(1).battle_area.len(),
        1,
        "defender should survive because cancel_attack skips battle"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "cancel_attack should close the attack flow"
    );
}

#[test]
fn force_attack_step_prompts_forced_attacker_controller() {
    let yaml = r#"
card: TEST-FORCE-ATTACK-RUNTIME
name: Force Attack Runtime Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: on_play
    process:
      - select_opponent_permanent:
          bind_as: forced
          filter: { kind: digimon }
          prompt: "Choose an opposing Digimon to attack"
      - force_attack:
          attacker: forced
          targets: player
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("FORCED", "Forced Attacker"))
        .start();
    let source = runner.place_on_field(0, "TEST-FORCE-ATTACK-RUNTIME", Some(0));
    let forced = runner.place_on_field(1, "FORCED", Some(0));

    runner.fire_on_play(0, source.index as usize);
    let select_forced_action = encode_attack(0, forced.index as u16);
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_opponent_permanent should be pending");
    assert_eq!(pending.selecting_player, 0);
    assert!(pending.valid_action_ids.contains(&select_forced_action));
    runner
        .game
        .resolve_selection(0, select_forced_action)
        .expect("forced attacker selection resolves");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("force_attack should install a target prompt");
    assert_eq!(
        pending.selecting_player, 1,
        "the forced Digimon's controller should choose the attack target"
    );
    assert!(
        !pending.is_optional,
        "force_attack should be mandatory once a legal attack exists"
    );
    assert!(
        !pending.valid_action_ids.contains(&PASS),
        "mandatory force_attack prompt should not include PASS"
    );

    let attack_player = encode_attack(forced.index as u16, SECURITY_TARGET);
    assert!(pending.valid_action_ids.contains(&attack_player));
    runner
        .game
        .resolve_selection(1, attack_player)
        .expect("forced attack target selection resolves");
    assert!(
        runner.game.player(1).battle_area[forced.index as usize].is_suspended,
        "forced attack should use the normal suspend cost by default"
    );
}
