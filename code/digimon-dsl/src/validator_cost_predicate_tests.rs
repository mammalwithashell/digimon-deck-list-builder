//! The cost-aware evolution leaf has one declarative formula contract in
//! process filters, trigger gates, and declarative filters alike.

use crate::{
    errors::ValidationError,
    raw_rust_registry::StubRegistry,
    validator::{validate, ValidationContext},
    CardSpec,
};

fn document(effect: &str) -> String {
    format!(
        "card: COST-PREDICATE-VALIDATION\nname: Cost predicate validation\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 2000\neffects:\n{effect}\n"
    )
}

fn leaf(cost: &str) -> String {
    format!("{{ can_digivolve_onto_with_cost: {{ target: source, cost: {cost} }} }}")
}

fn hand_count(cost: &str) -> String {
    format!(
        "{{ count_gte: {{ filter: {{ of: you, zone: [hand], all_of: [{}] }}, n: 1 }} }}",
        leaf(cost)
    )
}

fn trigger_gate(field: &str, cost: &str) -> String {
    document(&format!(
        "  - when: on_play\n    {field}: {}\n    process:\n      - gain_memory: 1",
        hand_count(cost)
    ))
}

fn process_filter(cost: &str) -> String {
    document(&format!(
        "  - when: on_play\n    process:\n      - select_hand:\n          of: you\n          filter: {}\n          bind_as: evolution\n          optional: true\n          prompt: Choose a payable evolution",
        leaf(cost)
    ))
}

fn declarative_filter(kind: &str, cost: &str) -> String {
    match kind {
        "aura" => document(&format!(
            "  - kind: aura\n    target: {}\n    dp_modifier: 1000",
            leaf(cost)
        )),
        "cost_reduction" => document(&format!(
            "  - kind: cost_reduction\n    when_playing_this: true\n    amount: 1\n    condition: {}",
            hand_count(cost)
        )),
        _ => unreachable!(),
    }
}

fn checked(yaml: &str) -> Result<(), Vec<ValidationError>> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("fixture must parse as typed card YAML");
    // Registration must not exempt a callback from the stricter read/commit
    // parity contract of this particular predicate.
    let raw = StubRegistry::with(["registered_cost_callback"]);
    validate(&spec, &ValidationContext { raw_rust: &raw })
}

fn assert_error(yaml: &str, path: &str, message: &str) {
    let errors = checked(yaml).expect_err("invalid cost-aware predicate must be rejected");
    assert!(
        errors
            .iter()
            .any(|error| { error.path.contains(path) && error.message.contains(message) }),
        "expected {path:?} / {message:?}, got {errors:?}"
    );
}

#[test]
fn trigger_count_filters_reject_registered_raw_cost_callbacks_in_every_gate() {
    for gate in ["condition", "active_when", "trigger_if"] {
        for cost in [
            "{ reduce: { raw_rust: registered_cost_callback } }",
            "{ reduce: { max: [1, { raw_rust: registered_cost_callback }] } }",
        ] {
            assert_error(
                &trigger_gate(gate, cost),
                &format!("{gate}.count_gte.filter.all_of[0].can_digivolve_onto_with_cost.cost"),
                "declarative cost formulas",
            );
        }
    }
}

#[test]
fn declarative_filters_reject_registered_raw_cost_callbacks() {
    for kind in ["aura", "cost_reduction"] {
        assert_error(
            &declarative_filter(kind, "{ reduce: { raw_rust: registered_cost_callback } }"),
            "can_digivolve_onto_with_cost.cost",
            "declarative cost formulas",
        );
    }
}

#[test]
fn nested_cost_formula_predicates_receive_normal_keyword_validation() {
    for filter in [
        "{has_keyword: UnknownCostKeyword}",
        "{event_added_card_any: {has_keyword: UnknownCostKeyword}}",
    ] {
        let cost =
            format!("{{reduce: {{source_stack_count: {{target: source, filter: {filter}}}}}}}");
        for yaml in [
            trigger_gate("condition", &cost),
            process_filter(&cost),
            declarative_filter("aura", &cost),
        ] {
            assert_error(
                &yaml,
                "can_digivolve_onto_with_cost.cost.reduce.source_stack_count.filter",
                "unknown keyword: UnknownCostKeyword",
            );
        }
    }
}

