//! Phase 2g source-selection DSL verbs parse and lower into compiled steps.

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{CompiledBindingRef, CompiledClause, CompiledStep};
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
        } => {
            assert_eq!(dp_budget, 5000);
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
            then,
        } => {
            assert_eq!(bind_as.as_deref(), Some("breeding_target"));
            assert_eq!(prompt, "Choose breeding");
            assert_eq!(then, vec![CompiledStep::GainMemory(1)]);
        }
        other => panic!("expected SelectOwnBreedingPermanent, got {other:?}"),
    }
}
