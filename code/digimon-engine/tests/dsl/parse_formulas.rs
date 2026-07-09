use digimon_engine::dsl::formula::{
    AggregateSelector, CardCountInZoneSpec, CompoundFormula, FormulaSpec, PerSelector,
};
use digimon_engine::dsl::{predicate::Zone, PlayerRef};

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
        FormulaSpec::BasePerDelta {
            base, per, delta, ..
        } => {
            assert_eq!((base, delta), (15, -1));
            assert!(matches!(per, PerSelector::MaterialCount));
        }
        _ => panic!("expected BasePerDelta"),
    }
}

#[test]
fn parse_base_per_card_count_in_zone() {
    let yaml = r#"
base: 0
per:
  card_count_in_zone:
    of: opponent
    zone: trash
delta: 1
"#;
    match parse(yaml) {
        FormulaSpec::BasePerDelta {
            base, per, delta, ..
        } => {
            assert_eq!((base, delta), (0, 1));
            assert_eq!(
                per,
                PerSelector::CardCountInZone(CardCountInZoneSpec {
                    of: PlayerRef::Opponent,
                    zone: Zone::Trash,
                    filter: None,
                })
            );
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

// G-DSL-FORMULA-OWN-LINK-CARD-COUNT — total link cards across all own Digimon.
#[test]
fn parse_base_per_own_link_card_count() {
    let yaml = r#"
base: 0
per:
  own_link_card_count:
    of: you
delta: 1
"#;
    match parse(yaml) {
        FormulaSpec::BasePerDelta {
            base, per, delta, ..
        } => {
            assert_eq!((base, delta), (0, 1));
            assert_eq!(per, PerSelector::OwnLinkCardCount { of: PlayerRef::You });
        }
        _ => panic!("expected BasePerDelta"),
    }
}

#[test]
fn round_trip_own_link_card_count() {
    let per = PerSelector::OwnLinkCardCount { of: PlayerRef::You };
    let json = serde_json::to_value(&per).unwrap();
    let back: PerSelector = serde_json::from_value(json).unwrap();
    assert_eq!(per, back);
}

// G-DSL-LINK-N-CARDS-PER-HOST (formula facet) — per-host link-card count.
#[test]
fn parse_base_per_source_link_card_count() {
    let yaml = r#"
base: 0
per: source_link_card_count
delta: 1
"#;
    match parse(yaml) {
        FormulaSpec::BasePerDelta { per, .. } => {
            assert_eq!(per, PerSelector::SourceLinkCardCount);
        }
        _ => panic!("expected BasePerDelta"),
    }
}

#[test]
fn round_trip_source_link_card_count() {
    let per = PerSelector::SourceLinkCardCount;
    let json = serde_json::to_value(&per).unwrap();
    let back: PerSelector = serde_json::from_value(json).unwrap();
    assert_eq!(per, back);
}
