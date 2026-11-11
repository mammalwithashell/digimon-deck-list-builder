//! Validation for the combined revealed-zone and optional-cost candidate
//! probe. Omitted cost keeps its established route-only meaning; explicitly
//! supplied costs use the same declarative read-only contract as the
//! single-card can_digivolve_onto_with_cost predicate.
use crate::{
    errors::ValidationError,
    raw_rust_registry::StubRegistry,
    validator::{validate, ValidationContext},
    CardSpec,
};

fn document(effect: &str) -> String {
    format!("card: CANDIDATE-VALIDATION\nname: Candidate Validation\nkind: digimon\nlevel: 3\ncolor: [black]\ncost: 3\ndp: 3000\neffects:\n{effect}\n")
}

fn leaf(zone: &str, cost: Option<&str>) -> String {
    let cost = cost.map(|c| format!(", cost: {c}")).unwrap_or_default();
    format!("{{ has_digivolve_candidate: {{ of: you, zone: [{zone}], filter: {{kind: digimon, trait_has: X Antibody}}{cost} }} }}")
}

fn process_filter(zone: &str, cost: Option<&str>) -> String {
    document(&format!("  - when: on_play\n    process:\n      - select_own_permanent:\n          bind_as: host\n          optional: true\n          filter: {}\n          prompt: Choose a host", leaf(zone, cost)))
}

fn trigger_gate(field: &str, cost: &str) -> String {
    document(&format!("  - when: on_play\n    {field}: {{any_permanent: {{of: you, zone: [battle_area], all_of: [{}]}}}}\n    process:\n      - gain_memory: 1", leaf("reveal", Some(cost))))
}

fn aura_filter(cost: &str) -> String {
    document(&format!(
        "  - kind: aura\n    target: {}\n    dp_modifier: 1000",
        leaf("hand, reveal", Some(cost))
    ))
}

fn aura_gate(cost: &str) -> String {
    document(&format!(
        "  - kind: aura\n    active_when: {}\n    target: {{}}\n    dp_modifier: 1000",
        leaf("trash, reveal", Some(cost))
    ))
}

fn reduction_gate(cost: &str) -> String {
    document(&format!("  - kind: cost_reduction\n    when_playing_this: true\n    amount: 1\n    condition: {{any_permanent: {{of: you, zone: [battle_area], all_of: [{}]}}}}", leaf("hand, trash, reveal", Some(cost))))
}

fn contexts(cost: &str) -> Vec<String> {
    vec![
        process_filter("reveal", Some(cost)),
        trigger_gate("condition", cost),
        trigger_gate("active_when", cost),
        trigger_gate("trigger_if", cost),
        aura_filter(cost),
        aura_gate(cost),
        reduction_gate(cost),
    ]
}

fn checked(yaml: &str) -> Result<(), Vec<ValidationError>> {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("typed candidate fixture must parse");
    // Registering a callback cannot grant the immutable predicate a matching
    // mutable runtime evaluator, so it must still be rejected for costs.
    let raw = StubRegistry::with(["registered_candidate_cost"]);
    validate(&spec, &ValidationContext { raw_rust: &raw })
}

fn error(yaml: &str, path: &str, message: &str) {
    let errors = checked(yaml).expect_err("invalid candidate predicate must fail validation");
    assert!(
        errors
            .iter()
            .any(|e| e.path.contains(path) && e.message.contains(message)),
        "expected {path:?} / {message:?}; got {errors:?}\n{yaml}"
    );
}

#[test]
fn candidate_valid_reveal_and_legacy_zone_combinations_are_accepted() {
    for zones in [
        "hand",
        "trash",
        "reveal",
        "hand, trash",
        "hand, reveal",
        "trash, reveal",
        "hand, trash, reveal",
    ] {
        let yaml = process_filter(zones, None);
        assert!(
            checked(&yaml).is_ok(),
            "zone [{zones}] must validate: {:?}",
            checked(&yaml)
        );
    }
}

