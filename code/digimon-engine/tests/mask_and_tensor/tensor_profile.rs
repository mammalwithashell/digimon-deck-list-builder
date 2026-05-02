use digimon_engine::tensor::{
    compute_positions, FIELD_SLOTS, GLOBAL_SIZE, HAND_SIZE, MAX_SOURCES, OFF_GLOBAL, OFF_MY_BATTLE,
    OFF_MY_BREEDING, OFF_MY_HAND, OFF_MY_SECURITY, OFF_MY_TRASH, OFF_OPP_BATTLE, OFF_OPP_BREEDING,
    OFF_OPP_HAND, OFF_OPP_SECURITY, OFF_OPP_TRASH, OFF_REVEALED, OFF_SELECTION, REVEALED_SIZE,
    SECURITY_SIZE, SELECTION_SIZE, SLOT_DP_OFFSET, SLOT_HEADER_SIZE, SLOT_LINKED_COUNT_OFFSET,
    SLOT_OPT_TOTAL_OFFSET, SLOT_OPT_USED_OFFSET, SLOT_SIZE, SLOT_SOURCE_COUNT_OFFSET,
    SLOT_SOURCE_START_OFFSET, SLOT_SUSPENDED_OFFSET, SLOT_TOP_CARD_OFFSET, SOURCE_CARD_ID_OFFSET,
    SOURCE_DP_CONTRIBUTION_OFFSET, SOURCE_ENTRY_SIZE, SOURCE_OPT_STATE_OFFSET, TENSOR_SIZE,
    TRASH_SIZE,
};
use digimon_engine::tensor_profiles::standard;
use digimon_engine::tensor_profiles::{
    all_profile_ids, default_profile, profile_by_id, TensorFieldKind, TensorSectionKind,
    COMPACT_V1_LEGACY_PROFILE_ID, STANDARD_COMPACT_V1_PROFILE_ID, STANDARD_LITE_V2_PROFILE_ID,
    STANDARD_V1_LEGACY_PROFILE_ID,
};

#[test]
fn default_profile_is_standard_compact_v1() {
    let profile = default_profile();

    assert_eq!(profile.id, STANDARD_COMPACT_V1_PROFILE_ID);
    assert_eq!(profile.id, "standard_compact_v1");
    assert_eq!(profile.game_mode, "standard");
    assert_eq!(profile.version, 1);
    assert_eq!(profile.tensor_version, 1);
    assert_eq!(profile.feature_schema_version, "standard_compact_v1.1");
    assert_eq!(profile.tensor_size, TENSOR_SIZE);
    assert_eq!(profile.field_slots, FIELD_SLOTS);
    assert_eq!(profile.slot_size, SLOT_SIZE);
    assert_eq!(profile.max_sources, MAX_SOURCES);
    assert_eq!(profile.slot_layout.size, SLOT_SIZE);
    assert_eq!(profile.slot_layout.source_entry_size, SOURCE_ENTRY_SIZE);
    assert_eq!(profile.card_id_slot_count, 520);
    assert_eq!(profile.scalar_slot_count, 855);
}

#[test]
fn every_profile_has_schema_version_and_layout_hash() {
    for id in all_profile_ids() {
        let profile = profile_by_id(id).unwrap();
        assert!(!profile.feature_schema_version.is_empty());
        assert!(profile.layout_hash.starts_with("sha256:"));
        assert_eq!(profile.layout_hash.len(), "sha256:".len() + 64);
        assert!(profile.layout_hash["sha256:".len()..]
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()));
        assert_eq!(
            profile.layout_hash,
            profile.layout_hash_with_schema_version_for_test(profile.feature_schema_version)
        );
    }
}

#[test]
fn layout_hash_changes_when_feature_schema_version_changes() {
    let profile = default_profile();
    let baseline = profile.layout_hash;
    let changed = profile.layout_hash_with_schema_version_for_test("schema-version-test-only");

    assert_ne!(changed, baseline);
    assert!(changed.starts_with("sha256:"));
}

