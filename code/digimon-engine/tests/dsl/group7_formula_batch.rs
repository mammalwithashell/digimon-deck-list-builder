use digimon_dsl::compiled::{
    CompiledAggregateSelector, CompiledBindingRef, CompiledCardKind, CompiledClause,
    CompiledDeclarativeClause, CompiledDpConstraint, CompiledFormula, CompiledModifierValue,
    CompiledPerSelector, CompiledPlayerRef, CompiledPredicate, CompiledStep, CompiledZone,
};
use digimon_dsl::formula::{CardCountInZoneSpec, FormulaSpec, PerSelector};
use digimon_dsl::predicate::Zone;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::formula_eval;
use digimon_engine::dsl_cards::predicate::{eval_predicate, PredicateSubject};
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::{EffectContext, EffectReadContext};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType, PlayerId};
use digimon_engine::modifiers::ModifierEntry;

fn compile_yaml(yaml: &str) -> digimon_dsl::compiled::CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse yaml");
    digimon_dsl::compile::compile(&spec).expect("compile yaml")
}

fn digimon_card(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn tamer_card(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.dp = None;
    card.level = None;
    card
}

fn colored_tamer_card(id: &str, color: CardColor) -> CardData {
    let mut card = tamer_card(id);
    card.colors = vec![color];
    card
}

fn digimon_card_with_level(id: &str, dp: i32, level: u8) -> CardData {
    let mut card = digimon_card(id, dp);
    card.level = Some(level);
    card
}

fn push_card_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("card_id registered");
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
}

#[test]
fn suspended_count_formula_counts_selected_players_battle_area() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 1000))
        .add_card(digimon_card("A", 3000))
        .add_card(digimon_card("B", 4000))
        .add_card(digimon_card("C", 5000))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    let own = runner.place_on_field(0, "A", None);
    let opp_1 = runner.place_on_field(1, "B", None);
    let opp_2 = runner.place_on_field(1, "C", None);
    runner.game.suspend(own);
    runner.game.suspend(opp_1);
    runner.game.suspend(opp_2);

    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::SuspendedCount {
            of: CompiledPlayerRef::Opponent,
            exclude_source: false,
        },
        delta: 1,
    };

    assert_eq!(formula_eval::evaluate(&formula, &ctx, source), 2);
}

#[test]
fn suspended_count_formula_compiles_from_yaml_per_selector() {
    let yaml = r#"
card: T-G7-SUSPENDED-COUNT
name: Suspended Count Formula
kind: option
color: [green]
cost: 0
effects:
  - when: main_from_hand
    process:
      - select_count_capped_multi:
          of: opponent
          zone: battle_area
          max:
            formula:
              base: 0
              per:
                suspended_count: { of: opponent }
              delta: 1
          bind_as: targets
          prompt: Pick suspended-count targets
          filter: { kind: digimon }
"#;
    let compiled = compile_yaml(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::SelectCountCappedMulti { max, .. } = &triggered.process[0] else {
        panic!("expected select_count_capped_multi");
    };
    assert!(matches!(
        max,
        digimon_dsl::compiled::CompiledCountBound::Formula(CompiledFormula::BasePerDelta {
            per: CompiledPerSelector::SuspendedCount {
                of: CompiledPlayerRef::Opponent,
                ..
            },
            ..
        })
    ));
}

#[test]
fn distinct_colors_count_formula_counts_both_players_battle_area_union() {
    // Phase 2 Track F (G-DSL-DISTINCT-COLORS-BOTH-PLAYERS-FORMULA) — P-182
    // WarGreymon's [All Turns] +1000 DP per distinct color across BOTH
    // players' Digimon+Tamers. Verifies that `of: Any` collapses to the
    // union of both players' colors rather than counting per-player and
    // summing.
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 1000))
        .add_card({
            let mut c = digimon_card("OWN-RED", 3000);
            c.colors = vec![CardColor::Red];
            c
        })
        .add_card({
            let mut c = digimon_card("OPP-RED", 3000);
            c.colors = vec![CardColor::Red];
            c
        })
        .add_card({
            let mut c = digimon_card("OPP-BLUE", 3000);
            c.colors = vec![CardColor::Blue];
            c
        })
        .add_card(colored_tamer_card("OPP-WHITE-TAMER", CardColor::White))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    runner.place_on_field(0, "OWN-RED", None);
    runner.place_on_field(1, "OPP-RED", None);
    runner.place_on_field(1, "OPP-BLUE", None);
    runner.place_on_field(1, "OPP-WHITE-TAMER", None);

    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::DistinctColorsCountScoped {
            zone: CompiledZone::BattleArea,
            of: CompiledPlayerRef::Any,
            filter: None,
        },
        delta: 1000,
    };

    // Union: SRC=test colorless, OWN-RED=red, OPP-RED=red, OPP-BLUE=blue,
    // OPP-WHITE-TAMER=white → distinct colors observed across both players'
    // top cards: {red, blue, white} → 3 (the colorless SRC contributes
    // no color to the union). Times 1000 DP delta → 3000.
    assert_eq!(formula_eval::evaluate(&formula, &ctx, source), 3000);
}