#[test]
fn candidate_empty_or_unsupported_zones_still_fail_closed() {
    for zones in [
        "",
        "deck",
        "security",
        "battle_area",
        "breeding",
        "digi_egg_deck",
        "hand, security",
        "reveal, deck",
    ] {
        error(
            &process_filter(zones, None),
            "has_digivolve_candidate.zone",
            "has_digivolve_candidate.zone",
        );
    }
}

#[test]
fn candidate_valid_cost_modes_work_in_trigger_declarative_and_process_contexts() {
    for cost in ["printed", "free", "0", "5", "{reduce: 1}", "{reduce: {source_dp: {}}}",
        "{reduce: {max: [0, {source_stack_count: {target: source, filter: {has_keyword: Blocker}}}]}}"] {
        for yaml in contexts(cost).into_iter().take(6) {
            assert!(checked(&yaml).is_ok(), "valid candidate cost rejected: {:?}\n{yaml}", checked(&yaml));
        }
    }
}

#[test]
fn candidate_registered_raw_cost_callbacks_are_rejected_in_every_context() {
    for cost in [
        "{reduce: {raw_rust: registered_candidate_cost}}",
        "{reduce: {max: [1, {raw_rust: registered_candidate_cost}]}}",
        "{reduce: {source_stack_count: {target: source, filter: {dp_gte: {raw_rust: registered_candidate_cost}}}}}",
    ] {
        for yaml in contexts(cost) {
            error(&yaml, "has_digivolve_candidate.cost", "declarative cost formulas");
        }
    }
}

#[test]
fn candidate_cost_formula_nested_filters_receive_normal_semantic_validation() {
    for filter in [
        "{has_keyword: NotARealCandidateKeyword}",
        "{event_added_card_any: {has_keyword: NotARealCandidateKeyword}}",
    ] {
        let cost =
            format!("{{reduce: {{source_stack_count: {{target: source, filter: {filter}}}}}}}");
        for yaml in contexts(&cost) {
            error(
                &yaml,
                "has_digivolve_candidate.cost.reduce.source_stack_count.filter",
                "unknown keyword: NotARealCandidateKeyword",
            );
        }
    }
}

#[test]
fn candidate_cost_process_formulas_reject_unbound_names_even_nested() {
    for cost in [
        "{reduce: {binding_value: missing_amount}}",
        "{reduce: {max: [1, {binding_value: missing_amount}]}}",
        "{reduce: {source_stack_count: {target: missing_amount}}}",
    ] {
        error(
            &process_filter("hand, reveal", Some(cost)),
            "has_digivolve_candidate.cost",
            "missing_amount",
        );
    }
    let nested = document(&format!("  - when: on_play\n    process:\n      - select_own_permanent:\n          bind_as: host\n          filter: {{all_of: [{}]}}\n          prompt: Choose a host", leaf("reveal", Some("{reduce: {binding_value: missing_amount}}"))));
    error(&nested, "has_digivolve_candidate.cost", "missing_amount");
}

#[test]
fn candidate_cost_process_formulas_accept_previously_declared_bindings() {
    for cost in [
        "{reduce: {binding_value: amount}}",
        "{reduce: {max: [0, {binding_value: amount}]}}",
    ] {
        let yaml = document(&format!("  - when: on_play\n    process:\n      - select_effect_choice:\n          labels: [One, Two]\n          bind_as: amount\n          prompt: Choose an amount\n      - select_own_permanent:\n          bind_as: host\n          filter: {}\n          prompt: Choose a host", leaf("reveal", Some(cost))));
        assert!(
            checked(&yaml).is_ok(),
            "declared formula binding rejected: {:?}\n{yaml}",
            checked(&yaml)
        );
    }
}

#[test]
fn candidate_nested_result_filter_keeps_its_binding_scope_contract() {
    let yaml = process_filter("reveal", None).replace(
        "kind: digimon, trait_has: X Antibody",
        "can_digivolve_onto: missing_host",
    );
    error(
        &yaml,
        "has_digivolve_candidate.filter.can_digivolve_onto",
        "missing_host",
    );
}

