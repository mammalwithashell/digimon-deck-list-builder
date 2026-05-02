use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::tensor_profiles::standard::v2_lite;

pub fn build_tensor_standard_lite_v2(
    _game: &Game,
    _player_id: PlayerId,
    _registry: &CardRegistry,
) -> Vec<f32> {
    let mut tensor = vec![0.0; v2_lite::TENSOR_SIZE];
    tensor[v2_lite::OFF_GLOBAL_FEATURES] = v2_lite::TENSOR_VERSION as f32;
    tensor
}
