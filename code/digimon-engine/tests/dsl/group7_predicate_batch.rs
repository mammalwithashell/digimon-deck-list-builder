use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDpConstraint, CompiledModifierTarget,
    CompiledModifierValue, CompiledPlayerRef, CompiledPredicate, CompiledStep,
};
use digimon_engine::action::space::{PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::{DualCardData, DualDigimonFace, DualOptionFace};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::predicate::{eval_predicate_with_bindings, PredicateSubject};
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::{EffectContext, EffectReadContext};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry, Keyword, ModifierType};
use digimon_engine::permanent::{Permanent, PermanentHandle};
use digimon_engine::selection::TriggerSource;

fn runner_with_dp_cards() -> DebugRunner {
    let mut src = make_test_card("SRC", "Source");
    src.play_cost = 9;
    let mut low = make_test_card("LOW-DP", "Low DP");
    low.dp = Some(3000);
    low.play_cost = 3;
    let mut high = make_test_card("HIGH-DP", "High DP");
    high.dp = Some(7000);
    high.play_cost = 7;
    DebugRunner::builder()
        .add_card(src)
        .add_card(low)
        .add_card(high)
        .hand(0, &["SRC", "LOW-DP", "HIGH-DP"])
        .build()
}

fn src_card(runner: &DebugRunner) -> CardHandle {
    runner.game.players[0].hand[0].handle()
}

#[test]
fn compile_lowers_group7_predicate_leaves() {
    let yaml = r#"
card: T-G7
name: Group 7
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_hand:
          of: you
          bind_as: pick
          prompt: Pick a cheap Digimon
          filter: { kind: digimon, play_cost_lte: 3 }
      - for_each:
          over: { kind: digimon, not_in_binding: saved, dp_lte: 6000 }
          bind_as: target
          body:
            - suspend: { target: target }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");

    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::SelectHand { filter, .. } = &triggered.process[0] else {
        panic!("expected select_hand");
    };
    assert_eq!(filter.play_cost_lte, Some(CompiledDpConstraint::Literal(3)));
    let CompiledStep::ForEach { over, .. } = &triggered.process[1] else {
        panic!("expected for_each");
    };
    assert_eq!(over.not_in_binding.as_deref(), Some("saved"));
    assert!(matches!(
        over.dp_lte,
        Some(CompiledDpConstraint::Literal(6000))
    ));
}

#[test]
fn predicate_of_alias_lowers_to_owner_scope() {
    let yaml = r#"
card: T-G7-OF
name: Predicate Of Alias
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - for_each:
          over: { of: opponent, kind: digimon }
          bind_as: target
          body:
            - suspend: { target: target }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::ForEach { over, .. } = &triggered.process[0] else {
        panic!("expected for_each");
    };

    assert_eq!(over.owner, Some(CompiledPlayerRef::Opponent));
}

#[test]
fn color_matches_any_field_digimon_compiles() {
    let yaml = r#"
card: T-G7-COLOR
name: Board Color Predicate
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_hand:
          of: you
          bind_as: tamer
          prompt: Pick matching Tamer
          filter:
            all_of:
              - kind: tamer
              - color_matches_any_field_digimon: { of: you }
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let digimon_dsl::compiled::CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let digimon_dsl::compiled::CompiledStep::SelectHand { filter, .. } = &triggered.process[0]
    else {
        panic!("expected select_hand");
    };
    assert!(filter
        .all_of
        .iter()
        .any(|p| p.color_matches_any_field_digimon
            == Some(digimon_dsl::compiled::CompiledPlayerRef::You)));
}

#[test]
fn self_digivolution_contains_name_compiles_in_when_digivolving_condition() {
    let yaml = r#"
card: T-G7-SELF-DIGI
name: Stack Name Predicate
kind: digimon
level: 6
color: [white]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    condition: { self_digivolution_contains_name: Omnimon }
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile yaml");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let condition = triggered.condition.as_ref().expect("condition");
    assert_eq!(
        condition.self_digivolution_contains_name.as_deref(),
        Some("Omnimon")
    );
}