/// Shared negative-entry-point matrix for both cost-aware leaves. These
/// fixtures do not claim that every context is a useful authoring idiom;
/// they require semantic validation to visit every parsed predicate/formula.
pub(super) fn additional_predicate_contexts(predicate: &str) -> Vec<(String, String)> {
    let mut cases = vec![
        ("timing_conditions", format!("  - when: [on_play, when_digivolving]\n    timing_conditions:\n      - when: on_play\n        condition: {predicate}\n    process: [{{gain_memory: 1}}]")),
        ("aura.while_condition", format!("  - kind: aura\n    target: {{}}\n    dp_modifier: 1000\n    while_condition: {predicate}")),
        ("cost_reduction.when_any_ally_digivolves_into", format!("  - kind: cost_reduction\n    amount: 1\n    when_any_ally_digivolves_into: {predicate}")),
        ("replacement.cost.link_card_filter", format!("  - kind: replacement\n    trigger: when_would_leave_battle_area\n    optional: true\n    cost: {{trash_own_link_card: true, link_card_filter: {predicate}}}\n    outcome: prevent")),
        ("replacement.cost.return_own_sources_to_deck.filter", format!("  - kind: replacement\n    trigger: when_would_leave_battle_area\n    optional: true\n    cost: {{return_own_sources_to_deck: {{filter: {predicate}, count: 1, position: bottom}}}}\n    outcome: prevent")),
        ("replacement.choose.card_filter", format!("  - kind: replacement\n    trigger: when_would_leave_battle_area\n    optional: true\n    choose: {{from: hand, card_filter: {predicate}, min: 1, max: 1}}\n    outcome: prevent")),
        ("partition.sources", format!("  - kind: partition\n    sources: [{predicate}]")),
        ("partition.process", format!("  - kind: partition\n    sources: [{{}}]\n    process:\n      - if:\n          condition: {predicate}\n          then: [{{gain_memory: 1}}]")),
        ("link_requirement.filter", format!("  - kind: link_requirement\n    cost: 1\n    filter: {predicate}")),
        ("link_condition.filter", format!("  - kind: link_condition\n    cost: 1\n    filter: {predicate}")),
        ("succession.filter", format!("  - kind: succession\n    filter: {predicate}")),
        ("use_cost_increase.amount", format!("  - kind: use_cost_increase\n    amount: {{source_stack_count: {{target: source, filter: {predicate}}}}}")),
        ("alt_path_registration.applies_to", format!("  - kind: alt_path_registration\n    trigger: on_play\n    registers: {{kind: digivolve, from: {{level_eq: 3}}, cost: 2}}\n    applies_to: {predicate}")),
    ].into_iter().map(|(name, effect)| (name.to_owned(), document(&effect))).collect::<Vec<_>>();
    let process = |wrapped: String| {
        document(&format!("  - when: on_play\n    process:\n      - if:\n          condition: {wrapped}\n          then: [{{gain_memory: 1}}]"))
    };
    for child in [
        "event_battle_deleter",
        "event_battle_surviving_deleter",
        "event_added_card_any",
        "event_discarded_card_any",
        "cost_target",
    ] {
        cases.push((
            format!("predicate.{child}"),
            process(format!("{{{child}: {predicate}}}")),
        ));
    }
    for metric in [
        "dp",
        "event_target_dp",
        "play_cost",
        "stack_size",
        "materials_count",
        "security_count",
        "level",
        "event_target_level",
    ] {
        cases.push((format!("metric.{metric}"), process(format!("{{{metric}: [{{op: gte, value: 0}}, {{op: lte, value: {{source_stack_count: {{target: source, filter: {predicate}}}}}}}]}}"))));
    }
    for threshold in [
        "play_cost_gte",
        "opponent_security_count_lte",
        "opponent_security_count_gte",
        "event_target_level_lte",
        "event_target_level_gte",
        "event_target_dp_eq",
        "event_target_dp_lte",
        "event_target_dp_gte",
        "event_card_level_gte",
        "effect_deleted_opponent_digimon_dp_gte",
    ] {
        cases.push((
            format!("threshold.{threshold}"),
            process(format!(
                "{{{threshold}: {{source_stack_count: {{target: source, filter: {predicate}}}}}}}"
            )),
        ));
    }
    cases.extend([
        ("use_requirement".into(), format!("{}use_requirement: {predicate}\n", document("  - when: on_play\n    process: [{gain_memory: 1}]"))),
        ("dual.option.use_requirement".into(), format!("{}dual:\n  digimon: {{level: 3, dp: 3000}}\n  option: {{use_cost: 1, use_requirement: {predicate}}}\n", document("  - when: on_play\n    process: [{gain_memory: 1}]").replace("kind: digimon", "kind: dual"))),
        ("alt_paths.from".into(), format!("{}alt_paths:\n  - kind: digivolve\n    from: {predicate}\n    cost: 2\n", document("  - when: on_play\n    process: [{gain_memory: 1}]"))),
        ("selector.then".into(), document(&format!("  - when: on_play\n    process:\n      - select_own_permanent:\n          bind_as: host\n          filter: {{}}\n          prompt: Choose host\n          then:\n            - if:\n                condition: {predicate}\n                then: [{{gain_memory: 1}}]"))),
        ("select_own_breeding_permanent.filter".into(), document(&format!("  - when: on_play\n    process:\n      - select_own_breeding_permanent:\n          bind_as: host\n          filter: {predicate}\n          prompt: Choose host"))),
    ]);
    cases
}