#[test]
fn sections_expose_debug_shapes() {
    let profile = default_profile();
    let global = profile.section("global").unwrap();
    assert_eq!(global.shape, &[10]);

    let my_battle = profile.section("my_battle").unwrap();
    assert_eq!(my_battle.shape, &[14, 40]);
}

#[test]
fn registry_lists_only_canonical_profile_ids() {
    assert_eq!(
        all_profile_ids(),
        vec![STANDARD_COMPACT_V1_PROFILE_ID, STANDARD_LITE_V2_PROFILE_ID]
    );
}

#[test]
fn registry_resolves_standard_compact_profile_and_legacy_aliases() {
    for id in [
        STANDARD_COMPACT_V1_PROFILE_ID,
        STANDARD_V1_LEGACY_PROFILE_ID,
        COMPACT_V1_LEGACY_PROFILE_ID,
    ] {
        let profile = profile_by_id(id).unwrap();
        assert_eq!(profile.id, STANDARD_COMPACT_V1_PROFILE_ID);
        assert_eq!(profile.game_mode, "standard");
        assert_eq!(profile.tensor_size, TENSOR_SIZE);
    }

    assert!(profile_by_id("missing_profile").is_none());
}

#[test]
fn standard_family_resolves_profile_by_version() {
    let profile = standard::profile_by_version(1).unwrap();

    assert_eq!(standard::DEFAULT_PROFILE, standard::v1::PROFILE);
    assert_eq!(profile, standard::v1::PROFILE);
    assert_eq!(
        standard::profile_by_version(2).unwrap(),
        standard::v2_lite::PROFILE
    );
}

#[test]
fn standard_v1_owns_tensor_layout_constants() {
    assert_eq!(standard::v1::PROFILE.tensor_size, standard::v1::TENSOR_SIZE);
    assert_eq!(standard::v1::PROFILE.field_slots, standard::v1::FIELD_SLOTS);
    assert_eq!(standard::v1::PROFILE.slot_size, standard::v1::SLOT_SIZE);
    assert_eq!(standard::v1::PROFILE.max_sources, standard::v1::MAX_SOURCES);

    assert_eq!(TENSOR_SIZE, standard::v1::TENSOR_SIZE);
    assert_eq!(FIELD_SLOTS, standard::v1::FIELD_SLOTS);
    assert_eq!(SLOT_SIZE, standard::v1::SLOT_SIZE);
    assert_eq!(MAX_SOURCES, standard::v1::MAX_SOURCES);
}

#[test]
fn standard_profile_sections_cover_tensor_without_overlap() {
    let profile = default_profile();
    let mut covered = Vec::new();

    for section in profile.sections {
        assert!(section.start + section.len <= profile.tensor_size);
        if section.kind == TensorSectionKind::PermanentSlots {
            assert_eq!(section.len % profile.slot_layout.size, 0);
        }
        covered.extend(section.start..section.start + section.len);
    }

    covered.sort();
    assert_eq!(covered.len(), profile.tensor_size);
    covered.dedup();
    assert_eq!(covered.len(), profile.tensor_size);
    assert_eq!(covered[0], 0);
    assert_eq!(*covered.last().unwrap(), profile.tensor_size - 1);
}

#[test]
fn standard_profile_card_and_scalar_positions_match_tensor_module() {
    let profile = default_profile();
    let (profile_cards, profile_scalars) = profile.positions();
    let (v1_cards, v1_scalars) = standard::v1::PROFILE.positions();
    let (tensor_cards, tensor_scalars) = compute_positions();

    assert_eq!(profile_cards, v1_cards);
    assert_eq!(profile_scalars, v1_scalars);
    assert_eq!(profile_cards, tensor_cards);
    assert_eq!(profile_scalars, tensor_scalars);
    assert_eq!(profile_cards.len(), profile.card_id_slot_count);
    assert_eq!(profile_scalars.len(), profile.scalar_slot_count);

    let card_set: std::collections::BTreeSet<_> = profile_cards.iter().copied().collect();
    let scalar_set: std::collections::BTreeSet<_> = profile_scalars.iter().copied().collect();
    let position_set: std::collections::BTreeSet<_> =
        card_set.union(&scalar_set).copied().collect();
    let expected_positions: std::collections::BTreeSet<_> = (0..profile.tensor_size).collect();

    assert!(card_set.is_disjoint(&scalar_set));
    assert!(position_set
        .iter()
        .all(|position| *position < profile.tensor_size));
    assert_eq!(position_set, expected_positions);
}

