//! Phase 2 Track B — DSL surface tests for the `activation_cost:` step.
//!
//! Parse + compile + lowering coverage for the new step that lifts
//! "by suspending this Tamer" / "by returning this Tamer to the bottom
//! of the deck" cost-gates onto `EffectBuilder::activation_cost(...)`.

use digimon_engine::dsl::compile::compile;
use digimon_engine::dsl::compiled::{
    CompiledActivationCostKind, CompiledClause, CompiledStep,
};
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::step::StepSpec;

fn parse_card(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).expect("test YAML parses")
}

const SUSPEND_SELF_BODY: &str = r#"
card: TEST-AC-001
name: SuspendSelfCostCard
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - activation_cost: { suspend_self: true }
      - draw: { of: you, count: 1 }
      - gain_memory: 1
"#;

const RETURN_SELF_BODY: &str = r#"
card: TEST-AC-002
name: ReturnSelfCostCard
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: start_of_your_main_phase
    optional: true
    process:
      - activation_cost: { return_self_to_deck_bottom: true }
      - draw: { of: you, count: 2 }
"#;

#[test]
fn parse_activation_cost_suspend_self_step() {
    let spec = parse_card(SUSPEND_SELF_BODY);
    let triggered = spec
        .effects
        .first()
        .and_then(|c| c.as_triggered())
        .expect("triggered clause present");
    let first = triggered
        .process
        .first()
        .expect("at least one process step");
    match first {
        StepSpec::ActivationCost(args) => {
            assert!(args.suspend_self, "suspend_self should round-trip");
            assert!(
                !args.return_self_to_deck_bottom,
                "return_self_to_deck_bottom should default to false"
            );
        }
        other => panic!("expected ActivationCost step, got {:?}", other),
    }
}

#[test]
fn parse_activation_cost_return_self_step() {
    let spec = parse_card(RETURN_SELF_BODY);
    let triggered = spec
        .effects
        .first()
        .and_then(|c| c.as_triggered())
        .expect("triggered clause present");
    match &triggered.process[0] {
        StepSpec::ActivationCost(args) => {
            assert!(args.return_self_to_deck_bottom);
            assert!(!args.suspend_self);
        }
        other => panic!("expected ActivationCost step, got {:?}", other),
    }
}

#[test]
fn compile_activation_cost_lowers_to_suspend_self_kind() {
    let spec = parse_card(SUSPEND_SELF_BODY);
    let compiled = compile(&spec).expect("activation_cost suspend_self: true compiles cleanly");
    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("compiled triggered clause");
    match &triggered.process[0] {
        CompiledStep::ActivationCost { kind } => {
            assert_eq!(*kind, CompiledActivationCostKind::SuspendSelf);
        }
        other => panic!("expected CompiledStep::ActivationCost, got {:?}", other),
    }
}

#[test]
fn compile_activation_cost_lowers_to_return_self_kind() {
    let spec = parse_card(RETURN_SELF_BODY);
    let compiled = compile(&spec).expect("should compile cleanly");
    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("compiled triggered clause");
    match &triggered.process[0] {
        CompiledStep::ActivationCost { kind } => {
            assert_eq!(*kind, CompiledActivationCostKind::ReturnSelfToDeckBottom);
        }
        other => panic!("expected CompiledStep::ActivationCost, got {:?}", other),
    }
}

fn collect_errors(yaml: &str) -> Vec<String> {
    let spec = parse_card(yaml);
    match compile(&spec) {
        Ok(_) => Vec::new(),
        Err(errs) => errs.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn validator_rejects_activation_cost_mid_body() {
    let yaml = r#"
card: TEST-AC-003
name: BadMidBody
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - draw: { of: you, count: 1 }
      - activation_cost: { suspend_self: true }
"#;
    let messages = collect_errors(yaml);
    assert!(
        !messages.is_empty(),
        "activation_cost must not be allowed mid-body — validator must reject"
    );
    let joined = messages.join("\n");
    assert!(
        joined.contains("activation_cost must be the first step"),
        "validator error must explain the placement rule; got: {}",
        joined
    );
}

#[test]
fn validator_rejects_activation_cost_with_no_kind_set() {
    let yaml = r#"
card: TEST-AC-004
name: NoKind
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - activation_cost: {}
      - draw: { of: you, count: 1 }
"#;
    let messages = collect_errors(yaml);
    assert!(
        !messages.is_empty(),
        "activation_cost with no kind set must be rejected"
    );
    let joined = messages.join("\n");
    assert!(
        joined.contains("activation_cost requires exactly one cost kind"),
        "got: {}",
        joined
    );
}

#[test]
fn validator_rejects_activation_cost_with_both_kinds_set() {
    let yaml = r#"
card: TEST-AC-005
name: BothKinds
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - activation_cost: { suspend_self: true, return_self_to_deck_bottom: true }
      - draw: { of: you, count: 1 }
"#;
    let messages = collect_errors(yaml);
    assert!(
        !messages.is_empty(),
        "activation_cost with both suspend_self + return_self_to_deck_bottom must be rejected"
    );
    let joined = messages.join("\n");
    assert!(joined.contains("mutually exclusive"), "got: {}", joined);
}
