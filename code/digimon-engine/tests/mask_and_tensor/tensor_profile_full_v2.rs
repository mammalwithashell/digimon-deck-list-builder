use digimon_engine::tensor_profiles::{
    all_profile_ids, profile_by_id, TensorSectionKind, STANDARD_COMPACT_V1_PROFILE_ID,
    STANDARD_FULL_V2_PROFILE_ID, STANDARD_LITE_DECK_V2_PROFILE_ID, STANDARD_LITE_V2_PROFILE_ID,
};

#[test]
fn standard_full_v2_layout_matches_spec() {
    let profile = profile_by_id("standard_full_v2").unwrap();

    assert_eq!(profile.id, "standard_full_v2");
    assert_eq!(profile.game_mode, "standard");
    assert_eq!(profile.version, 2);
    assert_eq!(profile.tensor_version, 2);
    // The owned physical choice prefix matches lite_v2; the full profile
    // appends all 2192 action feature rows before reserved space.
    assert_eq!(profile.feature_schema_version, "standard_full_v2.5");
    assert_eq!(profile.tensor_size, 44506);
    assert_eq!(profile.card_id_slot_count, 604);
    assert_eq!(profile.scalar_slot_count, 43902);

    let expected_sections = [
        ("global_features", 0, &[64][..], 64),
        ("player_summary", 64, &[2, 32][..], 64),
        ("permanent_slots", 128, &[2, 15, 99][..], 2970),
        ("own_hand", 3098, &[30, 32][..], 960),
        ("known_zone_cards", 4058, &[120, 8][..], 960),
        ("decision_context", 5018, &[64][..], 64),
        ("pending_choice_features", 5082, &[32, 128][..], 4096),
        ("action_id_features", 9178, &[2192, 16][..], 35072),
        ("reserved", 44250, &[256][..], 256),
    ];

    for (id, start, shape, len) in expected_sections {
        let section = profile.section(id).unwrap();
        assert_eq!(section.start, start, "{id} start");
        assert_eq!(section.shape, shape, "{id} shape");
        assert_eq!(section.len, len, "{id} len");
    }

    let action_id_features = profile.section("action_id_features").unwrap();
    assert_eq!(action_id_features.kind, TensorSectionKind::Scalars);
}

#[test]
fn standard_full_v2_positions_cover_tensor_once() {
    let profile = profile_by_id("standard_full_v2").unwrap();
    let (cards, scalars) = profile.positions();

    assert_eq!(cards.len(), 604);
    assert_eq!(scalars.len(), 43902);
    assert_eq!(cards.len() + scalars.len(), profile.tensor_size);

    let card_set: std::collections::BTreeSet<_> = cards.iter().copied().collect();
    let scalar_set: std::collections::BTreeSet<_> = scalars.iter().copied().collect();
    assert!(card_set.is_disjoint(&scalar_set));

    let all: std::collections::BTreeSet<_> = card_set.union(&scalar_set).copied().collect();
    let expected: std::collections::BTreeSet<_> = (0..profile.tensor_size).collect();
    assert_eq!(all, expected);
}

#[test]
fn profile_list_includes_compact_lite_v2_and_full_v2() {
    let profiles = all_profile_ids();
    assert_eq!(
        profiles,
        vec![
            STANDARD_COMPACT_V1_PROFILE_ID,
            STANDARD_LITE_V2_PROFILE_ID,
            STANDARD_LITE_DECK_V2_PROFILE_ID,
            STANDARD_FULL_V2_PROFILE_ID,
        ]
    );
}

#[test]
fn standard_full_v2_layout_hash_matches_canonical_recomputation() {
    let profile = profile_by_id("standard_full_v2").unwrap();

    assert_eq!(
        profile.layout_hash,
        profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version)
    );
}