#[test]
fn standard_profile_marks_card_sections() {
    let profile = default_profile();

    let hand = profile.section("my_hand").unwrap();
    assert_eq!(hand.kind, TensorSectionKind::CardIds);
    assert_eq!(hand.start, 1130);
    assert_eq!(hand.len, 20);

    let battle = profile.section("my_battle").unwrap();
    assert_eq!(battle.kind, TensorSectionKind::PermanentSlots);
    assert_eq!(battle.start, 10);
    assert_eq!(battle.len, 560);

    let global = profile.section("global").unwrap();
    assert_eq!(global.kind, TensorSectionKind::Scalars);
    assert_eq!(global.start, 0);
    assert_eq!(global.len, 10);
}

#[test]
fn standard_profile_sections_match_tensor_layout_constants() {
    let profile = default_profile();

    assert_eq!(
        (
            profile.section("global").unwrap().start,
            profile.section("global").unwrap().len
        ),
        (OFF_GLOBAL, GLOBAL_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_battle").unwrap().start,
            profile.section("my_battle").unwrap().len,
        ),
        (OFF_MY_BATTLE, FIELD_SLOTS * SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_battle").unwrap().start,
            profile.section("opponent_battle").unwrap().len,
        ),
        (OFF_OPP_BATTLE, FIELD_SLOTS * SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_hand").unwrap().start,
            profile.section("my_hand").unwrap().len
        ),
        (OFF_MY_HAND, HAND_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_hand").unwrap().start,
            profile.section("opponent_hand").unwrap().len,
        ),
        (OFF_OPP_HAND, HAND_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_trash").unwrap().start,
            profile.section("my_trash").unwrap().len
        ),
        (OFF_MY_TRASH, TRASH_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_trash").unwrap().start,
            profile.section("opponent_trash").unwrap().len,
        ),
        (OFF_OPP_TRASH, TRASH_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_security").unwrap().start,
            profile.section("my_security").unwrap().len,
        ),
        (OFF_MY_SECURITY, SECURITY_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_security").unwrap().start,
            profile.section("opponent_security").unwrap().len,
        ),
        (OFF_OPP_SECURITY, SECURITY_SIZE)
    );
    assert_eq!(
        (
            profile.section("my_breeding").unwrap().start,
            profile.section("my_breeding").unwrap().len,
        ),
        (OFF_MY_BREEDING, SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("opponent_breeding").unwrap().start,
            profile.section("opponent_breeding").unwrap().len,
        ),
        (OFF_OPP_BREEDING, SLOT_SIZE)
    );
    assert_eq!(
        (
            profile.section("revealed").unwrap().start,
            profile.section("revealed").unwrap().len
        ),
        (OFF_REVEALED, REVEALED_SIZE)
    );
    assert_eq!(
        (
            profile.section("selection").unwrap().start,
            profile.section("selection").unwrap().len,
        ),
        (OFF_SELECTION, SELECTION_SIZE)
    );

    assert_eq!(
        SLOT_HEADER_SIZE + SOURCE_ENTRY_SIZE * MAX_SOURCES,
        SLOT_SIZE
    );
}

