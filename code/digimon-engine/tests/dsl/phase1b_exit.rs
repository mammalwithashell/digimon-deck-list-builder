//! Phase 1b exit criteria — the full YAML→pack→embedded pipeline works
//! end-to-end for all 22 worked examples.

#[test]
fn phase_1b_exit_criteria() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded registry must load");

    // All 22 fixtures round-tripped through the pack.
    assert_eq!(registry.len(), 22);

    // Spot-check a handful of cards end-to-end.
    let st2_13 = registry.lookup("ST2-13").expect("ST2-13 present");
    assert_eq!(st2_13.name, "Hammer Spark");
    assert_eq!(st2_13.kind, digimon_dsl::compiled::CompiledCardKind::Option);
    assert_eq!(st2_13.effects.len(), 2);

    let war_greymon = registry.lookup("BT17-015").expect("BT17-015 present");
    assert_eq!(war_greymon.name, "WarGreymon");
    assert_eq!(war_greymon.level, Some(6));
    assert!(war_greymon.alt_paths.len() >= 1);

    let nokia = registry.lookup("BT22-084").expect("BT22-084 present");
    assert_eq!(nokia.name, "Nokia Shiramine");
    assert_eq!(nokia.kind, digimon_dsl::compiled::CompiledCardKind::Tamer);

    // Manifest is the one embedded at build time.
    assert_eq!(registry.manifest.pack_id, "core");
}
