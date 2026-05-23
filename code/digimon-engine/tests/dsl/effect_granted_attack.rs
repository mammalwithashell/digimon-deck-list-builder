use digimon_dsl::compiled::{
    CompiledAttackTargetSpec, CompiledBindingRef, CompiledClause, CompiledStep,
};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{decode_attack, encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

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

// ════════════════════════════════════════════════════════════════════════════
// G-ALLY-PLAYED-MAY-ATTACK — Phase 2 Track J, Task S2.1.
//
// Substrate proof: an `on_any_digimon_played` observer whose process is
// `may_attack_now: { attacker: event_target }` grants the may-attack to the
// EVENT-PLAYED Digimon (the `event_permanent` carried by the trigger), not to
// the observer's own source permanent.
//
// Step 0 finding: NO engine change needed. `may_attack_now`'s `attacker:` is a
// `BindingRef` and the lowering (`combat.rs::resolve_permanent_ref`) routes it
// through `resolve_binding_ref`, which already resolves `event_target` →
// `CompiledBindingRef::EventTarget` → the live `event_permanent`. The clause is
// composable from primitives that landed on/before BASE; these tests are the
// cited proof that close the gap as already-composable.
// ════════════════════════════════════════════════════════════════════════════

/// DSL lowering: `may_attack_now: { attacker: event_target }` lowers the
/// attacker binding to `CompiledBindingRef::EventTarget`, the variant that
/// `resolve_binding_ref` resolves to the event-played permanent.
#[test]
fn may_attack_now_event_target_yaml_lowers_to_event_target_binding() {
    let yaml = r#"
card: TEST-MAY-ATTACK-EVENT-TARGET
name: May Attack Event Target Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: on_any_digimon_played
    process:
      - may_attack_now:
          attacker: event_target
          targets: any
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::MayAttackNow {
        attacker, optional, ..
    } = &clause.process[0]
    else {
        panic!("expected may_attack_now step");
    };
    assert_eq!(
        *attacker,
        CompiledBindingRef::EventTarget,
        "attacker: event_target must lower to the EventTarget binding so the \
         attack is granted to the event-played Digimon, not the observer source"
    );
    assert!(
        *optional,
        "the may-attack must stay an optional decision (§17)"
    );
}

/// A minimal `on_any_digimon_played` observer that grants `may_attack_now` to
/// `event_target`. Used as the substrate vehicle (NOT the production BT20-017
/// card, which also needs token registration + a Union-play primitive — later
/// card-authoring work).
const ALLY_PLAYED_MAY_ATTACK_OBSERVER: &str = r#"
card: TEST-ALLY-PLAYED-OBSERVER
name: Ally Played May Attack Observer
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: on_any_digimon_played
    process:
      - may_attack_now:
          attacker: event_target
          targets: any
          optional: true
          prompt: "That Digimon may attack"
"#;

/// Fire the `OnEnterFieldAnyone` observer timing for a Digimon that has
/// already been placed on the field, carrying it as the `event_permanent`.
/// Mirrors how `play_from_hand` enqueues the trigger, but lets the test place
/// a non-summoning-sick attacker so the substrate question (which permanent
/// the prompt binds to) is isolated from the separate summoning-sickness rule.
fn fire_digimon_played(runner: &mut DebugRunner, played: PermanentHandle) {
    let card: CardHandle = runner.top_card(played);
    runner.game.enqueue_triggered(
        EffectTiming::OnEnterFieldAnyone,
        TriggerSource::EnteredField {
            player: played.player,
            permanent: played,
            card,
            effect_initiated: false,
        },
    );
    runner.game.drain_effect_queue();
}

/// Positive: the granted attack prompt is bound to the EVENT-PLAYED Digimon,
/// not the observer source. The prompt's legal attack actions decode to the
/// played Digimon's field index, and PASS is exposed (optional, §17).
#[test]
fn may_attack_now_event_target_grants_attack_to_event_played_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ALLY_PLAYED_MAY_ATTACK_OBSERVER)
        .unwrap()
        .add_card(make_test_card("ALLY", "Newly Played Ally"))
        .add_card(make_test_card("OPP-DEF", "Opponent Defender"))
        .start();
    runner.game.turn_count = 5;

    // The observer is in play. The "newly played" ally is placed with
    // turn_played = 0 so summoning sickness does not mask the substrate
    // result — what we are proving is which permanent the prompt binds to.
    let observer = runner.place_on_field(0, "TEST-ALLY-PLAYED-OBSERVER", Some(0));
    let played = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-DEF", Some(0));

    fire_digimon_played(&mut runner, played);

    let pending = runner
        .pending_selection_view()
        .expect("may_attack_now must install an attack-target selection");
    assert_eq!(
        pending.selecting_player, 0,
        "the controller of the played Digimon chooses the attack"
    );
    assert!(
        pending.is_optional,
        "the granted attack is a 'may' — the selection must be optional (§17)"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "PASS must be in the action mask so the optional may-attack can be \
         declined (§17)"
    );

    // Every non-PASS legal action must be an attack BY the event-played
    // Digimon (index `played.index`), proving the grant targets the
    // event_permanent and NOT the observer source.
    let attack_actions: Vec<u16> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert!(
        !attack_actions.is_empty(),
        "the played Digimon should have at least one legal attack target"
    );
    for action in &attack_actions {
        let (attacker_index, _target) = decode_attack(*action);
        assert_eq!(
            attacker_index as u8, played.index,
            "granted attack must be by the event-played Digimon (index {}), \
             not the observer source (index {})",
            played.index, observer.index
        );
        assert_ne!(
            attacker_index as u8, observer.index,
            "the observer source must NOT be the attacker"
        );
    }
}

