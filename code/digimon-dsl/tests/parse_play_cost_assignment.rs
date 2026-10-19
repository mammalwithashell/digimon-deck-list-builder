use digimon_dsl::{compile::compile, CardSpec};
fn document(body: &str) -> String {
    format!("card: COST-SCHEMA\nname: Cost schema\nkind: digimon\ncolor: [white]\ncost: 20\ndp: 15000\neffects:\n{body}\n")
}
fn valid(s: &str) -> bool {
    serde_yml::from_str::<CardSpec>(s)
        .ok()
        .is_some_and(|s| compile(&s).is_ok())
}
#[test]
fn typed_assignment_and_prefix_compile_round_trip_without_reduction() {
    let yaml=document("  - kind: cost_assignment\n    when_playing_this: true\n    set_cost: 0\n    pay_cost:\n      - pay_source_prefix:\n          host_filter: {name_is: Mother D-Reaper}\n          minimum: 7\n          optional: true");
    let spec: CardSpec = serde_yml::from_str(&yaml).expect("assignment syntax");
    let compiled = compile(&spec).expect("assignment and exact physical prefix");
    let serialized = serde_json::to_value(&compiled).unwrap();
    let text = serialized.to_string();
    assert!(text.contains("\"set_cost\":0"));
    assert!(text.contains("PaySourcePrefix") || text.contains("pay_source_prefix"));
    let round: CardSpec = serde_yml::from_str(&serde_yml::to_string(&spec).unwrap()).unwrap();
    assert_eq!(compile(&round).unwrap(), compiled);
}
#[test]
fn assignment_rejects_arithmetic_or_digivolution_ambiguity() {
    for extra in [
        "    amount: 3",
        "    amount_fn: {source_material_count: {}}",
        "    when_any_ally_digivolves_into: {}",
    ] {
        assert!(!valid(&document(&format!(
            "  - kind: cost_assignment\n    when_playing_this: true\n    set_cost: 0\n{extra}"
        ))));
    }
}
#[test]
fn physical_prefix_minimum_rejects_zero_and_out_of_literal_domain() {
    for n in [0u64, i64::MAX as u64 + 1, u64::MAX] {
        assert!(!valid(&document(&format!("  - when: on_play\n    process:\n      - pay_source_prefix:\n          host_filter: {{}}\n          minimum: {n}"))));
    }
}
#[test]
fn cost_assignment_unknown_fields_are_not_ignored() {
    assert!(!valid(&document("  - kind: cost_assignment\n    when_playing_this: true\n    set_cost: 0\n    set_cost_if_payment_started: true")));
}
