use digimon_engine::tensor_profiles::{all_profile_ids, profile_by_id, TensorSectionKind};

#[test]
fn standard_lite_deck_v2_layout_matches_spec() {
    let profile = profile_by_id("standard_lite_deck_v2").unwrap();

    assert_eq!(profile.id, "standard_lite_deck_v2");
    assert_eq!(profile.game_mode, "standard");
    assert_eq!(profile.version, 2);
    assert_eq!(profile.tensor_version, 2);
    assert_eq!(profile.feature_schema_version, "standard_lite_deck_v2.1");
    assert_eq!(profile.tensor_size, 8850);
    assert_eq!(profile.card_id_slot_count, 627);
    assert_eq!(profile.scalar_slot_count, 8223);
    assert!(profile.layout_hash.starts_with("sha256:"));
    assert_eq!(profile.layout_hash.len(), "sha256:".len() + 64);

    let expected_sections = [
        ("global_features", 0, &[64][..], 64),
        ("player_summary", 64, &[2, 32][..], 64),
        ("permanent_slots", 128, &[2, 15, 99][..], 2970),
        ("own_hand", 3098, &[30, 32][..], 960),
        ("known_zone_cards", 4058, &[120, 8][..], 960),
        ("decision_context", 5018, &[64][..], 64),
        ("pending_choice_features", 5082, &[32, 96][..], 3072),
        ("own_original_decklist", 8154, &[55, 8][..], 440),
        ("reserved", 8594, &[256][..], 256),
    ];

    for (id, start, shape, len) in expected_sections {
        let section = profile.section(id).unwrap();
        assert_eq!(section.start, start, "{id} start");
        assert_eq!(section.shape, shape, "{id} shape");
        assert_eq!(section.len, len, "{id} len");
    }

    assert_eq!(
        profile.section("own_original_decklist").unwrap().kind,
        TensorSectionKind::StandardLiteV2Rows
    );
    assert_eq!(
        profile.section("reserved").unwrap().kind,
        TensorSectionKind::Scalars
    );
}

#[test]
fn standard_lite_deck_v2_positions_cover_tensor_once() {
    let profile = profile_by_id("standard_lite_deck_v2").unwrap();
    let (cards, scalars) = profile.positions();

    assert_eq!(cards.len(), 627);
    assert_eq!(scalars.len(), 8223);
    assert_eq!(cards.len() + scalars.len(), profile.tensor_size);

    let card_set: std::collections::BTreeSet<_> = cards.iter().copied().collect();
    let scalar_set: std::collections::BTreeSet<_> = scalars.iter().copied().collect();
    assert!(card_set.is_disjoint(&scalar_set));

    let all: std::collections::BTreeSet<_> = card_set.union(&scalar_set).copied().collect();
    let expected: std::collections::BTreeSet<_> = (0..profile.tensor_size).collect();
    assert_eq!(all, expected);
}

#[test]
fn standard_lite_deck_v2_decklist_card_positions_are_embedded() {
    let profile = profile_by_id("standard_lite_deck_v2").unwrap();
    let section = profile.section("own_original_decklist").unwrap();
    let (cards, _) = profile.positions();

    for row in 0..55 {
        assert!(
            cards.contains(&(section.start + row * 8 + 1)),
            "decklist row {row} card_id offset should be a card-id position"
        );
    }
}

#[test]
fn profile_list_includes_lite_deck_v2() {
    let profiles = all_profile_ids();

    assert!(profiles.contains(&"standard_lite_deck_v2"));
}

#[test]
fn standard_lite_deck_v2_layout_hash_matches_canonical_recomputation() {
    let profile = profile_by_id("standard_lite_deck_v2").unwrap();

    assert_eq!(
        profile.layout_hash,
        profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version)
    );
}
