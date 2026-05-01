use digimon_engine::dsl::loader::{cross_check, CardDataDbStub};
use digimon_engine::dsl::spec::{CardKind, CardSpec, ColorSpec};

fn spec_with_overrides(card: &str, name: &str, kind: CardKind, cost: Option<i32>) -> CardSpec {
    CardSpec {
        card: card.into(),
        name: name.into(),
        kind,
        level: None,
        color: vec![ColorSpec::Red],
        cost,
        dp: None,
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![],
        spec_version: None,
    }
}

#[test]
fn cross_check_matches() {
    let db = CardDataDbStub::new().with_card(
        "ST2-13",
        "Hammer Spark",
        CardKind::Option,
        None,
        None,
        Some(0),
        vec![ColorSpec::Red],
    );
    let spec = spec_with_overrides("ST2-13", "Hammer Spark", CardKind::Option, Some(0));
    assert!(cross_check(&spec, &db).is_ok());
}

#[test]
fn cross_check_mismatched_name() {
    let db = CardDataDbStub::new().with_card(
        "ST2-13",
        "Hammer Spark",
        CardKind::Option,
        None,
        None,
        Some(0),
        vec![ColorSpec::Red],
    );
    let spec = spec_with_overrides("ST2-13", "Wrong Name", CardKind::Option, Some(0));
    let err = cross_check(&spec, &db).unwrap_err();
    assert!(err.to_string().contains("name"));
}

#[test]
fn cross_check_mismatched_kind() {
    let db = CardDataDbStub::new().with_card(
        "ST2-13",
        "Hammer Spark",
        CardKind::Option,
        None,
        None,
        Some(0),
        vec![ColorSpec::Red],
    );
    let spec = spec_with_overrides("ST2-13", "Hammer Spark", CardKind::Digimon, Some(0));
    let err = cross_check(&spec, &db).unwrap_err();
    assert!(err.to_string().contains("kind"));
}

#[test]
fn cross_check_card_id_not_found() {
    let db = CardDataDbStub::new();
    let spec = spec_with_overrides("NOPE-000", "Ghost", CardKind::Option, Some(0));
    let err = cross_check(&spec, &db).unwrap_err();
    assert!(err.to_string().contains("unknown card_id"));
}
