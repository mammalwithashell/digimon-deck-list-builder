use digimon_engine::dsl::loader::{cross_check, CardDataDb};
use digimon_engine::dsl_bridge::RealCardDataAdapter;
use digimon_engine::dsl::spec::CardSpec;
use std::path::PathBuf;

fn cards_json_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("data/cards.json")
}

#[test]
fn real_adapter_loads_cards_json() {
    let adapter = RealCardDataAdapter::from_path(&cards_json_path())
        .expect("cards.json must load");
    assert!(adapter.lookup("ST2-13").is_some());
    assert!(adapter.lookup("BT17-015").is_some());
}

#[test]
fn real_adapter_cross_checks_st2_13_fixture() {
    let yaml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples/ST2-13.yaml"),
    )
    .unwrap();
    let spec: CardSpec = serde_yml::from_str(&yaml).unwrap();
    let adapter = RealCardDataAdapter::from_path(&cards_json_path()).unwrap();
    cross_check(&spec, &adapter)
        .expect("ST2-13 fixture must cross-check against real cards.json");
}

#[test]
fn real_adapter_all_fixtures_cross_check() {
    use digimon_engine::dsl::loader;
    let examples =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let (specs, errs) = loader::load_dir_ok(&examples);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let adapter = RealCardDataAdapter::from_path(&cards_json_path()).unwrap();
    let mut failures = Vec::new();
    for spec in &specs {
        if let Err(e) = cross_check(spec, &adapter) {
            failures.push(format!("{}: {e}", spec.card));
        }
    }
    assert!(
        failures.is_empty(),
        "fixture cross-check failures:\n{}",
        failures.join("\n")
    );
}
