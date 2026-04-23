use digimon_engine::dsl::formula::{
    AggregateSelector, CompoundFormula, FormulaSpec, PerSelector,
};

fn parse(yaml: &str) -> FormulaSpec {
    serde_yml::from_str(yaml).unwrap()
}

#[test]
fn parse_literal() {
    assert!(matches!(parse("5"), FormulaSpec::Literal(5)));
}

#[test]
fn parse_base_per_delta() {
    let yaml = "base: 15\nper: material_count\ndelta: -1";
    match parse(yaml) {
        FormulaSpec::BasePerDelta { base, per, delta } => {
            assert_eq!((base, delta), (15, -1));
            assert!(matches!(per, PerSelector::MaterialCount));
        }
        _ => panic!("expected BasePerDelta"),
    }
}

#[test]
fn parse_floor_div() {
    match parse("floor_div: [10, 2]") {
        FormulaSpec::Compound(CompoundFormula::FloorDiv(v)) => assert_eq!(v.len(), 2),
        _ => panic!("expected FloorDiv"),
    }
}

#[test]
fn parse_aggregate() {
    match parse("aggregate: lowest_dp") {
        FormulaSpec::Compound(CompoundFormula::Aggregate(AggregateSelector::LowestDp)) => {}
        _ => panic!("expected Aggregate(LowestDp)"),
    }
}

#[test]
fn parse_raw_rust_formula() {
    match parse("raw_rust: my_fn") {
        FormulaSpec::Compound(CompoundFormula::RawRust(n)) => assert_eq!(n, "my_fn"),
        _ => panic!("expected RawRust"),
    }
}
