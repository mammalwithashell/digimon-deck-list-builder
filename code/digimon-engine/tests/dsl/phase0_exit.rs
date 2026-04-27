use digimon_engine::dsl::loader::{self, cross_check};
use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::schema::export_json_schema;
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::validator::{validate, ValidationContext};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples")
}

#[test]
fn phase_0_exit_criteria() {
    let (specs, errors) = loader::load_dir_ok(&examples_dir());
    assert!(errors.is_empty(), "parse errors: {errors:#?}");
    assert!(
        specs.len() >= 1,
        "phase 0 exit: at least 1 example must be present; got {}",
        specs.len()
    );

    let reg = StubRegistry::with([
        "bt13_007_royal_knight_cost_reduction",
        "bt10_111_arm_digixros_wildcard_for_turn",
        "ad1_025_on_play_process",
        "bt9_092_same_level_x_antibody_digivolve",
        "bt15_003_trash_bottom_security",
        "bt7_107_security_add_self_to_hand",
        "bt11_042_when_digivolving_security_search_recovery",
        "bt11_042_your_turn_ladydevimon_or_mirei",
        "ex6_072_main_dna_digivolve_from_field_and_hand",
        "ex6_072_add_self_to_hand",
        "ex11_027_optional_link_maquinamon",
        "ex11_027_link_requirements",
    ]);
    let ctx = ValidationContext { raw_rust: &reg };

    // CARGO_MANIFEST_DIR is `code/digimon-engine`; cards.json lives at the
    // repo root under `data/`, so go up two levels.
    let cards_json = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data/cards.json");
    let db = digimon_engine::dsl_bridge::RealCardDataAdapter::from_path(&cards_json)
        .expect("real cards.json adapter must load");

    for spec in &specs {
        validate(spec, &ctx)
            .unwrap_or_else(|errs| panic!("validation failed for {}: {:#?}", spec.card, errs));
        // Synthetic test cards (`TST-*`) don't exist in the real cards.json
        // and are intentionally excluded from cross-check. They still go
        // through validate + round-trip below.
        if !spec.card.starts_with("TST-") {
            cross_check(spec, &db)
                .unwrap_or_else(|e| panic!("cross-check failed for {}: {}", spec.card, e));
        }
        let printed = format_spec(spec);
        let reparsed: CardSpec = serde_yml::from_str(&printed).unwrap_or_else(|e| {
            panic!("reparse of {} failed: {e}\nprinted:\n{printed}", spec.card)
        });
        assert_eq!(&reparsed, spec, "round-trip mismatch for {}", spec.card);
    }

    let schema = export_json_schema();
    assert!(!schema.is_empty());
    assert!(schema.contains("\"CardSpec\""));
}
