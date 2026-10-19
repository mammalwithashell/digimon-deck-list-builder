use digimon_dsl::pack::{CardPack, PACK_FORMAT_VERSION};

#[test]
fn current_pack_layout_roundtrips() {
    let pack = CardPack::new("current", vec![]);
    assert_eq!(CardPack::from_bytes(&pack.to_bytes().unwrap()).unwrap(), pack);
}

#[test]
fn previous_layout_pack_is_rejected_even_when_empty() {
    let mut pack = CardPack::new("legacy", vec![]);
    pack.manifest.pack_version = "0.19.0".into();
    let err = CardPack::from_bytes(&pack.to_bytes().unwrap()).unwrap_err();
    assert!(err.contains("unsupported pack format"), "{err}");
    assert!(err.contains("0.19.0") && err.contains(PACK_FORMAT_VERSION), "{err}");
}

#[test]
fn layout_version_is_checked_before_decoding_card_payload() {
    let mut pack = CardPack::new("legacy", vec![]);
    pack.manifest.pack_version = "0.19.0".into();
    // The manifest is the first serialized field, stable across these layouts.
    // No card-vector bytes follow: a layout mismatch must win over a payload error.
    let bytes = bincode::serialize(&pack.manifest).unwrap();
    let err = CardPack::from_bytes(&bytes).unwrap_err();
    assert!(err.contains("unsupported pack format"), "{err}");
}

#[test]
fn future_layout_is_not_silently_accepted() {
    let mut pack = CardPack::new("future", vec![]);
    pack.manifest.pack_version = "999.0.0".into();
    let err = CardPack::from_bytes(&pack.to_bytes().unwrap()).unwrap_err();
    assert!(err.contains("unsupported pack format") && err.contains("999.0.0"), "{err}");
}

#[test]
fn corrupt_manifest_length_returns_error_without_allocating_from_prefix() {
    let bytes = u64::MAX.to_le_bytes();
    let err = CardPack::from_bytes(&bytes).unwrap_err();
    assert!(err.contains("manifest deserialize failed"), "{err}");
}
