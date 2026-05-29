//! Phase 2g source-selection DSL verbs parse and lower into compiled steps.

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledFormula, CompiledPredicate, CompiledStep,
};
use digimon_dsl::spec::CardSpec;

fn compile_first_step(yaml: &str) -> CompiledStep {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("YAML parse");
    let compiled = compile(&spec).expect("compile");
    let clause = &compiled.effects[0];
    let process = match clause {
        CompiledClause::Triggered(t) => &t.process,
        _ => panic!("expected triggered clause"),
    };
    process[0].clone()
}

#[test]
fn select_own_sources_lowers_with_nested_trash_step() {
    let yaml = r#"
card: X-SRC
name: Source Picker
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - select_own_sources:
          min: 1
          max: 2
          bind_as: picked_sources
          then:
            - trash_selected_sources:
                source_refs: picked_sources
"#;

    match compile_first_step(yaml) {
        CompiledStep::SelectOwnSources {
            target,
            min,
            max,
            bind_as,
            prompt,
            then,
            ..
        } => {
            assert!(target.is_none());
            assert_eq!(min, 1);
            assert_eq!(max, 2);
            assert_eq!(bind_as.as_deref(), Some("picked_sources"));
            assert_eq!(prompt, "Choose source cards");
            assert_eq!(
                then,
                vec![CompiledStep::TrashSelectedSources {
                    source_refs: "picked_sources".to_string(),
                }]
            );
        }
        other => panic!("expected SelectOwnSources, got {other:?}"),
    }
}

#[test]
fn digi_burst_lowers_to_self_source_selection_with_trash_cost_first() {
    let yaml = r#"
card: X-BURST
name: Digi-Burst Helper
kind: digimon
level: 5
color: [black]
cost: 8
dp: 7000
effects:
  - when: main_on_field
    process:
      - digi_burst:
          count: 1
          bind_as: burst_sources
          prompt: "Choose 1 digivolution card under this Digimon"
          then:
            - gain_memory: 2
"#;

    match compile_first_step(yaml) {
        CompiledStep::SelectOwnSources {
            target,
            min,
            max,
            bind_as,
            prompt,
            then,
            ..
        } => {
            assert_eq!(target, Some(CompiledBindingRef::Source));
            assert_eq!(min, 1);
            assert_eq!(max, 1);
            assert_eq!(bind_as.as_deref(), Some("burst_sources"));
            assert_eq!(prompt, "Choose 1 digivolution card under this Digimon");
            assert_eq!(
                then,
                vec![
                    CompiledStep::TrashSelectedSources {
                        source_refs: "burst_sources".to_string(),
                    },
                    CompiledStep::GainMemory(2),
                ]
            );
        }
        other => panic!("expected Digi-Burst to lower into SelectOwnSources, got {other:?}"),
    }
}

#[test]
fn select_own_sources_accepts_host_and_card_filter() {
    let yaml = r#"
card: X-SRC
name: Source Picker
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - scope: inherited
    when: on_opponent_attack
    process:
      - select_own_sources:
          from: source
          filter:
            any_of:
              - trait_has: Mineral
              - trait_has: Rock
          min: 3
          max: 3
          bind_as: picked_sources
          then:
            - trash_selected_sources:
                source_refs: picked_sources
"#;

    match compile_first_step(yaml) {
        CompiledStep::SelectOwnSources {
            target,
            min,
            max,
            bind_as,
            prompt,
            then,
            ..
        } => {
            assert_eq!(target, Some(CompiledBindingRef::Source));
            assert_eq!(min, 3);
            assert_eq!(max, 3);
            assert_eq!(bind_as.as_deref(), Some("picked_sources"));
            assert_eq!(prompt, "Choose source cards");
            assert_eq!(
                then,
                vec![CompiledStep::TrashSelectedSources {
                    source_refs: "picked_sources".to_string(),
                }]
            );
        }
        other => panic!("expected SelectOwnSources, got {other:?}"),
    }
}

