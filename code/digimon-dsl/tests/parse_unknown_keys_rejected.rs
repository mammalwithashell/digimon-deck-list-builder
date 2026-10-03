//! Unknown DSL keys are a hard parse error, everywhere.
//!
//! Regression suite for the silently-dropped-key class of bug: `PredicateSpec`
//! cannot carry `#[serde(deny_unknown_fields)]` (it is `#[serde(flatten)]`'d
//! into `MaterialSpec` / `ExistentialPredicate`), and its leftover-key sink
//! (`extra`) was never checked — so an invented key such as
//! `source_permanent_name_contains` (ST24-06 / ST24-10 / BT25-027),
//! `predicate:` inside `any_permanent` (BT13-112 / BT22-036), `color:` on an
//! alt-path `from` (10 cards), `binding:` as a `for_each.over`
//! (BT18-102) or `source: true` as an aura target (EX12-011) became an
//! always-true EMPTY predicate. Untagged enums with struct variants and the
//! one-key step map had the same hole.

use digimon_dsl::predicate::PredicateSpec;
use digimon_dsl::step::StepSpec;
use digimon_dsl::CardSpec;

fn err_of<T: serde::de::DeserializeOwned + std::fmt::Debug>(yaml: &str) -> String {
    match serde_yml::from_str::<T>(yaml) {
        Ok(v) => panic!("expected a parse error, got {v:?}"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn unknown_leaf_predicate_key_is_rejected() {
    let e = err_of::<PredicateSpec>("source_permanent_name_contains: Gaogamon");
    assert!(
        e.contains("unknown predicate key") && e.contains("source_permanent_name_contains"),
        "{e}"
    );
}

#[test]
fn known_predicate_keys_still_parse() {
    let p: PredicateSpec = serde_yml::from_str(
        "any_of:\n  - source_name_contains: Gaogamon\n  - source_permanent_trait_has: DATA SQUAD\n",
    )
    .expect("valid predicate parses");
    assert_eq!(p.any_of.len(), 2);
    assert!(p.extra.is_empty());
}

#[test]
fn unknown_key_nested_in_any_of_is_rejected() {
    let e = err_of::<PredicateSpec>(
        "all_of:\n  - replacement_subject_is_mine: true\n  - any_of:\n      - source_permanent_name_contains: Rosemon\n      - source_permanent_trait_has: DATA SQUAD\n",
    );
    assert!(e.contains("source_permanent_name_contains"), "{e}");
}

#[test]
fn unknown_key_inside_existential_is_rejected() {
    // ExistentialPredicate flattens its predicate — there is no `predicate:` key.
    let e = err_of::<PredicateSpec>("any_permanent: { of: opponent, predicate: { kind: digimon } }");
    assert!(e.contains("predicate"), "{e}");
    let ok: PredicateSpec =
        serde_yml::from_str("any_permanent: { of: opponent, kind: digimon }").expect("flat form parses");
    assert!(ok.any_permanent.is_some());
}

#[test]
fn unknown_key_on_alt_path_from_is_rejected() {
    let yaml = "card: X-1\nname: X\nkind: digimon\nlevel: 5\ncolor: [red]\ncost: 7\ndp: 7000\n\
                alt_paths:\n  - kind: digivolve\n    from: { level_eq: 4, color: red }\n    cost: 3\n";
    let e = err_of::<CardSpec>(yaml);
    assert!(e.contains("unknown predicate key") && e.contains("color"), "{e}");
    let fixed = yaml.replace("color: red }", "color_is: red }");
    serde_yml::from_str::<CardSpec>(&fixed).expect("color_is parses");
}

#[test]
fn unknown_key_on_inline_material_predicate_is_rejected() {
    // MaterialSpec flattens an inline PredicateSpec.
    let yaml = "card: X-2\nname: X\nkind: digimon\nlevel: 6\ncolor: [red]\ncost: 7\ndp: 7000\n\
                alt_paths:\n  - kind: dna_digivolve\n    materials:\n      - { levle_eq: 5 }\n      - { level_eq: 5 }\n    cost: 0\n";
    let e = err_of::<CardSpec>(yaml);
    assert!(e.contains("levle_eq"), "{e}");
}

#[test]
fn step_with_a_sibling_key_is_rejected() {
    let e = err_of::<StepSpec>("draw: { of: you, count: 1 }\noptional: true\n");
    assert!(e.contains("unexpected sibling key") && e.contains("optional"), "{e}");
}

#[test]
fn formula_struct_variant_with_unknown_key_is_rejected() {
    let e = err_of::<digimon_dsl::formula::FormulaSpec>("{ base: 0, per: ally_count, delta: 1, bogus: 2 }");
    assert!(!e.is_empty());
    serde_yml::from_str::<digimon_dsl::formula::FormulaSpec>("{ base: 0, per: ally_count, delta: 1 }")
        .expect("valid formula parses");
}

#[test]
fn cost_delta_reduce_with_unknown_key_is_rejected() {
    let e = err_of::<digimon_dsl::step::CostDelta>("{ reduce: 2, bogus: 1 }");
    assert!(!e.is_empty());
    serde_yml::from_str::<digimon_dsl::step::CostDelta>("{ reduce: 2 }").expect("valid reduce parses");
}

#[test]
fn select_count_capped_multi_on_material_zone_is_a_compile_error() {
    // The runtime only scans hand / trash / battle_area; `zone: material`
    // used to install nothing and silently skip the rest of the slice
    // (BT18-102).
    let yaml = "card: X-4\nname: X\nkind: digimon\nlevel: 6\ncolor: [white]\ncost: 9\ndp: 9000\n\
                effects:\n  - when: when_attacking\n    process:\n      - select_count_capped_multi:\n          of: you\n          zone: material\n          max: 5\n          filter: {}\n          bind_as: placed\n          prompt: p\n";
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parses");
    let errs = digimon_dsl::compile::compile(&spec).expect_err("material zone rejected");
    assert!(
        errs.iter().any(|e| e.message.contains("select_count_capped_multi supports zone")),
        "{errs:?}"
    );
}

#[test]
fn unknown_key_in_declarative_aura_target_is_a_compile_error() {
    let yaml = "card: X-3\nname: X\nkind: digimon\nlevel: 4\ncolor: [red]\ncost: 5\ndp: 5000\n\
                effects:\n  - scope: inherited\n    kind: aura\n    target: { source: true }\n    dp_modifier: 2000\n";
    let spec: CardSpec = serde_yml::from_str(yaml).expect("body is free-form at parse time");
    let errs = digimon_dsl::compile::compile(&spec).expect_err("typed body rejects `source`");
    assert!(
        errs.iter().any(|e| e.message.contains("unknown predicate key")),
        "{errs:?}"
    );
}
