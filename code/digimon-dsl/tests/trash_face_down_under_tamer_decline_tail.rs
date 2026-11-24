//! `trash_bottom_face_down_source_under_tamer` with `continue_on_decline` and
//! `bind_as` (G-OPTIONAL-TRASH-FACE-DOWN-UNDER-TAMER-DECLINE-TAIL): a declined
//! Tamer pick keeps running the clause with the binding unbound, so the clause
//! can `refund_opt` on the decline (DCGO `canNoSelect` + `RemoveUse()`;
//! BT26-044 Lilamon, EX13-032 Chirinmon; owner ruling 2026-11-23).

use digimon_dsl::{
    compile::compile,
    compiled::{CompiledClause, CompiledDeclarativeClause, CompiledStep},
    pack::CardPack,
    spec::CardSpec,
};

fn spec(step: &str, tail: &str) -> CardSpec {
    serde_yml::from_str(&format!(
        "card: TEST-TAMER-DECLINE\nname: Test\nkind: digimon\ncolor: [green]\nlevel: 5\ncost: 6\ndp: 7000\neffects:\n  - kind: replacement\n    trigger: when_would_leave_battle_area\n    optional: true\n    once_per_turn: true\n    process:\n      - trash_bottom_face_down_source_under_tamer:\n{step}{tail}"
    ))
    .unwrap()
}

const DECLINE_TAIL: &str = "      - if:\n          condition: { binding_present: paid_tamer }\n          then:\n            - cancel_replacement: {}\n          else:\n            - refund_opt: {}\n";

fn process(card: &digimon_dsl::compiled::CompiledCard) -> &[CompiledStep] {
    match &card.effects[0] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { process, .. }) => {
            process
        }
        other => panic!("replacement expected, got {other:?}"),
    }
}

#[test]
fn decline_tail_fields_compile_and_survive_a_pack_roundtrip() {
    let card = compile(&spec(
        "          of: you\n          optional: true\n          continue_on_decline: true\n          bind_as: paid_tamer\n",
        DECLINE_TAIL,
    ))
    .unwrap();
    match &process(&card)[0] {
        CompiledStep::TrashBottomFaceDownSourceUnderTamer {
            optional,
            continue_on_decline,
            bind_as,
            ..
        } => {
            assert!(*optional);
            assert!(*continue_on_decline);
            assert_eq!(bind_as.as_deref(), Some("paid_tamer"));
        }
        other => panic!("unexpected first step {other:?}"),
    }
    let pack = CardPack::new("decline-tail", vec![card]);
    assert_eq!(CardPack::from_bytes(&pack.to_bytes().unwrap()).unwrap(), pack);
}

#[test]
fn omitted_fields_keep_the_old_shape() {
    let card = compile(&spec("          of: you\n", "      - cancel_replacement: {}\n")).unwrap();
    match &process(&card)[0] {
        CompiledStep::TrashBottomFaceDownSourceUnderTamer {
            optional,
            continue_on_decline,
            bind_as,
            ..
        } => {
            assert!(!*optional);
            assert!(!*continue_on_decline);
            assert!(bind_as.is_none());
        }
        other => panic!("unexpected first step {other:?}"),
    }
}

#[test]
fn continue_on_decline_requires_an_optional_pick() {
    let errors = compile(&spec(
        "          of: you\n          continue_on_decline: true\n          bind_as: paid_tamer\n",
        DECLINE_TAIL,
    ))
    .unwrap_err();
    assert!(
        errors.iter().any(|e| e.path.ends_with("continue_on_decline")),
        "{errors:?}"
    );
}

/// The same step in a triggered clause, whose process the validator checks for
/// bindings used before they are declared; the `then` branch names the bound
/// Tamer as a step target.
fn triggered_spec(step: &str) -> CardSpec {
    serde_yml::from_str(&format!(
        "card: TEST-TAMER-DECLINE-T\nname: Test\nkind: digimon\ncolor: [yellow]\nlevel: 5\ncost: 7\ndp: 7000\neffects:\n  - when: when_attacking\n    once_per_turn: true\n    optional: true\n    outer_prompt: true\n    process:\n      - trash_bottom_face_down_source_under_tamer:\n{step}      - if:\n          condition: {{ binding_present: paid_tamer }}\n          then:\n            - suspend: {{ target: paid_tamer }}\n          else:\n            - refund_opt: {{}}\n"
    ))
    .unwrap()
}

fn validate(spec: &CardSpec) -> Result<(), Vec<digimon_dsl::errors::ValidationError>> {
    digimon_dsl::validator::validate(
        spec,
        &digimon_dsl::validator::ValidationContext {
            raw_rust: &digimon_dsl::raw_rust_registry::StubRegistry::empty(),
        },
    )
}

#[test]
fn the_bound_tamer_is_visible_to_later_binding_predicates() {
    let declared = triggered_spec(
        "          of: you\n          optional: true\n          continue_on_decline: true\n          bind_as: paid_tamer\n",
    );
    compile(&declared).expect("compiles");
    if let Err(errors) = validate(&declared) {
        assert!(
            !errors.iter().any(|e| e.message.contains("undeclared binding")),
            "`bind_as` declares the binding the later steps read: {errors:?}"
        );
    }
    // Without the `bind_as`, `paid_tamer` is an undeclared binding.
    let errors = validate(&triggered_spec(
        "          of: you\n          optional: true\n          continue_on_decline: true\n",
    ))
    .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("undeclared binding") && e.message.contains("paid_tamer")),
        "{errors:?}"
    );
}