/// Affordability probes inspect reducers themselves. Even free/fixed-zero
/// requests must not recursively invoke that probe from a reducer clause.
pub(super) fn reducer_contexts(predicate: &str) -> Vec<(String, String)> {
    [
        ("condition", format!("    amount: 1\n    condition: {predicate}")),
        ("active_when", format!("    amount: 1\n    active_when: {predicate}")),
        ("amount", format!("    amount: {{source_stack_count: {{target: source, filter: {predicate}}}}}")),
        ("when_any_ally_played", format!("    amount: 1\n    when_any_ally_played: {predicate}")),
        ("when_any_ally_digivolves_into", format!("    amount: 1\n    when_any_ally_digivolves_into: {predicate}")),
        ("pay_cost.then", format!("    amount: 1\n    pay_cost:\n      - select_own_permanent:\n          filter: {{}}\n          bind_as: host\n          prompt: Choose host\n          then:\n            - if:\n                condition: {predicate}\n                then: [{{gain_memory: 1}}]")),
    ].into_iter().flat_map(|(name, body)| ["cost_reduction", "cost_assignment"].into_iter().map(move |kind| (format!("{kind}.{name}"), document(&format!("  - kind: {kind}\n{body}"))))).collect()
}

/// The same selection binding is usable in its body, but not its filter,
/// a preceding gate, or a sibling branch before it has been declared.
pub(super) fn lexical_cost_contexts(predicate: &str) -> Vec<(String, String, bool)> {
    vec![
        ("bound selector body".into(), document(&format!("  - when: on_play\n    process:\n      - select_own_permanent:\n          filter: {{}}\n          bind_as: amount\n          prompt: Choose amount\n          then:\n            - if:\n                condition: {predicate}\n                then: [{{gain_memory: 1}}]")), true),
        ("own selector filter".into(), document(&format!("  - when: on_play\n    process:\n      - select_own_permanent:\n          filter: {predicate}\n          bind_as: amount\n          prompt: Choose amount")), false),
        ("gate before body declaration".into(), document(&format!("  - when: on_play\n    condition: {predicate}\n    process:\n      - select_effect_choice:\n          labels: [One, Two]\n          bind_as: amount\n          prompt: Choose amount")), false),
        ("sibling else".into(), document(&format!("  - when: on_play\n    process:\n      - if:\n          condition: {{}}\n          then:\n            - select_effect_choice:\n                labels: [One, Two]\n                bind_as: amount\n                prompt: Choose amount\n          else:\n            - if:\n                condition: {predicate}\n                then: [{{gain_memory: 1}}]")), false),
    ]
}

pub(super) fn additional_binding_contexts(predicate: &str) -> Vec<(String, String)> {
    [
        ("source_count.filter", format!("{{source_count: {{filter: {predicate}, at_least: 1}}}}")),
        ("link_card_count.filter", format!("{{link_card_count: {{filter: {predicate}, at_least: 1}}}}")),
        ("distinct_named_count_gte.filter", format!("{{distinct_named_count_gte: {{of: you, filter: {predicate}, n: 1}}}}")),
        ("distinct_colors_gte.filter", format!("{{distinct_colors_gte: {{of: you, filter: {predicate}, n: 1}}}}")),
    ].into_iter().map(|(name, wrapped)| (name.into(), document(&format!("  - when: on_play\n    process:\n      - if:\n          condition: {wrapped}\n          then: [{{gain_memory: 1}}]")))).collect()
}

