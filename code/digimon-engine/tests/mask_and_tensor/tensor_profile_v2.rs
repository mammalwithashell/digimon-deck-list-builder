use digimon_engine::tensor_profiles::{
    all_profile_ids, profile_by_id, TensorSectionKind, STANDARD_COMPACT_V1_PROFILE_ID,
    STANDARD_LITE_V2_PROFILE_ID,
};

#[test]
fn standard_lite_v2_layout_matches_spec() {
    let profile = profile_by_id("standard_lite_v2").unwrap();

    assert_eq!(profile.id, "standard_lite_v2");
    assert_eq!(profile.game_mode, "standard");
    assert_eq!(profile.version, 2);
    assert_eq!(profile.tensor_version, 2);
    assert_eq!(profile.feature_schema_version, "standard_lite_v2.1");
    assert_eq!(profile.tensor_size, 8320);
    assert_eq!(profile.card_id_slot_count, 542);
    assert_eq!(profile.scalar_slot_count, 7778);
    assert!(profile.layout_hash.starts_with("sha256:"));
    assert_eq!(profile.layout_hash.len(), "sha256:".len() + 64);

    let expected_sections = [
        ("global_features", 0, &[64][..], 64),
        ("player_summary", 64, &[2, 32][..], 64),
        ("permanent_slots", 128, &[2, 15, 96][..], 2880),
        ("own_hand", 3008, &[30, 32][..], 960),
        ("known_zone_cards", 3968, &[120, 8][..], 960),
        ("decision_context", 4928, &[64][..], 64),
        ("pending_choice_features", 4992, &[32, 96][..], 3072),
        ("reserved", 8064, &[256][..], 256),
    ];

    for (id, start, shape, len) in expected_sections {
        let section = profile.section(id).unwrap();
        assert_eq!(section.start, start, "{id} start");
        assert_eq!(section.shape, shape, "{id} shape");
        assert_eq!(section.len, len, "{id} len");
    }

    assert_eq!(
        profile.section("reserved").unwrap().kind,
        TensorSectionKind::Scalars
    );
}

#[test]
fn standard_lite_v2_card_id_positions_match_spec() {
    let profile = profile_by_id("standard_lite_v2").unwrap();
    let (cards, _) = profile.positions();

    let mut expected = Vec::new();

    for row in 0..30 {
        let base = 128 + row * 96;
        expected.push(base + 8);
        for source_index in 0..11 {
            expected.push(base + 63 + source_index * 3);
        }
    }

    for row in 0..30 {
        expected.push(3008 + row * 32 + 1);
    }

    for row in 0..120 {
        expected.push(3968 + row * 8 + 1);
    }

    for row in 0..32 {
        expected.push(4992 + row * 96 + 44);
    }

    expected.sort();
    assert_eq!(cards, expected);
    assert_eq!(cards.len(), 542);
}

#[test]
fn standard_lite_v2_positions_cover_tensor_once() {
    let profile = profile_by_id("standard_lite_v2").unwrap();
    let (cards, scalars) = profile.positions();

    assert_eq!(cards.len(), 542);
    assert_eq!(scalars.len(), 7778);
    assert_eq!(cards.len() + scalars.len(), profile.tensor_size);

    let card_set: std::collections::BTreeSet<_> = cards.iter().copied().collect();
    let scalar_set: std::collections::BTreeSet<_> = scalars.iter().copied().collect();
    assert!(card_set.is_disjoint(&scalar_set));

    let all: std::collections::BTreeSet<_> = card_set.union(&scalar_set).copied().collect();
    let expected: std::collections::BTreeSet<_> = (0..profile.tensor_size).collect();
    assert_eq!(all, expected);
}

#[test]
fn profile_list_includes_compact_and_lite_v2() {
    let profiles = all_profile_ids();
    assert_eq!(
        profiles,
        vec![STANDARD_COMPACT_V1_PROFILE_ID, STANDARD_LITE_V2_PROFILE_ID]
    );
}

#[test]
fn v2_lite_alias_resolves_to_canonical_profile() {
    let profile = profile_by_id("v2_lite").unwrap();

    assert_eq!(profile.id, STANDARD_LITE_V2_PROFILE_ID);
}

#[test]
fn standard_lite_v2_layout_hash_matches_canonical_recomputation() {
    let profile = profile_by_id("standard_lite_v2").unwrap();

    assert_eq!(
        profile.layout_hash,
        profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version)
    );
}
