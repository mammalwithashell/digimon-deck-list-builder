#[test]
fn embedded_registry_loads_all_22_examples() {
    let registry =
        digimon_engine::dsl_registry::from_embedded().expect("embedded cards.pack must load");
    assert_eq!(registry.len(), 22, "expected 22 examples in embedded pack");
    assert!(registry.lookup("ST2-13").is_some());
    assert!(registry.lookup("BT17-015").is_some());
    assert!(registry.lookup("EX11-012").is_some());
    assert!(registry.lookup("BT9-092").is_some());
    assert!(registry.lookup("BT15-003").is_some());
    assert!(registry.lookup("EX11-027").is_some());
}

#[test]
fn embedded_registry_manifest_declares_pack_id() {
    let registry = digimon_engine::dsl_registry::from_embedded().unwrap();
    assert_eq!(registry.manifest.pack_id, "core");
}
