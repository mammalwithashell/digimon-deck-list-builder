use digimon_engine::dsl::loader::{cross_check, CardDataDb};
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl_bridge::RealCardDataAdapter;
use std::path::PathBuf;

fn cards_json_path() -> PathBuf {
    // CARGO_MANIFEST_DIR is `code/digimon-engine`; cards.json lives at the
    // repo root under `data/`, so go up two levels.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data/cards.json")
}

#[test]
fn real_adapter_loads_cards_json() {
    let adapter = RealCardDataAdapter::from_path(&cards_json_path()).expect("cards.json must load");
    assert!(adapter.lookup("ST2-13").is_some());
    assert!(adapter.lookup("BT17-015").is_some());
}

#[test]
fn real_adapter_cross_checks_st2_13_fixture() {
    // ST2-13 was promoted from cards/_examples/ to the per-set folder on
    // 2026-05-20 (DNA Omnimon missing-card authoring pass).
    let yaml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/st2/ST2-13.yaml"),
    )
    .unwrap();
    let spec: CardSpec = serde_yml::from_str(&yaml).unwrap();
    let adapter = RealCardDataAdapter::from_path(&cards_json_path()).unwrap();
    cross_check(&spec, &adapter).expect("ST2-13 fixture must cross-check against real cards.json");
}

#[test]
fn real_adapter_all_fixtures_cross_check() {
    // Cross-checking every fixture against cards.json is stack-heavy on the
    // default Windows libtest thread. Keep the mitigation local to this guard
    // instead of requiring a process-wide RUST_MIN_STACK override.
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(check_real_adapter_all_fixtures_cross_check)
        .expect("spawn large-stack thread")
        .join()
        .expect("real cards cross-check guard thread panicked");
}

fn check_real_adapter_all_fixtures_cross_check() {
    use digimon_engine::dsl::loader;
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let (specs, errs) = loader::load_dir_ok(&examples);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let adapter = RealCardDataAdapter::from_path(&cards_json_path()).unwrap();
    let mut failures = Vec::new();
    for spec in &specs {
        // Synthetic test cards (`TST-*`) don't exist in the real cards.json
        // and are intentionally excluded from cross-check.
        if spec.card.starts_with("TST-") {
            continue;
        }
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