#[test]
fn distinct_colors_count_formula_counts_filtered_tamer_colors() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 1000))
        .add_card(colored_tamer_card("TAI", CardColor::White))
        .add_card(colored_tamer_card("RED-TAMER", CardColor::Red))
        .add_card(colored_tamer_card("RED-TAMER-2", CardColor::Red))
        .add_card(digimon_card("RED-DIGIMON", 3000))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    runner.place_on_field(0, "TAI", None);
    runner.place_on_field(0, "RED-TAMER", None);
    runner.place_on_field(0, "RED-TAMER-2", None);
    runner.place_on_field(0, "RED-DIGIMON", None);

    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 2,
        per: CompiledPerSelector::DistinctColorsCountScoped {
            zone: CompiledZone::BattleArea,
            of: CompiledPlayerRef::You,
            filter: Some(Box::new(CompiledPredicate {
                kind: Some(CompiledCardKind::Tamer),
                ..Default::default()
            })),
        },
        delta: 1,
    };

    assert_eq!(formula_eval::evaluate(&formula, &ctx, source), 4);
}

#[test]
fn level_is_lowest_among_opponent_digimon_filters_only_lowest_level_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card_with_level("SRC", 3000, 3))
        .add_card(digimon_card_with_level("OPP-LOW", 3000, 3))
        .add_card(digimon_card_with_level("OPP-HIGH", 12000, 6))
        .add_card(tamer_card("OPP-TAMER"))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    let low = runner.place_on_field(1, "OPP-LOW", None);
    let high = runner.place_on_field(1, "OPP-HIGH", None);
    let tamer = runner.place_on_field(1, "OPP-TAMER", None);
    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let rctx = EffectReadContext::new(&runner.game, src_card, Some(source), 0);
    let predicate = CompiledPredicate {
        level_matches_aggregate: Some((
            CompiledAggregateSelector::LowestLevel,
            CompiledPlayerRef::Opponent,
        )),
        ..Default::default()
    };

    assert!(eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(low)
    ));
    assert!(!eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(high)
    ));
    assert!(!eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(tamer)
    ));
}