/// Decline branch: resolving the granted may-attack with PASS leaves no attack
/// in flight and the field unchanged — the optional grant can be turned down.
#[test]
fn may_attack_now_event_target_decline_branch_starts_no_attack() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ALLY_PLAYED_MAY_ATTACK_OBSERVER)
        .unwrap()
        .add_card(make_test_card("ALLY", "Newly Played Ally"))
        .add_card(make_test_card("OPP-DEF", "Opponent Defender"))
        .start();
    runner.game.turn_count = 5;

    runner.place_on_field(0, "TEST-ALLY-PLAYED-OBSERVER", Some(0));
    let played = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-DEF", Some(0));

    fire_digimon_played(&mut runner, played);

    let pending = runner
        .pending_selection_view()
        .expect("may_attack_now must install an attack-target selection");
    assert!(
        pending.is_optional,
        "the granted may-attack must be optional so it can be declined (§17)"
    );
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "PASS must be available in the action mask to decline the optional \
         may-attack"
    );

    runner
        .execute_action(0, PASS)
        .expect("declining the optional may-attack must resolve cleanly");

    assert!(
        runner.game.pending_attack.is_none(),
        "declining the may-attack must not start an attack"
    );
    assert!(
        runner.game.pending_selection.is_none(),
        "no follow-up selection should remain after declining"
    );
    assert!(
        !runner.game.player(0).battle_area[played.index as usize].is_suspended,
        "the played Digimon must stay unsuspended after the grant is declined"
    );
}

/// Cross-check: the SAME observer YAML, when its `may_attack_now` attacker is
/// `this` instead of `event_target`, binds the prompt to the observer source —
/// confirming `may_attack_now` is NOT hard-bound to `self` and that the
/// attacker binding genuinely selects between the two permanents.
#[test]
fn may_attack_now_this_vs_event_target_select_different_attackers() {
    let this_observer = r#"
card: TEST-ALLY-PLAYED-OBSERVER-THIS
name: Ally Played Observer (this attacks)
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: on_any_digimon_played
    process:
      - may_attack_now:
          attacker: this
          targets: any
          optional: true
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(this_observer)
        .unwrap()
        .add_card(make_test_card("ALLY", "Newly Played Ally"))
        .add_card(make_test_card("OPP-DEF", "Opponent Defender"))
        .start();
    runner.game.turn_count = 5;

    let observer = runner.place_on_field(0, "TEST-ALLY-PLAYED-OBSERVER-THIS", Some(0));
    let played = runner.place_on_field(0, "ALLY", Some(0));
    runner.place_on_field(1, "OPP-DEF", Some(0));

    fire_digimon_played(&mut runner, played);

    let pending = runner
        .pending_selection_view()
        .expect("may_attack_now must install an attack-target selection");
    for action in pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
    {
        let (attacker_index, _) = decode_attack(action);
        assert_eq!(
            attacker_index as u8, observer.index,
            "with attacker: this the prompt binds to the observer source"
        );
        assert_ne!(
            attacker_index as u8, played.index,
            "with attacker: this the event-played Digimon must NOT be the attacker"
        );
    }
}

/// Documents the summoning-sickness boundary: when the event-played Digimon is
/// summoning-sick (`turn_played == turn_count`, no Rush), `may_attack_now`
/// finds no legal attack for it and installs no prompt. This is the engine's
/// generic attack-eligibility rule, independent of the `event_target`
/// binding — recorded here so the gap entry stays accurate about scope.
#[test]
fn may_attack_now_event_target_respects_summoning_sickness() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(ALLY_PLAYED_MAY_ATTACK_OBSERVER)
        .unwrap()
        .add_card(make_test_card("ALLY", "Newly Played Ally"))
        .add_card(make_test_card("OPP-DEF", "Opponent Defender"))
        .start();
    runner.game.turn_count = 5;

    runner.place_on_field(0, "TEST-ALLY-PLAYED-OBSERVER", Some(0));
    // turn_played == turn_count → summoning-sick, cannot attack (no Rush).
    let played = runner.place_on_field(0, "ALLY", Some(5));
    runner.place_on_field(1, "OPP-DEF", Some(0));

    fire_digimon_played(&mut runner, played);

    assert!(
        runner.pending_selection().is_none(),
        "a summoning-sick event-played Digimon has no legal attack, so \
         may_attack_now installs no prompt (generic attack-eligibility rule)"
    );
}