#[test]
fn standard_profile_slot_field_offsets_match_tensor_layout_constants() {
    assert_eq!(SLOT_TOP_CARD_OFFSET, 0);
    assert_eq!(SLOT_DP_OFFSET, 1);
    assert_eq!(SLOT_SUSPENDED_OFFSET, 2);
    assert_eq!(SLOT_OPT_TOTAL_OFFSET, 3);
    assert_eq!(SLOT_OPT_USED_OFFSET, 4);
    assert_eq!(SLOT_LINKED_COUNT_OFFSET, 5);
    assert_eq!(SLOT_SOURCE_COUNT_OFFSET, 6);
    assert_eq!(SLOT_SOURCE_START_OFFSET, SLOT_HEADER_SIZE);
    assert_eq!(
        SLOT_SIZE,
        SLOT_SOURCE_START_OFFSET + MAX_SOURCES * SOURCE_ENTRY_SIZE
    );

    assert_eq!(SOURCE_CARD_ID_OFFSET, 0);
    assert_eq!(SOURCE_OPT_STATE_OFFSET, SOURCE_CARD_ID_OFFSET + 1);
    assert_eq!(SOURCE_DP_CONTRIBUTION_OFFSET, SOURCE_OPT_STATE_OFFSET + 1);
    assert!(SOURCE_DP_CONTRIBUTION_OFFSET < SOURCE_ENTRY_SIZE);
}

#[test]
fn standard_profile_slot_layout_is_auditable_metadata() {
    let profile = default_profile();

    assert_eq!(profile.max_sources, MAX_SOURCES);
    assert_eq!(profile.slot_layout.size, SLOT_SIZE);
    assert_eq!(profile.slot_layout.source_start, SLOT_SOURCE_START_OFFSET);
    assert_eq!(profile.slot_layout.source_entry_size, SOURCE_ENTRY_SIZE);
    assert_eq!(profile.slot_layout.max_sources, MAX_SOURCES);

    let header_fields: Vec<_> = profile
        .slot_layout
        .header_fields
        .iter()
        .map(|field| (field.id, field.offset, field.kind))
        .collect();
    assert_eq!(
        header_fields,
        vec![
            ("top_card_id", SLOT_TOP_CARD_OFFSET, TensorFieldKind::CardId),
            ("dp", SLOT_DP_OFFSET, TensorFieldKind::Scalar),
            ("suspended", SLOT_SUSPENDED_OFFSET, TensorFieldKind::Scalar),
            ("opt_total", SLOT_OPT_TOTAL_OFFSET, TensorFieldKind::Scalar),
            ("opt_used", SLOT_OPT_USED_OFFSET, TensorFieldKind::Scalar),
            (
                "linked_count",
                SLOT_LINKED_COUNT_OFFSET,
                TensorFieldKind::Scalar,
            ),
            (
                "source_count",
                SLOT_SOURCE_COUNT_OFFSET,
                TensorFieldKind::Scalar,
            ),
        ]
    );

    let source_fields: Vec<_> = profile
        .slot_layout
        .source_fields
        .iter()
        .map(|field| (field.id, field.offset, field.kind))
        .collect();
    assert_eq!(
        source_fields,
        vec![
            ("card_id", SOURCE_CARD_ID_OFFSET, TensorFieldKind::CardId),
            (
                "opt_state",
                SOURCE_OPT_STATE_OFFSET,
                TensorFieldKind::Scalar,
            ),
            (
                "dp_contribution",
                SOURCE_DP_CONTRIBUTION_OFFSET,
                TensorFieldKind::Scalar,
            ),
        ]
    );
}

#[test]
fn singular_tensor_profile_alias_still_works() {
    let singular = digimon_engine::tensor_profile::default_profile();
    let plural = digimon_engine::tensor_profiles::default_profile();

    assert_eq!(singular, plural);
    assert_eq!(
        digimon_engine::tensor_profile::STANDARD_COMPACT_V1_PROFILE_ID,
        "standard_compact_v1"
    );
    assert_eq!(
        digimon_engine::tensor_profile::STANDARD_V1_LEGACY_PROFILE_ID,
        "standard_v1"
    );
    assert_eq!(
        digimon_engine::tensor_profile::COMPACT_V1_LEGACY_PROFILE_ID,
        "compact_v1"
    );
}