#[test]
fn material_count_is_fewest_among_opponent_digimon_filters_all_tied_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card_with_level("SRC", 3000, 3))
        .add_card(digimon_card_with_level("OPP-ZERO-A", 3000, 3))
        .add_card(digimon_card_with_level("OPP-ZERO-B", 3000, 3))
        .add_card(digimon_card_with_level("OPP-ONE-MAT", 3000, 3))
        .add_card(digimon_card_with_level("OPP-ONE-TOP", 5000, 4))
        .add_card(digimon_card_with_level("OPP-TWO-MAT-A", 3000, 3))
        .add_card(digimon_card_with_level("OPP-TWO-MAT-B", 5000, 4))
        .add_card(digimon_card_with_level("OPP-TWO-TOP", 7000, 5))
        .add_card(tamer_card("OPP-TAMER"))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    let zero_a = runner.place_on_field(1, "OPP-ZERO-A", None);
    let zero_b = runner.place_on_field(1, "OPP-ZERO-B", None);
    let one = runner.place_stack(1, &["OPP-ONE-MAT", "OPP-ONE-TOP"]);
    let two = runner.place_stack(1, &["OPP-TWO-MAT-A", "OPP-TWO-MAT-B", "OPP-TWO-TOP"]);
    let tamer = runner.place_on_field(1, "OPP-TAMER", None);
    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let rctx = EffectReadContext::new(&runner.game, src_card, Some(source), 0);
    let predicate = CompiledPredicate {
        kind: Some(CompiledCardKind::Digimon),
        materials_count_matches_aggregate: Some((
            CompiledAggregateSelector::FewestMaterials,
            CompiledPlayerRef::Opponent,
        )),
        ..Default::default()
    };

    assert!(eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(zero_a)
    ));
    assert!(eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(zero_b)
    ));
    assert!(!eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(one)
    ));
    assert!(!eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(two)
    ));
    assert!(!eval_predicate(
        &predicate,
        &rctx,
        PredicateSubject::Permanent(tamer)
    ));
}

#[test]
fn same_level_pair_count_formula_reads_source_stack_levels() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card_with_level("SRC-4A", 3000, 4))
        .add_card(digimon_card_with_level("SRC-4B", 3000, 4))
        .add_card(digimon_card_with_level("SRC-5", 5000, 5))
        .add_card(digimon_card_with_level("TOP", 12000, 6))
        .build();
    let target = runner.place_stack(0, &["SRC-4A", "SRC-4B", "SRC-5", "TOP"]);
    let src_card = runner.game.players[0].battle_area[target.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::SameLevelPairsInSources,
        delta: 1,
    };

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 1);
}

#[test]
fn shared_trash_count_formula_compiles() {
    let compiled = compile_yaml(
        r#"
card: T-G7-FORMULA
name: Shared Trash Formula
kind: option
color: [purple]
cost: 3
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          bind_as: target
          prompt: Delete target
          filter:
            kind: digimon
            dp_lte:
              formula:
                base: 7000
                per:
                  shared_trash_count: {}
                bucket: 10
                delta: 2000
"#,
    );

    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::SelectOpponentPermanent { filter, .. } = &triggered.process[0] else {
        panic!("expected select_opponent_permanent");
    };
    assert!(matches!(
        filter.dp_lte,
        Some(CompiledDpConstraint::Formula(
            CompiledFormula::BasePerDelta {
                per: CompiledPerSelector::SharedTrashCount { bucket: Some(10) },
                ..
            }
        ))
    ));
}

#[test]
fn binding_dp_formula_compiles() {
    let _compiled = compile_yaml(
        r#"
card: T-G7-BIND-DP
name: Binding DP Formula
kind: option
color: [yellow]
cost: 2
effects:
  - when: main_from_hand
    process:
      - select_own_permanent:
          bind_as: ally
          prompt: Pick ally
          filter: { kind: digimon }
      - select_opponent_permanent:
          bind_as: target
          prompt: Pick target
          filter:
            kind: digimon
            dp_lte:
              formula:
                binding_dp: ally
"#,
    );
}

#[test]
fn card_count_in_zone_formula_accepts_filter() {
    let compiled = compile_yaml(
        r#"
card: T-G7-ZONE-FILTER
name: Filtered Zone Count
kind: option
color: [red]
cost: 7
effects:
  - kind: cost_reduction
    scope: face_up
    amount_fn:
      base: 0
      per:
        card_count_in_zone:
          zone: battle_area
          of: opponent
          filter: { kind: digimon }
      delta: 1
"#,
    );

    let CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
        amount_fn: Some(CompiledFormula::BasePerDelta { per, .. }),
        ..
    }) = &compiled.effects[0]
    else {
        panic!("expected formula cost reduction");
    };
    assert!(matches!(
        per,
        CompiledPerSelector::FilteredCardCountInZoneScoped {
            zone: CompiledZone::BattleArea,
            of: CompiledPlayerRef::Opponent,
            ..
        }
    ));
}