#[test]
fn self_digivolution_contains_name_scans_source_permanent_stack_at_runtime() {
    let yaml = r#"
card: T-G7-SELF-DIGI-RUNTIME
name: Stack Name Predicate Runtime
kind: digimon
level: 6
color: [white]
cost: 12
dp: 12000
effects:
  - when: when_digivolving
    condition: { self_digivolution_contains_name: Omnimon }
    process:
      - gain_memory: 1
"#;
    let mut omnimon = make_test_card("BASE-OMNI", "Omnimon");
    omnimon.card_kind = CardKind::Digimon;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(omnimon)
        .build();
    let target = {
        let g = runner.game_mut();
        let turn = g.turn_count;
        let base_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "BASE-OMNI")
            .unwrap();
        let top_idx = g
            .card_data
            .iter()
            .position(|c| c.card_id == "T-G7-SELF-DIGI-RUNTIME")
            .unwrap();
        let base = CardSource::new(base_idx, 0, g.next_card_index());
        let top = CardSource::new(top_idx, 0, g.next_card_index());
        let mut permanent = Permanent::new(base, turn);
        permanent.card_sources.push(top);
        g.players[0].battle_area.push(permanent);
        PermanentHandle {
            player: 0,
            index: (g.players[0].battle_area.len() - 1) as u8,
        }
    };

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(target),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 1);
}

#[test]
fn has_keyword_predicate_matches_permanent_keywords() {
    let mut runner = runner_with_dp_cards();
    let low = runner.place_on_field(0, "LOW-DP", None);
    let high = runner.place_on_field(0, "HIGH-DP", None);
    runner
        .game
        .modifiers
        .grant_keyword(high, Keyword::Jamming, Expiry::Permanent, 0);
    let rctx = EffectReadContext::new(&runner.game, src_card(&runner), None, 0);

    let pred = CompiledPredicate {
        has_keyword: Some("Jamming".to_string()),
        ..Default::default()
    };

    assert!(eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::Permanent(high),
        None
    ));
    assert!(!eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::Permanent(low),
        None
    ));
}

#[test]
fn source_permanent_trait_predicate_reads_carrier_top_card_traits() {
    let mut runner = runner_with_dp_cards();
    let mut free = make_test_card("FREE-TOP", "Free Top");
    free.traits.push("Free".to_string());
    let plain = make_test_card("PLAIN-TOP", "Plain Top");
    runner.game.card_data.push(free);
    runner.game.card_data.push(plain);
    let matching = runner.place_on_field(0, "FREE-TOP", None);
    let non_matching = runner.place_on_field(0, "PLAIN-TOP", None);

    let pred = CompiledPredicate {
        source_permanent_trait_has: Some("Free".to_string()),
        ..Default::default()
    };

    let src = src_card(&runner);
    let matching_ctx = EffectReadContext::new(&runner.game, src, Some(matching), 0);
    let non_matching_ctx = EffectReadContext::new(&runner.game, src, Some(non_matching), 0);

    assert!(eval_predicate_with_bindings(
        &pred,
        &matching_ctx,
        PredicateSubject::None,
        None
    ));
    assert!(!eval_predicate_with_bindings(
        &pred,
        &non_matching_ctx,
        PredicateSubject::None,
        None
    ));
}

#[test]
fn permanent_dp_lte_and_dp_gte_filter_by_effective_dp() {
    let mut runner = runner_with_dp_cards();
    let low = runner.place_on_field(0, "LOW-DP", None);
    let high = runner.place_on_field(0, "HIGH-DP", None);
    let rctx = EffectReadContext::new(&runner.game, src_card(&runner), None, 0);

    let dp_lte = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        dp_lte: Some(CompiledDpConstraint::Literal(3000)),
        ..Default::default()
    };
    assert!(eval_predicate_with_bindings(
        &dp_lte,
        &rctx,
        PredicateSubject::Permanent(low),
        None
    ));
    assert!(!eval_predicate_with_bindings(
        &dp_lte,
        &rctx,
        PredicateSubject::Permanent(high),
        None
    ));

    let dp_gte = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        dp_gte: Some(CompiledDpConstraint::Literal(7000)),
        ..Default::default()
    };
    assert!(!eval_predicate_with_bindings(
        &dp_gte,
        &rctx,
        PredicateSubject::Permanent(low),
        None
    ));
    assert!(eval_predicate_with_bindings(
        &dp_gte,
        &rctx,
        PredicateSubject::Permanent(high),
        None
    ));
}

#[test]
fn play_cost_lte_filters_cards_and_permanents() {
    let mut runner = runner_with_dp_cards();
    let cheap = runner.game.players[0].hand[1].handle();
    let expensive = runner.game.players[0].hand[2].handle();
    let high = runner.place_on_field(0, "HIGH-DP", None);
    let rctx = EffectReadContext::new(&runner.game, src_card(&runner), None, 0);
    let pred = CompiledPredicate {
        play_cost_lte: Some(CompiledDpConstraint::Literal(3)),
        ..Default::default()
    };

    assert!(eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::Card(cheap),
        None
    ));
    assert!(!eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::Card(expensive),
        None
    ));
    assert!(!eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::Permanent(high),
        None
    ));
}

