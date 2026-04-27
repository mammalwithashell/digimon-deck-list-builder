#[test]
fn embedded_registry_loads_examples() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded cards.pack must load");
    assert!(
        registry.len() >= 1,
        "embedded pack must contain at least 1 example card; got {}",
        registry.len()
    );
    assert!(registry.lookup("ST2-13").is_some());
    assert!(registry.lookup("BT17-015").is_some());
    assert!(registry.lookup("EX11-012").is_some());
    assert!(registry.lookup("BT9-092").is_some());
    assert!(registry.lookup("BT15-003").is_some());
    assert!(registry.lookup("EX11-027").is_some());
    assert!(registry.lookup("TST-DNA-TRIGGER").is_some());
}

#[test]
fn embedded_registry_manifest_declares_pack_id() {
    let registry = digimon_engine::dsl_registry::from_embedded().unwrap();
    assert_eq!(registry.manifest.pack_id, "core");
}
