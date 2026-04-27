use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledPredicate,
    CompiledScope, CompiledStep, CompiledZone,
};
use digimon_dsl::spec::{CardKind, CardSpec};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::enums::{EffectTiming, Zone};
use digimon_engine::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};
use std::sync::Arc;

fn compile_replacement_process(yaml_process: &str) -> Vec<CompiledStep> {
    let yaml = format!(
        r#"
card: TEST-REDUCER
name: Reducer Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    process:
{yaml_process}
"#
    );
    let spec: CardSpec = serde_yml::from_str(&yaml).expect("parse replacement YAML");
    assert_eq!(spec.kind, CardKind::Digimon);
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile replacement YAML");
    match &compiled.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { process, .. }) => {
            process.clone()
        }
        other => panic!("expected replacement clause, got {other:?}"),
    }
}

#[test]
fn replacement_outcome_verbs_parse_and_compile() {
    let process = compile_replacement_process(
        r#"      - cancel_leave: {}
      - handle_replacement: {}
      - redirect_replacement: { destination: deck }
      - substitute_permanent: { target: source }
"#,
    );

    assert!(matches!(process[0], CompiledStep::CancelLeave));
    assert!(matches!(process[1], CompiledStep::HandleReplacement));
    assert!(matches!(
        process[2],
        CompiledStep::RedirectReplacement {
            destination: CompiledZone::Deck
        }
    ));
    assert!(matches!(
        process[3],
        CompiledStep::SubstitutePermanent { .. }
    ));
}

fn compiled_replacement_card(
    active_when: Option<CompiledPredicate>,
    process: Vec<CompiledStep>,
) -> CompiledCard {
    CompiledCard {
        card: "TEST-REDUCER".into(),
        name: "Reducer Test".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![],
        cost: Some(3),
        dp: Some(1000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::Replacement {
                scope: CompiledScope::FaceUp,
                active_when,
                trigger: "when_would_be_deleted".into(),
                process,
                summary: None,
                summary_key: None,
            },
        )],
    }
}

fn runner_with_replacement(
    active_when: Option<CompiledPredicate>,
    process: Vec<CompiledStep>,
) -> (DebugRunner, digimon_engine::permanent::PermanentHandle) {
    let card = compiled_replacement_card(active_when, process);
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TEST-REDUCER", "Reducer Test"))
        .start();
    runner.register_effect("TEST-REDUCER", Arc::new(DslCardEffect::new(Arc::new(card))));
    let target = runner.place_on_field(0, "TEST-REDUCER", Some(0));
    (runner, target)
}

#[test]
fn replacement_process_cancel_leave_sets_cancelled_outcome() {
    let process = compile_replacement_process("      - cancel_leave: {}\n");
    let (mut runner, target) = runner_with_replacement(None, process);

    let outcome = runner.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(target),
        ReplacementCause::OpponentEffect,
        Some(Zone::Trash),
    );

    assert_eq!(outcome, ReplacementOutcome::Cancelled);
}

#[test]
fn replacement_active_when_false_suppresses_candidate() {
    let process = compile_replacement_process("      - cancel_leave: {}\n");
    let never = CompiledPredicate {
        your_turn: Some(false),
        opponents_turn: Some(false),
        ..Default::default()
    };
    let (mut runner, target) = runner_with_replacement(Some(never), process);

    let outcome = runner.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(target),
        ReplacementCause::OpponentEffect,
        Some(Zone::Trash),
    );

    assert_eq!(outcome, ReplacementOutcome::None);
}
