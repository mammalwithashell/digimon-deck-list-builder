use crate::action::explain::{explain_action, ActionKind, ActionZone};
use crate::action::{build_action_mask, ACTION_SPACE_SIZE};
use crate::card_registry::CardRegistry;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::tensor_profiles::standard::{v2_full, v2_lite};
use crate::tensor_v2_lite::build_tensor_standard_lite_v2;

pub fn build_tensor_standard_full_v2(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
) -> Vec<f32> {
    let lite = build_tensor_standard_lite_v2(game, player_id, registry);
    let mut tensor = vec![0.0f32; v2_full::TENSOR_SIZE];
    tensor[..v2_lite::OFF_RESERVED].copy_from_slice(&lite[..v2_lite::OFF_RESERVED]);
    write_action_id_features(&mut tensor, game, player_id);
    tensor
}

fn write_action_id_features(tensor: &mut [f32], game: &Game, player_id: PlayerId) {
    let mask = build_action_mask(game, player_id);
    for action_id in 0..ACTION_SPACE_SIZE {
        let action_id_u16 = action_id as u16;
        let explanation = explain_action(game, player_id, action_id_u16);
        let base = v2_full::OFF_ACTION_ID_FEATURES + action_id * v2_full::ACTION_ID_ROW_SIZE;

        tensor[base] = mask[action_id];
        tensor[base + 1] = action_id as f32 / ACTION_SPACE_SIZE as f32;
        tensor[base + 2] = action_kind_bucket(explanation.kind);
        tensor[base + 3] = phase_bucket(game.current_phase);
        tensor[base + 4] = explanation.source_zone.map(zone_bucket).unwrap_or(0.0);
        tensor[base + 5] = index_bucket(explanation.source_index);
        tensor[base + 6] = explanation.target_zone.map(zone_bucket).unwrap_or(0.0);
        tensor[base + 7] = index_bucket(explanation.target_index);
        tensor[base + 8] = permanent_slot_bucket(explanation.source_zone, explanation.source_index);
        tensor[base + 9] = permanent_slot_bucket(explanation.target_zone, explanation.target_index);
        tensor[base + 12] = uses_hand(explanation.source_zone, explanation.target_zone);
        tensor[base + 13] = uses_permanent(explanation.source_zone, explanation.target_zone);
        tensor[base + 14] = prompt_selection_action_flag(game, player_id, action_id_u16);
    }
}

fn action_kind_bucket(kind: ActionKind) -> f32 {
    match kind {
        ActionKind::Play => 1.0,
        ActionKind::HandEffect => 2.0,
        ActionKind::Hatch => 3.0,
        ActionKind::Move => 4.0,
        ActionKind::Pass => 5.0,
        ActionKind::DnaDigivolve => 6.0,
        ActionKind::Attack => 7.0,
        ActionKind::Digivolve => 8.0,
        ActionKind::FieldEffect => 9.0,
        ActionKind::TrashEffect => 10.0,
        ActionKind::SourceSelect => 11.0,
        ActionKind::Selection => 12.0,
        ActionKind::Unknown => 0.0,
    }
}

fn phase_bucket(phase: GamePhase) -> f32 {
    phase.tensor_value() as f32
}

fn zone_bucket(zone: ActionZone) -> f32 {
    match zone {
        ActionZone::Hand => 1.0,
        ActionZone::Battle => 2.0,
        ActionZone::Breeding => 3.0,
        ActionZone::Security => 4.0,
        ActionZone::Trash => 5.0,
        ActionZone::Source => 6.0,
        ActionZone::Revealed => 7.0,
        ActionZone::EffectChoice => 8.0,
    }
}

fn index_bucket(index: Option<u16>) -> f32 {
    index.map(|idx| idx as f32 / 30.0).unwrap_or(0.0)
}

fn permanent_slot_bucket(zone: Option<ActionZone>, index: Option<u16>) -> f32 {
    match zone {
        Some(ActionZone::Battle) => index.map(|idx| idx as f32 / 14.0).unwrap_or(0.0),
        Some(ActionZone::Breeding) => 1.0,
        _ => 0.0,
    }
}

fn uses_hand(source_zone: Option<ActionZone>, target_zone: Option<ActionZone>) -> f32 {
    if matches!(source_zone, Some(ActionZone::Hand))
        || matches!(target_zone, Some(ActionZone::Hand))
    {
        1.0
    } else {
        0.0
    }
}

fn uses_permanent(source_zone: Option<ActionZone>, target_zone: Option<ActionZone>) -> f32 {
    if matches!(source_zone, Some(ActionZone::Battle | ActionZone::Breeding))
        || matches!(target_zone, Some(ActionZone::Battle | ActionZone::Breeding))
    {
        1.0
    } else {
        0.0
    }
}

fn prompt_selection_action_flag(game: &Game, player_id: PlayerId, action_id: u16) -> f32 {
    let Some(selection) = game.pending_selection.as_ref() else {
        return 0.0;
    };
    if selection.selecting_player != player_id {
        return 0.0;
    }
    if selection.valid_action_ids.contains(&action_id)
        || (selection.is_optional && action_id == crate::action::PASS)
    {
        1.0
    } else {
        0.0
    }
}
