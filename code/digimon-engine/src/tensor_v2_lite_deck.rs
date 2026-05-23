use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::tensor_profiles::standard::{v2_lite, v2_lite_deck as layout};
use crate::tensor_v2_lite::build_tensor_standard_lite_v2;

pub fn build_tensor_standard_lite_deck_v2(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
) -> Vec<f32> {
    let lite = build_tensor_standard_lite_v2(game, player_id, registry);
    let mut tensor = vec![0.0f32; layout::TENSOR_SIZE];
    tensor[..v2_lite::OFF_RESERVED].copy_from_slice(&lite[..v2_lite::OFF_RESERVED]);
    write_original_decklist(&mut tensor, game, player_id, registry);
    tensor
}

fn write_original_decklist(
    tensor: &mut [f32],
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
) {
    let mut rows = game.player(player_id).original_deck.clone();
    rows.sort_by_key(|row| (registry.get_index(&row.card_id), row.card_id.clone()));

    for (row_index, row) in rows.into_iter().take(layout::DECKLIST_ROWS).enumerate() {
        let base = layout::OFF_OWN_ORIGINAL_DECKLIST + row_index * layout::DECKLIST_ROW_SIZE;
        tensor[base] = 1.0;
        tensor[base + layout::DECKLIST_CARD_ID_OFFSET] = registry.get_index(&row.card_id) as f32;
        tensor[base + 2] = row.count as f32 / 4.0;
        tensor[base + 3] = if row.is_digitama { 0.0 } else { 1.0 };
        tensor[base + 4] = if row.is_digitama { 1.0 } else { 0.0 };
    }
}
