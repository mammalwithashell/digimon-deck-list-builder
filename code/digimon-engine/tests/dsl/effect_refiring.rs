use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_dsl::validator::{validate, ValidationContext};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::action::space::{encode_digivolve, PASS};
use digimon_engine::build_action_mask;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::enums::EffectSourceKind;
use digimon_engine::selection::SelectionKind;

#[test]
fn dual_option_main_refire_uses_option_attribution_without_playing_the_option() {
    let yaml = r#"
card: TEST-DUAL-REFIRE
name: Dual Refire Test
kind: dual
level: 4
dp: 5000
color: [red]
alt_paths:
  - kind: digivolve
    from: { level_eq: 3, color_is: red }
    cost: 3
dual:
  digimon:
    level: 4
    dp: 5000
    colors: [red]
    effects:
      - when: when_digivolving
        process:
          - refire_effect: { source: source, timing: option_main, optional: true }
  option:
    use_cost: 5
    colors: [purple]
    effects:
      - when: main
        process:
          - select_opponent_permanent:
              bind_as: victim
              optional: false
              filter: { kind: digimon }
              prompt: Choose an opposing Digimon to delete
          - delete_permanent: { target: victim }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let registry = StubRegistry::empty();
    validate(&spec, &ValidationContext { raw_rust: &registry }).unwrap();

    for accept in [false, true] {
        let mut runner = DebugRunner::builder()
            .from_dsl_yaml(yaml)
            .unwrap()
            .add_card(make_test_card("BASE", "Base"))
            .add_card(make_test_card("TARGET", "Target"))
            .hand(0, &["TEST-DUAL-REFIRE"])
            .deck(0, &["BASE"])
            .memory(10)
            .start();
        let base = runner.place_on_field(0, "BASE", Some(0));
        runner.place_on_field(1, "TARGET", Some(0));
        runner.game.enter_main_phase();
        let action = encode_digivolve(0, base.index as u16);
        assert_eq!(build_action_mask(&runner.game, 0)[action as usize], 1.0);
        runner.game.decode_action(action, 0);
        runner.assert_prompt(SelectionKind::EffectChoice, true, 0);
        assert_eq!(runner.pending_action_count(), 1);
        assert_eq!(build_action_mask(&runner.game, 0)[PASS as usize], 1.0);
        if accept {
            runner.execute_branch(0).unwrap();
            runner.assert_prompt(SelectionKind::OppField, false, 0);
            let prompt = runner.pending_selection_view().unwrap();
            assert_eq!(prompt.source_kind, EffectSourceKind::Option);
            assert_eq!(prompt.source_permanent, Some(runner.perm_handle(0, 0)));
            assert_eq!(runner.prompt_card_candidates().len(), 1);
            runner.choose_prompt_card("TARGET", None).unwrap();
        } else {
            runner.decline_prompt().unwrap();
        }
        assert!(runner.pending_selection().is_none());
        assert_eq!(runner.battle_area_size(0), 1);
        assert_eq!(runner.battle_area_size(1), usize::from(!accept));
        assert_eq!(runner.memory(), 7);
        assert!(runner.game.pending_option.is_none());
    }
}

#[test]
fn refire_effect_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-REFIRE-EFFECT
name: Refire Effect Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - refire_effect:
          source: target
          timing: when_digivolving
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let process = &clause.process;
    assert!(matches!(
        process[0],
        CompiledStep::RefireEffect {
            target_attribution: false,
            ..
        }
    ));
    let target_yaml = yaml.replace(
        "optional: true",
        "optional: true\n          target_attribution: true",
    );
    let target_spec: CardSpec =
        serde_yml::from_str(&target_yaml).expect("target attribution parses");
    let target_compiled = compile(&target_spec).expect("target attribution compiles");
    let CompiledClause::Triggered(target_clause) = &target_compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    assert!(matches!(
        target_clause.process[0],
        CompiledStep::RefireEffect {
            target_attribution: true,
            ..
        }
    ));
}