#[test]
fn valid_declarative_costs_work_in_trigger_process_and_declarative_filters() {
    for cost in [
        "printed",
        "free",
        "2",
        "{ reduce: { source_dp: {} } }",
        "{ reduce: { source_stack_count: { target: source, filter: { has_keyword: Blocker } } } }",
    ] {
        for yaml in [
            trigger_gate("condition", cost),
            trigger_gate("active_when", cost),
            trigger_gate("trigger_if", cost),
            process_filter(cost),
            declarative_filter("aura", cost),
        ] {
            assert!(
                checked(&yaml).is_ok(),
                "valid fixture rejected:\n{yaml}\n{:?}",
                checked(&yaml)
            );
        }
    }
}

#[test]
fn process_target_and_formula_bindings_still_require_an_in_scope_declaration() {
    assert_error(
        &process_filter("printed").replace("target: source", "target: missing_host"),
        "can_digivolve_onto_with_cost.target",
        "missing_host",
    );
    assert_error(
        &process_filter("{ reduce: { binding_value: missing_amount } }"),
        "can_digivolve_onto_with_cost.cost",
        "missing_amount",
    );
}

fn all_additional_contexts_reject(predicate: &str, expected: &str) {
    let mut missed = Vec::new();
    for (name, yaml) in super::candidate_predicate_tests::additional_predicate_contexts(predicate) {
        match checked(&yaml) {
            Err(errors)
                if errors.iter().any(|e| {
                    e.path.contains("can_digivolve_onto_with_cost.cost")
                        && e.message.contains(expected)
                }) => {}
            result => missed.push(format!("{name}: {result:?}")),
        }
    }
    assert!(
        missed.is_empty(),
        "cost validation skipped entry points: {missed:#?}"
    );
}

#[test]
fn cost_additional_entry_points_reject_registered_raw_callbacks() {
    all_additional_contexts_reject(
        &leaf("{reduce: {raw_rust: registered_cost_callback}}"),
        "declarative cost formulas",
    );
}

#[test]
fn cost_additional_entry_points_validate_nested_semantics() {
    all_additional_contexts_reject(
        &leaf("{reduce: {source_stack_count: {target: source, filter: {has_keyword: UnknownCostKeyword}}}}"),
        "unknown keyword: UnknownCostKeyword",
    );
}

#[test]
fn cost_nested_source_filters_retain_formula_binding_scope() {
    let predicate = leaf("{reduce: {binding_value: missing_amount}}");
    let mut missed = Vec::new();
    for (name, yaml) in super::candidate_predicate_tests::additional_binding_contexts(&predicate) {
        match checked(&yaml) {
            Err(errors)
                if errors.iter().any(|e| {
                    e.path.contains("can_digivolve_onto_with_cost.cost")
                        && e.message.contains("missing_amount")
                }) => {}
            result => missed.push(format!("{name}: {result:?}")),
        }
    }
    assert!(
        missed.is_empty(),
        "cost binding validation skipped: {missed:#?}"
    );
}

#[test]
fn cost_predicates_are_rejected_inside_every_reducer_context() {
    let mut missed = Vec::new();
    for cost in ["printed", "free", "0", "{reduce: 1}"] {
        for (name, yaml) in super::candidate_predicate_tests::reducer_contexts(&leaf(cost)) {
            match checked(&yaml) {
                Err(errors)
                    if errors.iter().any(|e| {
                        e.path.contains("can_digivolve_onto_with_cost.cost")
                            && e.message.contains("cannot be used inside cost_reduction")
                    }) => {}
                result => missed.push(format!("{name}, {cost}: {result:?}")),
            }
        }
    }
    assert!(
        missed.is_empty(),
        "recursive evolution probes accepted: {missed:#?}"
    );
}

#[test]
fn cost_bindings_follow_lexical_selector_and_branch_scope() {
    for (name, yaml, valid) in super::candidate_predicate_tests::lexical_cost_contexts(&leaf(
        "{reduce: {binding_value: amount}}",
    )) {
        if valid {
            assert!(checked(&yaml).is_ok(), "{name}: {:?}", checked(&yaml));
        } else {
            assert_error(&yaml, "can_digivolve_onto_with_cost.cost", "amount");
        }
    }
}
