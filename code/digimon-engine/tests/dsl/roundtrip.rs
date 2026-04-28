use digimon_engine::dsl::loader;
use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::validator::{validate, ValidationContext};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples")
}

fn registry_for_examples() -> StubRegistry {
    StubRegistry::with([
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
        "ex11_012_return_trash_to_deck_bottom",
    ])
}

#[test]
fn every_example_parses() {
    let (loaded, errors) = loader::load_dir_ok(&examples_dir());
    assert!(errors.is_empty(), "parse errors: {:#?}", errors);
    assert!(
        loaded.len() >= 1,
        "roundtrip: at least 1 example must round-trip; got {}",
        loaded.len()
    );
}

#[test]
fn every_example_validates() {
    let (specs, _) = loader::load_dir_ok(&examples_dir());
    let reg = registry_for_examples();
    let ctx = ValidationContext { raw_rust: &reg };
    let mut failures = Vec::new();
    for spec in &specs {
        if let Err(errs) = validate(spec, &ctx) {
            for e in errs {
                failures.push(format!("{}: {}", spec.card, e));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "validation failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_example_round_trips() {
    let (specs, _) = loader::load_dir_ok(&examples_dir());
    for spec in specs {
        let formatted = format_spec(&spec);
        let reparsed: CardSpec = serde_yml::from_str(&formatted).unwrap_or_else(|e| {
            panic!(
                "{} failed to reparse:\n{}\nerror: {}",
                spec.card, formatted, e
            )
        });
        assert_eq!(reparsed, spec, "round-trip mismatch for {}", spec.card);
    }
}