#[test]
fn select_opponent_sources_lowers_with_target_and_nested_trash_step() {
    // G-SELECT-OPPONENT-SOURCES — BT16-085 DNA branch shape: pick exactly 3
    // digivolution cards under a chosen opponent Digimon, then trash them.
    let yaml = r#"
card: X-OPP-SRC
name: Opponent Source Picker
kind: tamer
color: [blue]
cost: 4
effects:
  - when: on_digivolve
    process:
      - select_opponent_sources:
          target: opp_stack
          min: 3
          max: 3
          bind_as: trashed_sources
          prompt: "Choose 3 digivolution cards under an opponent Digimon"
          then:
            - trash_selected_sources:
                source_refs: trashed_sources
"#;

    match compile_first_step(yaml) {
        CompiledStep::SelectOpponentSources {
            target,
            filter,
            min,
            max,
            bind_as,
            prompt,
            then,
        } => {
            assert_eq!(target, Some(CompiledBindingRef::Named("opp_stack".into())));
            assert_eq!(filter, CompiledPredicate::default());
            assert_eq!(min, 3);
            assert_eq!(max, 3);
            assert_eq!(bind_as.as_deref(), Some("trashed_sources"));
            assert_eq!(
                prompt,
                "Choose 3 digivolution cards under an opponent Digimon"
            );
            assert_eq!(
                then,
                vec![CompiledStep::TrashSelectedSources {
                    source_refs: "trashed_sources".to_string(),
                }]
            );
        }
        other => panic!("expected SelectOpponentSources, got {other:?}"),
    }
}

#[test]
fn select_opponent_sources_accepts_filter_and_up_to_n() {
    let yaml = r#"
card: X-OPP-SRC2
name: Opponent Source Filter Picker
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - select_opponent_sources:
          filter:
            kind: digimon
          min: 0
          max: 2
          bind_as: picked
          then:
            - trash_selected_sources:
                source_refs: picked
"#;

    match compile_first_step(yaml) {
        CompiledStep::SelectOpponentSources {
            target,
            min,
            max,
            bind_as,
            prompt,
            ..
        } => {
            assert!(target.is_none());
            assert_eq!(min, 0);
            assert_eq!(max, 2);
            assert_eq!(bind_as.as_deref(), Some("picked"));
            assert_eq!(prompt, "Choose source cards");
        }
        other => panic!("expected SelectOpponentSources, got {other:?}"),
    }
}

#[test]
fn select_opponent_sources_rejects_unknown_field() {
    let yaml = r#"
card: X-OPP-BAD
name: Bad Opponent Source Picker
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - select_opponent_sources:
          min: 1
          max: 1
          count: 3
"#;
    let spec: Result<CardSpec, _> = serde_yml::from_str(yaml);
    assert!(
        spec.is_err(),
        "select_opponent_sources must reject the unknown `count` field"
    );
}

#[test]
fn select_opponent_dp_budget_lowers_with_bound_delete_step() {
    let yaml = r#"
card: X-DP
name: DP Picker
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - select_opponent_dp_budget:
          dp_budget: 5000
          min_picks: 1
          bind_as: targets
          prompt: Choose opponents
          then:
            - delete_bound_permanents:
                binding: targets
"#;

    match compile_first_step(yaml) {
        CompiledStep::SelectOpponentDpBudget {
            dp_budget,
            min_picks,
            bind_as,
            prompt,
            then,
            ..
        } => {
            assert_eq!(dp_budget, CompiledFormula::Literal(5000));
            assert_eq!(min_picks, 1);
            assert_eq!(bind_as.as_deref(), Some("targets"));
            assert_eq!(prompt, "Choose opponents");
            assert_eq!(
                then,
                vec![CompiledStep::DeleteBoundPermanents {
                    binding: "targets".to_string(),
                }]
            );
        }
        other => panic!("expected SelectOpponentDpBudget, got {other:?}"),
    }
}

#[test]
fn select_own_breeding_permanent_lowers_with_tail() {
    let yaml = r#"
card: X-BREED
name: Breeding Picker
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - select_own_breeding_permanent:
          bind_as: breeding_target
          prompt: Choose breeding
          then:
            - gain_memory: 1
"#;

    match compile_first_step(yaml) {
        CompiledStep::SelectOwnBreedingPermanent {
            bind_as,
            prompt,
            filter,
            then,
            ..
        } => {
            assert_eq!(bind_as.as_deref(), Some("breeding_target"));
            assert_eq!(prompt, "Choose breeding");
            assert_eq!(filter, CompiledPredicate::default());
            assert_eq!(then, vec![CompiledStep::GainMemory(1)]);
        }
        other => panic!("expected SelectOwnBreedingPermanent, got {other:?}"),
    }
}