#[test]
fn not_in_binding_excludes_permanent_list_members_in_for_each() {
    let mut runner = runner_with_dp_cards();
    let saved = runner.place_on_field(0, "LOW-DP", None);
    let deleted = runner.place_on_field(0, "HIGH-DP", None);
    let source = src_card(&runner);
    let memory_before = runner.game.memory;
    let steps = vec![CompiledStep::ForEach {
        over: CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            not_in_binding: Some("saved".to_string()),
            ..Default::default()
        },
        bind_as: "candidate".to_string(),
        body: vec![CompiledStep::GainMemory(1)],
    }];

    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent_list("saved", vec![saved]);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert_eq!(runner.game.memory, memory_before + 1);
    assert_eq!(
        runner.game.players[deleted.player as usize].battle_area[deleted.index as usize]
            .top_card()
            .card_id(&runner.game.card_data),
        "HIGH-DP"
    );
}

#[test]
fn not_in_binding_requires_existing_binding() {
    let mut runner = runner_with_dp_cards();
    let low = runner.place_on_field(0, "LOW-DP", None);
    let rctx = EffectReadContext::new(&runner.game, src_card(&runner), None, 0);
    let pred = CompiledPredicate {
        not_in_binding: Some("missing".to_string()),
        ..Default::default()
    };
    assert!(!eval_predicate_with_bindings(
        &pred,
        &rctx,
        PredicateSubject::Permanent(low),
        Some(&Bindings::new()),
    ));
}

#[test]
fn add_modifier_filter_threads_bindings_for_not_in_binding() {
    let mut runner = runner_with_dp_cards();
    let saved = runner.place_on_field(0, "LOW-DP", None);
    let target = runner.place_on_field(0, "HIGH-DP", None);
    let source = src_card(&runner);
    let steps = vec![CompiledStep::AddModifier {
        target: CompiledModifierTarget::Filter(CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            not_in_binding: Some("saved".to_string()),
            ..Default::default()
        }),
        modifier: "CannotBeAffected".to_string(),
        value: CompiledModifierValue::Literal(0),
        expiry: "end_of_turn".to_string(),
    }];

    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent_list("saved", vec![saved]);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert!(
        !runner
            .game
            .modifiers
            .has(saved, ModifierType::CannotBeAffected),
        "bound permanent should be excluded from the modifier filter"
    );
    assert!(
        runner
            .game
            .modifiers
            .has(target, ModifierType::CannotBeAffected),
        "unbound permanent should receive the modifier"
    );
}

#[test]
fn select_hand_uses_play_cost_lte_for_valid_actions() {
    let mut runner = runner_with_dp_cards();
    let source = src_card(&runner);
    let steps = vec![CompiledStep::SelectHand {
        of: digimon_dsl::compiled::CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            play_cost_lte: Some(CompiledDpConstraint::Literal(3)),
            ..Default::default()
        },
        bind_as: Some("pick".to_string()),
        prompt: "Pick cheap".to_string(),
        prompt_key: None,
        optional: false,
    }];

    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_hand should install a pending selection");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START + 1]);
}

#[test]
fn play_cost_lte_accepts_formula_threshold_yaml() {
    let yaml = r#"
card: T-G7-PLAY-COST-FORMULA
name: Formula Play Cost Predicate
kind: option
color: [black]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          bind_as: source_digimon
          prompt: Pick source
          filter: { kind: digimon }
      - select_hand:
          of: you
          bind_as: pick
          prompt: Pick no-more-expensive Digimon
          filter:
            kind: digimon
            play_cost_lte:
              formula:
                binding_play_cost: source_digimon
"#;
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("formula play_cost_lte yaml parses");
    let compiled = digimon_dsl::compile::compile(&spec).expect("formula play_cost_lte compiles");

    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::SelectHand { filter, .. } = &triggered.process[1] else {
        panic!("expected select_hand");
    };
    assert!(
        filter.play_cost_lte.is_some(),
        "formula-valued play_cost_lte should lower into the compiled predicate"
    );
}