#[test]
fn formula_new_shapes_serialize_in_parseable_yaml_shape() {
    let binding = FormulaSpec::BindingDp {
        binding_dp: "ally".to_string(),
    };
    let binding_yaml = serde_yml::to_string(&binding).expect("serialize binding_dp");
    assert!(binding_yaml.contains("binding_dp"));
    let reparsed: FormulaSpec = serde_yml::from_str(&binding_yaml).expect("reparse binding_dp");
    assert_eq!(reparsed, binding);

    let filtered = FormulaSpec::BasePerDelta {
        base: 0,
        per: PerSelector::CardCountInZone(CardCountInZoneSpec {
            zone: Zone::BattleArea,
            of: digimon_dsl::common::PlayerRef::Opponent,
            filter: Some(Box::new(digimon_dsl::predicate::PredicateSpec {
                kind: Some(digimon_dsl::spec::CardKind::Digimon),
                ..Default::default()
            })),
        }),
        bucket: None,
        delta: 1,
    };
    let filtered_yaml = serde_yml::to_string(&filtered).expect("serialize filtered count");
    assert!(filtered_yaml.contains("card_count_in_zone"));
    assert!(!filtered_yaml.contains("filtered_card_count_in_zone"));
    let reparsed: FormulaSpec =
        serde_yml::from_str(&filtered_yaml).expect("reparse filtered count");
    assert_eq!(reparsed, filtered);
}

#[test]
fn shared_trash_count_bucket_formula_evaluates() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 3000))
        .add_card(digimon_card("TRASH-A", 1000))
        .add_card(digimon_card("TRASH-B", 1000))
        .build();
    let target = runner.place_on_field(0, "SRC", None);
    for i in 0..5 {
        push_card_to_trash(
            &mut runner,
            0,
            if i % 2 == 0 { "TRASH-A" } else { "TRASH-B" },
        );
    }
    for i in 0..7 {
        push_card_to_trash(
            &mut runner,
            1,
            if i % 2 == 0 { "TRASH-A" } else { "TRASH-B" },
        );
    }

    let formula = CompiledFormula::BasePerDelta {
        base: 7000,
        per: CompiledPerSelector::SharedTrashCount { bucket: Some(10) },
        delta: 2000,
    };
    let src_card = runner.game.players[0].battle_area[0].top_card().handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(target), 0);

    assert_eq!(formula_eval::evaluate(&formula, &ctx, target), 9000);
}

#[test]
fn binding_dp_formula_reads_bound_permanent_effective_dp() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 3000))
        .add_card(digimon_card("ALLY", 3000))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    let ally = runner.place_on_field(0, "ALLY", None);
    runner.game.modifiers.add(
        ally,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, 0),
    );

    let formula = CompiledFormula::BindingDp("ally".to_string());
    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("ally", ally);

    assert_eq!(
        formula_eval::evaluate_with_bindings(&formula, &ctx, source, Some(&bindings)),
        6000
    );
}

#[test]
fn filtered_battle_area_count_counts_only_matching_kind() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 3000))
        .add_card(digimon_card("OPP-DIGI", 4000))
        .add_card(tamer_card("OPP-TAMER"))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    runner.place_on_field(1, "OPP-DIGI", None);
    runner.place_on_field(1, "OPP-TAMER", None);

    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::FilteredCardCountInZoneScoped {
            zone: CompiledZone::BattleArea,
            of: CompiledPlayerRef::Opponent,
            filter: Box::new(CompiledPredicate {
                kind: Some(CompiledCardKind::Digimon),
                ..Default::default()
            }),
        },
        delta: 1,
    };
    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);

    assert_eq!(formula_eval::evaluate(&formula, &ctx, source), 1);
}