fn all_additional_contexts_reject(predicate: &str, expected: &str) {
    let mut missed = Vec::new();
    for (name, yaml) in additional_predicate_contexts(predicate) {
        match checked(&yaml) {
            Err(errors)
                if errors.iter().any(|e| {
                    e.path.contains("has_digivolve_candidate.cost") && e.message.contains(expected)
                }) => {}
            result => missed.push(format!("{name}: {result:?}")),
        }
    }
    assert!(
        missed.is_empty(),
        "candidate validation skipped entry points: {missed:#?}"
    );
}

#[test]
fn candidate_cost_additional_entry_points_reject_registered_raw_callbacks() {
    all_additional_contexts_reject(
        &leaf(
            "hand",
            Some("{reduce: {raw_rust: registered_candidate_cost}}"),
        ),
        "declarative cost formulas",
    );
}

#[test]
fn candidate_cost_additional_entry_points_validate_nested_semantics() {
    all_additional_contexts_reject(
        &leaf("hand", Some("{reduce: {source_stack_count: {target: source, filter: {has_keyword: NotARealCandidateKeyword}}}}")),
        "unknown keyword: NotARealCandidateKeyword",
    );
}

#[test]
fn candidate_cost_nested_source_filters_retain_formula_binding_scope() {
    let predicate = leaf("hand", Some("{reduce: {binding_value: missing_amount}}"));
    let mut missed = Vec::new();
    for (name, yaml) in additional_binding_contexts(&predicate) {
        match checked(&yaml) {
            Err(errors)
                if errors.iter().any(|e| {
                    e.path.contains("has_digivolve_candidate.cost")
                        && e.message.contains("missing_amount")
                }) => {}
            result => missed.push(format!("{name}: {result:?}")),
        }
    }
    assert!(
        missed.is_empty(),
        "candidate binding validation skipped: {missed:#?}"
    );
}

#[test]
fn candidate_cost_predicates_are_rejected_inside_every_reducer_context() {
    let mut missed = Vec::new();
    for cost in ["printed", "free", "0", "{reduce: 1}"] {
        for (name, yaml) in reducer_contexts(&leaf("hand", Some(cost))) {
            match checked(&yaml) {
                Err(errors)
                    if errors.iter().any(|e| {
                        e.path.contains("has_digivolve_candidate.cost")
                            && e.message.contains("cannot be used inside cost_reduction")
                    }) => {}
                result => missed.push(format!("{name}, {cost}: {result:?}")),
            }
        }
    }
    assert!(
        missed.is_empty(),
        "recursive candidate probes accepted: {missed:#?}"
    );
}

#[test]
fn candidate_route_only_predicate_remains_allowed_inside_reducers() {
    for (name, yaml) in reducer_contexts(&leaf("hand", None)) {
        assert!(
            checked(&yaml).is_ok(),
            "legacy route-only {name}: {:?}",
            checked(&yaml)
        );
    }
    let nested = format!(
        "{{has_digivolve_candidate: {{of: you, zone: [hand], filter: {}}}}}",
        leaf("hand", Some("free"))
    );
    for (_, yaml) in reducer_contexts(&nested) {
        error(
            &yaml,
            "has_digivolve_candidate.cost",
            "cannot be used inside cost_reduction",
        );
    }
}

#[test]
fn candidate_cost_bindings_follow_lexical_selector_and_branch_scope() {
    for (name, yaml, valid) in
        lexical_cost_contexts(&leaf("hand", Some("{reduce: {binding_value: amount}}")))
    {
        if valid {
            assert!(checked(&yaml).is_ok(), "{name}: {:?}", checked(&yaml));
        } else {
            error(&yaml, "has_digivolve_candidate.cost", "amount");
        }
    }
}
