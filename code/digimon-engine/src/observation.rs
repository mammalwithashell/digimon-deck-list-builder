use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::tensor_profiles::{
    all_profile_ids, profile_by_id, TensorProfile, COMPACT_V1_LEGACY_PROFILE_ID,
    STANDARD_COMPACT_V1_PROFILE_ID, STANDARD_FULL_V2_PROFILE_ID, STANDARD_LITE_DECK_V2_PROFILE_ID,
    STANDARD_LITE_V2_PROFILE_ID, STANDARD_V1_LEGACY_PROFILE_ID, V2_LITE_TRANSITION_PROFILE_ID,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationProfileId {
    StandardCompactV1,
    StandardLiteV2,
    StandardLiteDeckV2,
    StandardFullV2,
}

impl ObservationProfileId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StandardCompactV1 => STANDARD_COMPACT_V1_PROFILE_ID,
            Self::StandardLiteV2 => STANDARD_LITE_V2_PROFILE_ID,
            Self::StandardLiteDeckV2 => STANDARD_LITE_DECK_V2_PROFILE_ID,
            Self::StandardFullV2 => STANDARD_FULL_V2_PROFILE_ID,
        }
    }
}

pub fn default_observation_profile() -> ObservationProfileId {
    ObservationProfileId::StandardLiteV2
}

pub fn parse_observation_profile(raw: &str) -> Result<ObservationProfileId, String> {
    match raw.trim() {
        STANDARD_COMPACT_V1_PROFILE_ID
        | STANDARD_V1_LEGACY_PROFILE_ID
        | COMPACT_V1_LEGACY_PROFILE_ID => Ok(ObservationProfileId::StandardCompactV1),
        STANDARD_LITE_V2_PROFILE_ID | V2_LITE_TRANSITION_PROFILE_ID => {
            Ok(ObservationProfileId::StandardLiteV2)
        }
        STANDARD_LITE_DECK_V2_PROFILE_ID => Ok(ObservationProfileId::StandardLiteDeckV2),
        STANDARD_FULL_V2_PROFILE_ID => Ok(ObservationProfileId::StandardFullV2),
        other => Err(format!("unknown observation profile: {other}")),
    }
}

pub fn list_observation_profiles() -> Vec<&'static str> {
    all_profile_ids()
}

pub fn observation_layout(profile: ObservationProfileId) -> TensorProfile {
    profile_by_id(profile.as_str()).expect("registered observation profile")
}

pub fn build_observation_tensor(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
    profile: ObservationProfileId,
) -> Vec<f32> {
    match profile {
        ObservationProfileId::StandardCompactV1 => {
            crate::tensor::build_tensor(game, player_id, registry)
        }
        ObservationProfileId::StandardLiteV2 => {
            crate::tensor_v2_lite::build_tensor_standard_lite_v2(game, player_id, registry)
        }
        ObservationProfileId::StandardLiteDeckV2 => {
            crate::tensor_v2_lite_deck::build_tensor_standard_lite_deck_v2(
                game, player_id, registry,
            )
        }
        ObservationProfileId::StandardFullV2 => {
            crate::tensor_v2_full::build_tensor_standard_full_v2(game, player_id, registry)
        }
    }
}
