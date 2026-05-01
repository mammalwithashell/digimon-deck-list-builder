use digimon_engine::tensor::{compute_positions, FIELD_SLOTS, SLOT_SIZE, TENSOR_SIZE};
use digimon_engine::tensor_profile::{
    all_profile_ids, default_profile, profile_by_id, TensorSectionKind, STANDARD_V1_PROFILE_ID,
};

#[test]
fn default_profile_is_standard_v1() {
    let profile = default_profile();

    assert_eq!(profile.id, STANDARD_V1_PROFILE_ID);
    assert_eq!(profile.version, 1);
    assert_eq!(profile.tensor_size, TENSOR_SIZE);
    assert_eq!(profile.field_slots, FIELD_SLOTS);
    assert_eq!(profile.slot_size, SLOT_SIZE);
    assert_eq!(profile.card_id_slot_count, 520);
    assert_eq!(profile.scalar_slot_count, 855);
}

#[test]
fn registry_resolves_standard_profile_by_id() {
    let ids = all_profile_ids();
    assert_eq!(ids, vec![STANDARD_V1_PROFILE_ID]);

    let profile = profile_by_id(STANDARD_V1_PROFILE_ID).unwrap();
    assert_eq!(profile.id, "standard_v1");
    assert!(profile_by_id("missing_profile").is_none());
}

#[test]
fn standard_profile_sections_cover_tensor_without_overlap() {
    let profile = default_profile();
    let mut covered = Vec::new();

    for section in profile.sections {
        assert!(section.start + section.len <= profile.tensor_size);
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
    let (tensor_cards, tensor_scalars) = compute_positions();

    assert_eq!(profile_cards, tensor_cards);
    assert_eq!(profile_scalars, tensor_scalars);
    assert_eq!(profile_cards.len(), profile.card_id_slot_count);
    assert_eq!(profile_scalars.len(), profile.scalar_slot_count);
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