#[test]
fn filtered_card_zone_count_does_not_count_permanent_only_predicates() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 3000))
        .add_card(digimon_card("TRASH-A", 1000))
        .add_card(digimon_card("TRASH-B", 1000))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    push_card_to_trash(&mut runner, 0, "TRASH-A");
    push_card_to_trash(&mut runner, 0, "TRASH-B");

    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::FilteredCardCountInZoneScoped {
            zone: CompiledZone::Trash,
            of: CompiledPlayerRef::You,
            filter: Box::new(CompiledPredicate {
                dp_lte: Some(CompiledDpConstraint::Literal(3000)),
                ..Default::default()
            }),
        },
        delta: 1,
    };
    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);

    assert_eq!(formula_eval::evaluate(&formula, &ctx, source), 0);
}

#[test]
fn filtered_card_zone_count_allows_mixed_any_and_not_combinators() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 3000))
        .add_card(digimon_card("TRASH-DIGI", 1000))
        .add_card(tamer_card("TRASH-TAMER"))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    push_card_to_trash(&mut runner, 0, "TRASH-DIGI");
    push_card_to_trash(&mut runner, 0, "TRASH-TAMER");

    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);

    let mixed_any = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::FilteredCardCountInZoneScoped {
            zone: CompiledZone::Trash,
            of: CompiledPlayerRef::You,
            filter: Box::new(CompiledPredicate {
                any_of: vec![
                    CompiledPredicate {
                        kind: Some(CompiledCardKind::Digimon),
                        ..Default::default()
                    },
                    CompiledPredicate {
                        dp_lte: Some(CompiledDpConstraint::Literal(3000)),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }),
        },
        delta: 1,
    };
    assert_eq!(formula_eval::evaluate(&mixed_any, &ctx, source), 1);

    let not_unsupported = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::FilteredCardCountInZoneScoped {
            zone: CompiledZone::Trash,
            of: CompiledPlayerRef::You,
            filter: Box::new(CompiledPredicate {
                not: Some(Box::new(CompiledPredicate {
                    dp_lte: Some(CompiledDpConstraint::Literal(3000)),
                    ..Default::default()
                })),
                ..Default::default()
            }),
        },
        delta: 1,
    };
    assert_eq!(formula_eval::evaluate(&not_unsupported, &ctx, source), 2);
}

#[test]
fn modifier_formula_binding_dp_uses_current_step_bindings() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 3000))
        .add_card(digimon_card("ALLY", 3000))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    let ally = runner.place_on_field(0, "ALLY", None);
    runner.game.modifiers.add(
        ally,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, 0),
    );
    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let mut ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);
    let mut bindings = Bindings::new();
    bindings.insert_permanent("target", source);
    bindings.insert_permanent("ally", ally);
    let steps = vec![CompiledStep::AddDpModifier {
        target: CompiledBindingRef::Named("target".to_string()),
        value: CompiledModifierValue::Formula(CompiledFormula::BindingDp("ally".to_string())),
        expiry: "end_of_turn".to_string(),
    }];

    run_steps(&steps, &mut ctx, &mut bindings);

    let read = EffectReadContext::new(&runner.game, src_card, Some(source), 0);
    assert_eq!(runner.game.effective_dp(source), Some(9000));
    assert_eq!(
        formula_eval::evaluate_read_with_bindings(
            &CompiledFormula::BindingDp("ally".to_string()),
            &read,
            source,
            Some(&bindings),
        ),
        6000
    );
}

// ── G-DSL-FORMULA-OWN-LINK-CARD-COUNT (BT25-075 Vulcanusmon) ──────────────
//
// "for each of your link cards, ＜De-Digivolve 1＞ all of your opponent's
// Digimon" — the magnitude is the TOTAL count of link cards across every one of
// the controller's battle-area Digimon (DCGO:
// `card.Owner.GetBattleAreaDigimons().Map(p => p.LinkedCards).Flat().Count()`).

/// A link card fixture — a plain Option, attached via `push_linked_owned`.
fn link_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.dp = None;
    c.level = None;
    c
}

