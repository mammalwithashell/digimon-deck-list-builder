use digimon_engine::cards::raw_rust::raw_rust_budget_status;

#[test]
fn raw_rust_budget_allows_three_percent_or_less() {
    assert!(raw_rust_budget_status(3, 100).is_ok());
    assert!(raw_rust_budget_status(0, 0).is_ok());
}

#[test]
fn raw_rust_budget_flags_above_three_percent() {
    let err = raw_rust_budget_status(4, 100).unwrap_err();
    assert!(err.contains("4 raw_rust"));
    assert!(err.contains("4.0%"));
}
