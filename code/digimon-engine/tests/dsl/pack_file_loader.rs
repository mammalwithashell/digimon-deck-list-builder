//! Engine-side from_pack_file round-trip test — simulates the cache-dir
//! flow that runtime-downloaded packs will use.

#[test]
fn round_trip_registry_through_temp_pack_file() {
    use digimon_dsl::{loader, CardPack};
    use std::path::PathBuf;

    // Load fixtures fresh.
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let (specs, errs) = loader::load_dir_ok(&examples);
    assert!(errs.is_empty(), "fixture parse errors: {errs:#?}");

    let registry =
        digimon_dsl::CardRegistry::from_specs("test-pack", &specs).expect("compile fixtures");
    let pack = CardPack {
        manifest: registry.manifest.clone(),
        cards: registry.iter().map(|(_, c)| c.clone()).collect(),
    };

    // Write to a temp file.
    let mut temp = std::env::temp_dir();
    temp.push("digimon-pack-file-loader-test.pack");
    let bytes = pack.to_bytes().expect("serialize pack");
    std::fs::write(&temp, &bytes).expect("write temp pack");

    // Load via engine-side wrapper.
    let loaded = digimon_engine::dsl_registry::from_pack_file(&temp).expect("from_pack_file");
    assert_eq!(loaded.len(), registry.len());
    assert_eq!(loaded.manifest.pack_id, "test-pack");
    assert!(loaded.lookup("ST2-13").is_some());

    // Cleanup.
    let _ = std::fs::remove_file(&temp);
}

#[test]
fn pack_file_loader_rejects_missing_manifest_raw_rust_requirements() {
    use digimon_dsl::{loader, CardPack};
    use digimon_engine::dsl::raw_rust_registry::StubRegistry;
    use std::path::PathBuf;

    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let (specs, errs) = loader::load_dir_ok(&examples);
    assert!(errs.is_empty(), "fixture parse errors: {errs:#?}");

    let registry =
        digimon_dsl::CardRegistry::from_specs("test-pack", &specs).expect("compile fixtures");
    let mut manifest = registry.manifest.clone();
    manifest.required_raw_rust_fns = vec!["missing_manifest_fn".into()];
    let pack = CardPack {
        manifest,
        cards: registry.iter().map(|(_, c)| c.clone()).collect(),
    };

    let mut temp = std::env::temp_dir();
    temp.push("digimon-pack-file-loader-missing-raw-rust-test.pack");
    let bytes = pack.to_bytes().expect("serialize pack");
    std::fs::write(&temp, &bytes).expect("write temp pack");

    let reg = StubRegistry::empty();
    let err = match digimon_engine::dsl_registry::from_pack_file_with_raw_registry(&temp, &reg) {
        Ok(_) => panic!("expected missing raw_rust manifest requirement to fail"),
        Err(err) => err,
    };
    assert!(err.contains("test-pack"));
    assert!(err.contains("missing_manifest_fn"));

    let _ = std::fs::remove_file(&temp);
}

#[test]
fn pack_file_loader_accepts_present_manifest_raw_rust_requirements() {
    use digimon_dsl::{loader, CardPack};
    use digimon_engine::dsl::raw_rust_registry::StubRegistry;
    use std::path::PathBuf;

    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cards/_examples");
    let (specs, errs) = loader::load_dir_ok(&examples);
    assert!(errs.is_empty(), "fixture parse errors: {errs:#?}");

    let registry =
        digimon_dsl::CardRegistry::from_specs("test-pack", &specs).expect("compile fixtures");
    let mut manifest = registry.manifest.clone();
    manifest.required_raw_rust_fns = vec!["present_manifest_fn".into()];
    let pack = CardPack {
        manifest,
        cards: registry.iter().map(|(_, c)| c.clone()).collect(),
    };

    let mut temp = std::env::temp_dir();
    temp.push("digimon-pack-file-loader-present-raw-rust-test.pack");
    let bytes = pack.to_bytes().expect("serialize pack");
    std::fs::write(&temp, &bytes).expect("write temp pack");

    let reg = StubRegistry::with(["present_manifest_fn"]);
    let loaded = digimon_engine::dsl_registry::from_pack_file_with_raw_registry(&temp, &reg)
        .expect("from_pack_file_with_raw_registry");
    assert_eq!(loaded.manifest.pack_id, "test-pack");

    let _ = std::fs::remove_file(&temp);
}
