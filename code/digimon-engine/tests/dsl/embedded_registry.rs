#[test]
fn embedded_registry_loads_all_16_examples() {
    let registry = digimon_engine::dsl_registry::from_embedded()
        .expect("embedded cards.pack must load");
    assert_eq!(registry.len(), 16, "expected 16 examples in embedded pack");
    assert!(registry.lookup("ST2-13").is_some());
    assert!(registry.lookup("BT17-015").is_some());
    assert!(registry.lookup("EX11-012").is_some());
    assert!(registry.lookup("TST-DNA-TRIGGER").is_some());
}

#[test]
fn embedded_registry_manifest_declares_pack_id() {
    let registry = digimon_engine::dsl_registry::from_embedded().unwrap();
    assert_eq!(registry.manifest.pack_id, "core");
}