#[test]
fn scalar_threshold_predicates_accept_formula_threshold_yaml() {
    let yaml = r#"
card: T-G7-SCALAR-FORMULAS
name: Scalar Formula Predicate Thresholds
kind: option
color: [white]
cost: 0
effects:
  - when: main_from_hand
    active_when:
      memory_lte: { formula: 2 }
      security_count_gte: { formula: 1 }
    process:
      - select_hand:
          of: you
          bind_as: pick
          prompt: Pick a levelled card
          filter:
            kind: digimon
            level_lte: { formula: 5 }
            level_gte: { formula: 3 }
      - select_count_capped_multi:
          of: opponent
          zone: battle_area
          max: { formula: 2 }
          bind_as: targets
          prompt: Pick targets
          filter:
            kind: digimon
            materials_count_lte: { formula: 4 }
            stack_size_gte: { formula: 1 }
"#;
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).expect("formula scalar predicate yaml parses");
    let compiled =
        digimon_dsl::compile::compile(&spec).expect("formula scalar predicate thresholds compile");
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let active_when = triggered
        .active_when
        .as_ref()
        .expect("active_when should compile");
    assert!(active_when.memory_lte.is_some());
    assert!(active_when.security_count_gte.is_some());
    let CompiledStep::SelectHand { filter, .. } = &triggered.process[0] else {
        panic!("expected select_hand");
    };
    assert!(filter.level_lte.is_some());
    assert!(filter.level_gte.is_some());
    let CompiledStep::SelectCountCappedMulti { filter, max, .. } = &triggered.process[1] else {
        panic!("expected select_count_capped_multi");
    };
    assert!(matches!(
        max,
        digimon_dsl::compiled::CompiledCountBound::Formula(_)
    ));
    assert!(filter.materials_count_lte.is_some());
    assert!(filter.stack_size_gte.is_some());
}

#[test]
fn select_hand_play_cost_lte_formula_reads_bound_permanent_play_cost() {
    let mut source = make_test_card("SRC", "Source");
    source.card_kind = CardKind::Option;
    source.play_cost = 2;
    let mut bound = make_test_card("BOUND", "Bound");
    bound.card_kind = CardKind::Digimon;
    bound.play_cost = 6;
    let mut cheap = make_test_card("CHEAP", "Cheap");
    cheap.card_kind = CardKind::Digimon;
    cheap.play_cost = 6;
    let mut expensive = make_test_card("EXPENSIVE", "Expensive");
    expensive.card_kind = CardKind::Digimon;
    expensive.play_cost = 7;

    let mut runner = DebugRunner::builder()
        .add_card(source)
        .add_card(bound)
        .add_card(cheap)
        .add_card(expensive)
        .hand(0, &["SRC", "CHEAP", "EXPENSIVE"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let bound_permanent = runner.place_on_field(0, "BOUND", None);
    let steps = vec![CompiledStep::SelectHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            play_cost_lte: Some(CompiledDpConstraint::Formula(
                digimon_dsl::compiled::CompiledFormula::BindingPlayCost(
                    "source_digimon".to_string(),
                ),
            )),
            ..Default::default()
        },
        bind_as: Some("pick".to_string()),
        prompt: "Pick matching play cost".to_string(),
        prompt_key: None,
        optional: false,
    }];

    let mut ctx = EffectContext::new(&mut runner.game, source_card, None, 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("source_digimon", bound_permanent);
    run_steps(&steps, &mut ctx, &mut bindings);

    let pending = runner.game.pending_selection.as_ref().expect("selection");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START + 1]);
}

#[test]
fn result_bound_suspend_predicate_reads_current_effect_result_log() {
    let mut runner = runner_with_dp_cards();
    let own = runner.place_on_field(0, "LOW-DP", None);
    let source = src_card(&runner);
    let steps = vec![
        CompiledStep::Suspend {
            target: digimon_dsl::compiled::CompiledBindingRef::Binding("target".to_string()),
        },
        CompiledStep::If {
            condition: CompiledPredicate {
                effect_suspended_any_own_digimon: Some(true),
                ..Default::default()
            },
            then: vec![CompiledStep::AddModifier {
                target: CompiledModifierTarget::Binding(
                    digimon_dsl::compiled::CompiledBindingRef::Named("target".to_string()),
                ),
                modifier: "ChangeDp".to_string(),
                value: CompiledModifierValue::Literal(1000),
                expiry: "end_of_turn".to_string(),
            }],
            else_branch: vec![],
        },
    ];

    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", own);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert_eq!(
        runner.game.modifiers.sum(own, ModifierType::ChangeDp),
        1000,
        "branch should fire after this effect suspended an own Digimon"
    );
}

