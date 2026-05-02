use digimon_engine::observation::{build_observation_tensor, parse_observation_profile};
use digimon_engine::tensor_profiles::standard::v2_lite;

use crate::tensor_helpers::sample_game_with_known_cards;

#[test]
fn v2_lite_tensor_has_expected_size_and_version_marker() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    assert_eq!(tensor.len(), v2_lite::TENSOR_SIZE);
    assert_eq!(tensor[v2_lite::OFF_GLOBAL_FEATURES], 2.0);
}

#[test]
fn v2_lite_does_not_encode_opponent_hand_identities() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);
    let opponent_hand_card_index = registry.get_index("ST1-03") as f32;

    let known_positions = &digimon_engine::tensor_profiles::profile_by_id("standard_lite_v2")
        .unwrap()
        .positions()
        .0;
    for position in known_positions {
        if *position < v2_lite::OFF_OWN_HAND
            || *position >= v2_lite::OFF_OWN_HAND + v2_lite::OWN_HAND_SIZE
        {
            assert_ne!(
                tensor[*position], opponent_hand_card_index,
                "opponent hand card identity leaked at tensor position {position}"
            );
        }
    }
}

#[test]
fn v2_lite_uses_breeding_slot_14_with_battle_affordances_off() {
    let (game, registry) = sample_game_with_known_cards();
    let profile = parse_observation_profile("standard_lite_v2").unwrap();
    let tensor = build_observation_tensor(&game, 0, &registry, profile);

    let own_breeding_base = v2_lite::OFF_PERMANENT_SLOTS + 14 * v2_lite::PERMANENT_SLOT_SIZE;
    assert_eq!(tensor[own_breeding_base], 1.0);
    assert_eq!(tensor[own_breeding_base + 3], 0.0);
    assert_eq!(tensor[own_breeding_base + 4], 1.0);
    assert_eq!(tensor[own_breeding_base + 33], 0.0);
    assert_eq!(tensor[own_breeding_base + 34], 0.0);
}