#[test]
fn refire_effect_accepts_on_play_timing() {
    let yaml = r#"
card: TEST-REFIRE-ON-PLAY
name: Refire On Play Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - refire_effect:
          source: target
          timing: on_play
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("on_play refire compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    assert!(matches!(
        &clause.process[0],
        CompiledStep::RefireEffect { timing, .. } if timing == "on_play"
    ));

    let registry = StubRegistry::empty();
    let ctx = ValidationContext {
        raw_rust: &registry,
    };
    validate(&spec, &ctx).expect("on_play refire validates");
}

#[test]
fn refire_effect_accepts_on_play_or_when_digivolving_timing() {
    let yaml = r#"
card: TEST-REFIRE-EITHER
name: Refire Either Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: end_of_your_turn
    process:
      - refire_effect:
          source: target
          timing: on_play_or_when_digivolving
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("either refire compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    assert!(matches!(
        &clause.process[0],
        CompiledStep::RefireEffect { timing, .. } if timing == "on_play_or_when_digivolving"
    ));
}

#[test]
fn refire_effect_rejects_optional_combined_timing() {
    let yaml = r#"
card: TEST-REFIRE-EITHER-OPTIONAL
name: Refire Either Optional Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: end_of_your_turn
    process:
      - refire_effect:
          source: target
          timing: on_play_or_when_digivolving
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");

    let compile_errors = compile(&spec).expect_err("optional combined refire should not compile");
    assert!(compile_errors.iter().any(|e| {
        e.path.ends_with(".refire_effect.optional") && e.message.contains("optional: true")
    }));

    let registry = StubRegistry::empty();
    let ctx = ValidationContext {
        raw_rust: &registry,
    };
    let validate_errors =
        validate(&spec, &ctx).expect_err("optional combined refire should not validate");
    assert!(validate_errors
        .iter()
        .any(|e| e.path.ends_with(".optional") && e.message.contains("optional: true")));
}

// ─── refire_card_effect (foreign-card refire — BT15-102) ────────────────────

#[test]
fn refire_card_effect_yaml_lowers_with_bound_card_and_timing() {
    let yaml = r#"
card: TEST-REFIRE-CARD
name: Refire Card Test
kind: digimon
color: [white]
level: 7
cost: 12
dp: 12000
effects:
  - when: end_of_your_turn
    process:
      - select_trash:
          of: you
          bind_as: pick
          optional: true
          filter: { level_lte: 6 }
          prompt: "Place 1 level 6 or lower card"
      - place_as_bottom_source:
          source: pick
          target: source
          bind_placed_as: placed
      - refire_card_effect:
          card: placed
          timing: on_play
      - trash_from_top:
          of: opponent
          count:
            base: 0
            per:
              source_stack_count:
                filter: { level_eq: 6 }
            delta: 2
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    assert!(matches!(
        &clause.process[1],
        CompiledStep::PlaceAsBottomSource { bind_placed_as: Some(name), .. } if name == "placed"
    ));
    assert!(matches!(
        &clause.process[2],
        CompiledStep::RefireCardEffect { timing, .. } if timing == "on_play"
    ));
    assert!(matches!(
        &clause.process[3],
        CompiledStep::TrashFromTop {
            count_fn: Some(_),
            ..
        }
    ));

    let registry = StubRegistry::empty();
    let ctx = ValidationContext {
        raw_rust: &registry,
    };
    validate(&spec, &ctx).expect("refire_card_effect validates");
}

#[test]
fn refire_card_effect_rejects_unknown_timing() {
    let yaml = r#"
card: TEST-REFIRE-CARD-BAD
name: Refire Card Bad Timing
kind: digimon
color: [white]
level: 7
cost: 12
dp: 12000
effects:
  - when: end_of_your_turn
    process:
      - refire_card_effect:
          card: placed
          timing: on_deletion
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    assert!(
        compile(&spec).is_err(),
        "refire_card_effect must reject timings outside on_play / when_digivolving"
    );
}
