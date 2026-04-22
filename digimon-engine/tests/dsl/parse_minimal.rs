#[test]
fn dsl_module_loads() {
    // Sanity check: the feature-gated module is reachable from tests.
    let _ = digimon_engine::dsl::ValidationError {
        card_id: "X".into(),
        path: "y".into(),
        message: "z".into(),
    };
}