#[test]
fn result_bound_suspend_predicate_ignores_opponent_suspends() {
    let mut runner = runner_with_dp_cards();
    let opp = runner.place_on_field(1, "LOW-DP", None);
    let source = src_card(&runner);
    let steps = vec![
        CompiledStep::Suspend {
            target: digimon_dsl::compiled::CompiledBindingRef::Binding("target".to_string()),
        },
        CompiledStep::If {
            condition: CompiledPredicate {
                effect_suspended_any_own_digimon: Some(true),
                ..Default::default()
            },
            then: vec![CompiledStep::GainMemory(1)],
            else_branch: vec![],
        },
    ];

    let before = runner.game.memory;
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", opp);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert_eq!(
        runner.game.memory, before,
        "own-suspend result predicate must not fire for opponent suspends"
    );
}

#[test]
fn result_bound_returned_any_card_reads_current_effect_result_log() {
    let mut runner = runner_with_dp_cards();
    let target = runner.place_on_field(1, "LOW-DP", None);
    let source = src_card(&runner);
    let before = runner.game.memory;
    let steps = vec![
        CompiledStep::ReturnToDeck {
            target: digimon_dsl::compiled::CompiledBindingRef::Binding("target".to_string()),
            position: digimon_dsl::compiled::CompiledStackPosition::Bottom,
            include_sources: false,
        },
        CompiledStep::If {
            condition: CompiledPredicate {
                effect_returned_any_card: Some(true),
                ..Default::default()
            },
            then: vec![CompiledStep::GainMemory(1)],
            else_branch: vec![],
        },
    ];

    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", target);
    run_steps(&steps, &mut ctx, &mut bindings);

    assert_eq!(runner.game.memory, before + 1);
}

#[test]
fn binding_present_and_absent_predicates_read_current_bindings() {
    let mut source = make_test_card("SRC", "Source");
    source.card_kind = CardKind::Option;
    let mut target = make_test_card("TARGET", "Target");
    target.card_kind = CardKind::Digimon;

    let mut runner = DebugRunner::builder()
        .add_card(source)
        .add_card(target)
        .hand(0, &["SRC"])
        .build();
    let source_card = runner.game.players[0].hand[0].handle();
    let target_handle = runner.place_on_field(0, "TARGET", None);
    let rctx = EffectReadContext::new(&runner.game, source_card, None, 0);

    let mut bindings = Bindings::new();
    bindings.insert_permanent("picked", target_handle);

    let present = CompiledPredicate {
        binding_present: Some("picked".to_string()),
        ..Default::default()
    };
    assert!(eval_predicate_with_bindings(
        &present,
        &rctx,
        PredicateSubject::None,
        Some(&bindings)
    ));

    let absent = CompiledPredicate {
        binding_absent: Some("missing".to_string()),
        ..Default::default()
    };
    assert!(eval_predicate_with_bindings(
        &absent,
        &rctx,
        PredicateSubject::None,
        Some(&bindings)
    ));

    let wrongly_absent = CompiledPredicate {
        binding_absent: Some("picked".to_string()),
        ..Default::default()
    };
    assert!(!eval_predicate_with_bindings(
        &wrongly_absent,
        &rctx,
        PredicateSubject::None,
        Some(&bindings)
    ));
}

#[test]
fn select_hand_color_matches_any_field_digimon_filters_by_live_board_colors() {
    let mut red_digimon = make_test_card("RED-DIGI", "Red Digimon");
    red_digimon.colors = vec![CardColor::Red];
    red_digimon.card_kind = CardKind::Digimon;
    let mut red_tamer = make_test_card("RED-TAMER", "Red Tamer");
    red_tamer.colors = vec![CardColor::Red];
    red_tamer.card_kind = CardKind::Tamer;
    let mut blue_tamer = make_test_card("BLUE-TAMER", "Blue Tamer");
    blue_tamer.colors = vec![CardColor::Blue];
    blue_tamer.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(red_digimon)
        .add_card(red_tamer)
        .add_card(blue_tamer)
        .hand(0, &["RED-DIGI", "RED-TAMER", "BLUE-TAMER"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    runner.place_on_field(0, "RED-DIGI", None);

    let steps = vec![CompiledStep::SelectHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Tamer),
            color_matches_any_field_digimon: Some(CompiledPlayerRef::You),
            ..Default::default()
        },
        bind_as: Some("pick".to_string()),
        prompt: "Pick matching Tamer".to_string(),
        prompt_key: None,
        optional: false,
    }];
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);

    let pending = runner.game.pending_selection.as_ref().expect("selection");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START + 1]);
}

