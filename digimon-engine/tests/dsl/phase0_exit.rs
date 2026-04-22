use digimon_engine::dsl::loader::{self, cross_check, CardDataDbStub};
use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::schema::export_json_schema;
use digimon_engine::dsl::spec::{CardKind, CardSpec, ColorSpec};
use digimon_engine::dsl::validator::{validate, ValidationContext};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples")
}

fn build_stub_db() -> CardDataDbStub {
    // Phase 0 ships a hand-crafted stub per card; Phase 1 replaces with a
    // real loader over `digimon_gym/engine/data/cards.json`.
    CardDataDbStub::new()
        .with_card("ST2-13", "Hammer Spark", CardKind::Option, None, None, Some(0), vec![ColorSpec::Red])
        .with_card("BT17-007", "Agumon", CardKind::Digimon, Some(3), Some(2000), Some(3), vec![ColorSpec::Red])
        .with_card("BT22-084", "Nokia Shiramine", CardKind::Tamer, None, None, Some(5), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT5-093", "Tai Kamiya & Matt Ishida", CardKind::Tamer, None, None, Some(4), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT17-015", "WarGreymon", CardKind::Digimon, Some(6), Some(12000), Some(11), vec![ColorSpec::Red])
        .with_card("AD1-025", "Omnimon", CardKind::Digimon, Some(7), Some(13000), Some(15), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT24-016", "Lamiamon", CardKind::Digimon, Some(5), Some(7000), Some(7), vec![ColorSpec::Red])
        .with_card("BT18-019", "Millenniummon", CardKind::Digimon, Some(7), Some(13000), Some(14), vec![ColorSpec::Black])
        .with_card("BT20-083", "Omekamon", CardKind::Digimon, Some(4), Some(4000), Some(5), vec![ColorSpec::Red, ColorSpec::Blue])
        .with_card("BT18-102", "Susanoomon", CardKind::Digimon, Some(7), Some(15000), Some(9),
            vec![ColorSpec::Red, ColorSpec::Blue, ColorSpec::Yellow, ColorSpec::Green, ColorSpec::Black, ColorSpec::Purple])
        .with_card("BT13-060", "Rosemon: Burst Mode", CardKind::Digimon, Some(7), Some(15000), Some(15), vec![ColorSpec::Green])
        .with_card("BT13-007", "King Drasil_7D6", CardKind::DigiEgg, None, None, Some(0), vec![ColorSpec::Yellow])
        .with_card("BT12-112", "Shoutmon X7: Superior Mode", CardKind::Digimon, Some(7), Some(17000), Some(15), vec![ColorSpec::Red])
        .with_card("BT10-111", "Shoutmon (King Version)", CardKind::Digimon, Some(4), Some(4000), Some(5), vec![ColorSpec::Red])
        .with_card("EX11-012", "Medusamon", CardKind::Digimon, Some(6), Some(11000), Some(11), vec![ColorSpec::Purple])
}

#[test]
fn phase_0_exit_criteria() {
    let (specs, errors) = loader::load_dir_ok(&examples_dir());
    assert!(errors.is_empty(), "parse errors: {errors:#?}");
    assert_eq!(specs.len(), 15, "expected exactly 15 examples");

    let reg = StubRegistry::with([
        "bt13_007_royal_knight_cost_reduction",
        "bt10_111_arm_digixros_wildcard_for_turn",
        "ad1_025_on_play_process",
    ]);
    let ctx = ValidationContext { raw_rust: &reg };

    let db = build_stub_db();

    for spec in &specs {
        validate(spec, &ctx).unwrap_or_else(|errs| {
            panic!("validation failed for {}: {:#?}", spec.card, errs)
        });
        cross_check(spec, &db).unwrap_or_else(|e| {
            panic!("cross-check failed for {}: {}", spec.card, e)
        });
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