#[test]
fn own_link_card_count_formula_sums_across_all_own_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 1000))
        .add_card(digimon_card("OWN-A", 3000))
        .add_card(digimon_card("OWN-B", 4000))
        .add_card(digimon_card("OPP", 5000))
        .add_card(link_option("LINK"))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    let own_a = runner.place_on_field(0, "OWN-A", None);
    let own_b = runner.place_on_field(0, "OWN-B", None);
    let opp = runner.place_on_field(1, "OPP", None);

    // 2 link cards on OWN-A, 1 on OWN-B, 0 on SRC → total for player 0 = 3.
    // A link card on the OPPONENT's Digimon must NOT be counted for `of: you`.
    runner.push_linked_owned(own_a, "LINK", 0);
    runner.push_linked_owned(own_a, "LINK", 0);
    runner.push_linked_owned(own_b, "LINK", 0);
    runner.push_linked_owned(opp, "LINK", 1);

    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);

    // `base_per_delta` with `own_link_card_count { of: you }`, delta 1 → the
    // raw De-Digivolve-repeat magnitude.
    let you = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::OwnLinkCardCount {
            of: CompiledPlayerRef::You,
        },
        delta: 1,
    };
    assert_eq!(formula_eval::evaluate(&you, &ctx, source), 3);

    // `of: opponent` counts only the single link card on the opponent's board.
    let opponent = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::OwnLinkCardCount {
            of: CompiledPlayerRef::Opponent,
        },
        delta: 1,
    };
    assert_eq!(formula_eval::evaluate(&opponent, &ctx, source), 1);
}

#[test]
fn source_link_card_count_formula_reads_carrier_only() {
    let mut runner = DebugRunner::builder()
        .add_card(digimon_card("SRC", 1000))
        .add_card(digimon_card("OTHER", 3000))
        .add_card(link_option("LINK"))
        .build();
    let source = runner.place_on_field(0, "SRC", None);
    let other = runner.place_on_field(0, "OTHER", None);

    // 2 link cards on the CARRIER (SRC), 5 on another own Digimon — the per-host
    // selector reads only the carrier.
    runner.push_linked_owned(source, "LINK", 0);
    runner.push_linked_owned(source, "LINK", 0);
    for _ in 0..5 {
        runner.push_linked_owned(other, "LINK", 0);
    }

    let src_card = runner.game.players[0].battle_area[source.index as usize]
        .top_card()
        .handle();
    let ctx = EffectContext::new(&mut runner.game, src_card, Some(source), 0);
    let formula = CompiledFormula::BasePerDelta {
        base: 0,
        per: CompiledPerSelector::SourceLinkCardCount,
        delta: 1,
    };
    assert_eq!(formula_eval::evaluate(&formula, &ctx, source), 2);
}

/// End-to-end gate shape: `de_digivolve` consumes `own_link_card_count` as its
/// magnitude (`amount:`). Proves the formula composes into the mass-De-Digivolve
/// clause BT25-075 needs (does NOT author the card).
#[test]
fn de_digivolve_amount_compiles_from_own_link_card_count_formula() {
    let yaml = r#"
card: T-G7-OWN-LINK-DEDIGI
name: Own Link Card Count De-Digivolve
kind: option
color: [black]
cost: 5
effects:
  - when: main_from_hand
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: "Choose 1 opponent Digimon"
      - de_digivolve:
          target: victim
          amount:
            base: 0
            per:
              own_link_card_count: { of: you }
            delta: 1
"#;
    let compiled = compile_yaml(yaml);
    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let dedigi = triggered
        .process
        .iter()
        .find_map(|s| match s {
            CompiledStep::DeDigivolve { amount_fn, .. } => Some(amount_fn),
            _ => None,
        })
        .expect("de_digivolve step present");
    assert!(
        matches!(
            dedigi,
            Some(CompiledFormula::BasePerDelta {
                per: CompiledPerSelector::OwnLinkCardCount {
                    of: CompiledPlayerRef::You
                },
                ..
            })
        ),
        "de_digivolve amount lowered to the own_link_card_count formula, got {dedigi:?}"
    );
}