#[test]
fn select_hand_color_matches_binding_filters_against_bound_tamer_colors() {
    let mut yellow_tamer = make_test_card("YELLOW-TAMER", "Yellow Tamer");
    yellow_tamer.colors = vec![CardColor::Yellow];
    yellow_tamer.card_kind = CardKind::Tamer;
    let mut yellow_digimon = make_test_card("YELLOW-DIGI", "Yellow Digimon");
    yellow_digimon.colors = vec![CardColor::Yellow];
    yellow_digimon.card_kind = CardKind::Digimon;
    let mut red_digimon = make_test_card("RED-DIGI", "Red Digimon");
    red_digimon.colors = vec![CardColor::Red];
    red_digimon.card_kind = CardKind::Digimon;

    let mut runner = DebugRunner::builder()
        .add_card(yellow_tamer)
        .add_card(yellow_digimon)
        .add_card(red_digimon)
        .hand(0, &["YELLOW-DIGI", "RED-DIGI"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let tamer = runner.place_on_field(0, "YELLOW-TAMER", Some(0));

    let steps = vec![CompiledStep::SelectHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            color_matches_binding: Some("chosen_tamer".to_string()),
            ..Default::default()
        },
        bind_as: Some("pick".to_string()),
        prompt: "Pick matching Digimon".to_string(),
        prompt_key: None,
        optional: false,
    }];
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("chosen_tamer", tamer);
    run_steps(&steps, &mut ctx, &mut bindings);

    let pending = runner.game.pending_selection.as_ref().expect("selection");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START]);
}

#[test]
fn color_matches_any_field_digimon_uses_dual_digimon_face_colors() {
    let mut dual = make_test_card("DUAL-FIELD", "Dual Field");
    dual.card_kind = CardKind::Dual;
    dual.colors = vec![CardColor::Purple];
    dual.dual = Some(DualCardData {
        digimon: DualDigimonFace {
            level: 4,
            dp: 5000,
            colors: vec![CardColor::Green],
            traits: Vec::new(),
            evo_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            keywords: Vec::new(),
        },
        option: DualOptionFace {
            use_cost: 3,
            colors: vec![CardColor::Purple],
            effect_text: String::new(),
            security_text: String::new(),
            keywords: Vec::new(),
        },
    });
    let mut green_tamer = make_test_card("GREEN-TAMER", "Green Tamer");
    green_tamer.colors = vec![CardColor::Green];
    green_tamer.card_kind = CardKind::Tamer;
    let mut purple_tamer = make_test_card("PURPLE-TAMER", "Purple Tamer");
    purple_tamer.colors = vec![CardColor::Purple];
    purple_tamer.card_kind = CardKind::Tamer;

    let mut runner = DebugRunner::builder()
        .add_card(dual)
        .add_card(green_tamer)
        .add_card(purple_tamer)
        .hand(0, &["DUAL-FIELD", "GREEN-TAMER", "PURPLE-TAMER"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    runner.place_on_field(0, "DUAL-FIELD", None);

    let steps = vec![CompiledStep::SelectHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Tamer),
            color_matches_any_field_digimon: Some(CompiledPlayerRef::You),
            ..Default::default()
        },
        bind_as: Some("pick".to_string()),
        prompt: "Pick matching Tamer".to_string(),
        prompt_key: None,
        optional: false,
    }];
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);

    let pending = runner.game.pending_selection.as_ref().expect("selection");
    assert_eq!(pending.valid_action_ids, vec![PLAY_HAND_START + 1]);
}

#[test]
fn select_trash_uses_play_cost_lte_for_valid_actions() {
    let mut runner = runner_with_dp_cards();
    let source = src_card(&runner);
    let low = runner.game.players[0].hand.remove(1);
    let high = runner.game.players[0].hand.remove(1);
    runner.game.players[0].trash.push(low);
    runner.game.players[0].trash.push(high);
    let steps = vec![CompiledStep::SelectTrash {
        of: digimon_dsl::compiled::CompiledPlayerRef::You,
        filter: CompiledPredicate {
            kind: Some(CompiledCardKind::Digimon),
            play_cost_lte: Some(CompiledDpConstraint::Literal(3)),
            ..Default::default()
        },
        bind_as: Some("pick".to_string()),
        prompt: "Pick cheap trash".to_string(),
        prompt_key: None,
        optional: false,
    }];

    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("select_trash should install a pending selection");
    assert_eq!(pending.valid_action_ids, vec![TRASH_EFFECT_START]);
}
