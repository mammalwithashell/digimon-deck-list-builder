use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{ACTION_SPACE_SIZE, PASS};
use digimon_engine::enums::{EffectSourceKind, GamePhase};
use digimon_engine::observation::{
    build_observation_tensor, default_observation_profile, observation_layout,
    parse_observation_profile, ObservationProfileId,
};
use digimon_engine::selection::{PendingSelection, SelectionKind};
use digimon_engine::tensor_profiles::standard::{v2_full, v2_lite};
use digimon_engine::tensor_profiles::STANDARD_FULL_V2_PROFILE_ID;
use digimon_engine::tensor_v2_full::build_tensor_standard_full_v2;
use digimon_engine::tensor_v2_lite::build_tensor_standard_lite_v2;

use crate::tensor_helpers::sample_game_with_known_cards;

#[test]
fn v2_full_tensor_has_expected_size() {
    let (game, registry) = sample_game_with_known_cards();

    let tensor = build_tensor_standard_full_v2(&game, 0, &registry);

    assert_eq!(tensor.len(), v2_full::TENSOR_SIZE);
}

#[test]
fn observation_dispatch_supports_standard_full_v2_without_changing_default() {
    let (game, registry) = sample_game_with_known_cards();

    assert_eq!(
        default_observation_profile(),
        ObservationProfileId::StandardLiteV2
    );

    let profile_id = parse_observation_profile(STANDARD_FULL_V2_PROFILE_ID).unwrap();
    assert_eq!(profile_id, ObservationProfileId::StandardFullV2);
    assert_eq!(profile_id.as_str(), STANDARD_FULL_V2_PROFILE_ID);

    let layout = observation_layout(profile_id);
    assert_eq!(layout.id, STANDARD_FULL_V2_PROFILE_ID);
    assert_eq!(layout.tensor_size, v2_full::TENSOR_SIZE);

    let dispatched = build_observation_tensor(&game, 0, &registry, profile_id);
    let direct = build_tensor_standard_full_v2(&game, 0, &registry);
    assert_eq!(dispatched, direct);
}

#[test]
fn v2_full_tensor_keeps_lite_prefix_through_reserved_offset() {
    let (game, registry) = sample_game_with_known_cards();

    let lite = build_tensor_standard_lite_v2(&game, 0, &registry);
    let full = build_tensor_standard_full_v2(&game, 0, &registry);

    assert_eq!(
        &full[..v2_lite::OFF_RESERVED],
        &lite[..v2_lite::OFF_RESERVED]
    );
}

#[test]
fn v2_full_action_rows_match_mask_and_normalized_action_id() {
    let (game, registry) = sample_game_with_known_cards();

    let mask = build_action_mask(&game, 0);
    let tensor = build_tensor_standard_full_v2(&game, 0, &registry);

    assert_eq!(mask.len(), ACTION_SPACE_SIZE);
    for action_id in 0..ACTION_SPACE_SIZE {
        let row_base = v2_full::OFF_ACTION_ID_FEATURES + action_id * v2_full::ACTION_ID_ROW_SIZE;
        assert_eq!(
            tensor[row_base], mask[action_id],
            "legal flag mismatch for action {action_id}"
        );
        assert_eq!(
            tensor[row_base + 1],
            action_id as f32 / ACTION_SPACE_SIZE as f32,
            "normalized action id mismatch for action {action_id}"
        );
    }
}

#[test]
fn v2_full_marks_prompt_selection_flag_for_optional_pass_action() {
    let (mut game, registry) = sample_game_with_known_cards();
    let source_card = game.players[0].battle_area[0].top_card().handle();
    game.current_phase = GamePhase::EffectChoice;
    game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: GamePhase::Main,
        valid_action_ids: vec![PASS],
        is_optional: true,
        prompt: "decline optional effect".to_string(),
        effect_choices: None,
        source_card,
        source_permanent: None,
        source_kind: EffectSourceKind::Digimon,
        callback: Box::new(|_, _| {}),
        on_decline: Some(Box::new(|_| {})),
    });

    let tensor = build_tensor_standard_full_v2(&game, 0, &registry);
    let pass_base = v2_full::OFF_ACTION_ID_FEATURES + PASS as usize * v2_full::ACTION_ID_ROW_SIZE;

    assert_eq!(tensor[pass_base], 1.0);
    assert_eq!(tensor[pass_base + 14], 1.0);
}
