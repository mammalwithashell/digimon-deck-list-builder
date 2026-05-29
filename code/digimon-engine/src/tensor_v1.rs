//! Standard Compact V1 observation tensor builder.
//!
//! Parallels `tensor_v2_lite::build_tensor_standard_lite_v2` and
//! `tensor_v2_lite_deck::build_tensor_standard_lite_deck_v2` — this is the
//! historical v1 builder, relocated out of `tensor.rs` when the engine's
//! default profile flipped to `standard_lite_deck_v2`. Callers that want
//! the v1 layout explicitly invoke this function (or
//! `observation::build_observation_tensor(.., StandardCompactV1)`).
//!
//! The v1 layout is documented inline below; the canonical layout constants
//! live in `tensor_profiles::standard::v1`.

use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::tensor::DP_NORM;
use crate::tensor_profiles::standard::v1::{
    FIELD_SLOTS, MAX_HAND, MAX_REVEALED, MAX_SECURITY, MAX_TRASH, OFF_MY_BATTLE, OFF_MY_BREEDING,
    OFF_MY_HAND, OFF_MY_SECURITY, OFF_MY_TRASH, OFF_OPP_BATTLE, OFF_OPP_BREEDING, OFF_OPP_HAND,
    OFF_OPP_SECURITY, OFF_OPP_TRASH, OFF_REVEALED, OFF_SELECTION, SLOT_SIZE, SOURCE_ENTRY_SIZE,
    TENSOR_SIZE,
};
use crate::tensor_profiles::{self, TensorSlotHeaderField, TensorSourceField};

/// Build a flat observation tensor from the given player's perspective
/// against the `standard_compact_v1` layout (TENSOR_SIZE = 1375).
///
/// Layout:
/// ```text
///   [0-9]         Global data
///   [10-569]      My battle area  (14 slots × 40)
///   [570-1129]    Opp battle area (14 slots × 40)
///   [1130-1149]   My hand  (20 card IDs)
///   [1150-1169]   Opp hand (20 card IDs)
///   [1170-1214]   My trash (45 card IDs)
///   [1215-1259]   Opp trash (45 card IDs)
///   [1260-1269]   My security (10 card IDs, face-down = 0.0)
///   [1270-1279]   Opp security (10 card IDs, face-down = 0.0)
///   [1280-1319]   My breeding (1 slot × 40)
///   [1320-1359]   Opp breeding (1 slot × 40)
///   [1360-1369]   Revealed cards (10 card IDs, from `game.revealed_cards`)
///   [1370-1374]   Selection context
/// ```
pub fn build_tensor_standard_compact_v1(
    game: &Game,
    player_id: PlayerId,
    registry: &CardRegistry,
) -> Vec<f32> {
    let mut t = vec![0.0f32; TENSOR_SIZE];

    let (me_id, opp_id) = resolve_perspective(game, player_id);
    let me = game.player(me_id);
    let opp = game.player(opp_id);

    // --- Global [0-9] ---
    t[0] = (game.turn_count as f32 / 30.0).min(1.0);
    t[1] = game.current_phase.tensor_value();
    t[2] = get_memory_for(game, me_id) as f32 / 10.0;
    // [3-9] reserved

    // --- My battle area ---
    write_field(
        &mut t,
        OFF_MY_BATTLE,
        me_id,
        FIELD_SLOTS,
        game,
        &game.card_data,
        registry,
    );

    // --- Opp battle area ---
    write_field(
        &mut t,
        OFF_OPP_BATTLE,
        opp_id,
        FIELD_SLOTS,
        game,
        &game.card_data,
        registry,
    );

    // --- My hand ---
    write_card_ids(
        &mut t,
        OFF_MY_HAND,
        &me.hand,
        MAX_HAND,
        &game.card_data,
        registry,
    );

    // --- Opp hand ---
    write_card_ids(
        &mut t,
        OFF_OPP_HAND,
        &opp.hand,
        MAX_HAND,
        &game.card_data,
        registry,
    );

    // --- My trash ---
    write_card_ids(
        &mut t,
        OFF_MY_TRASH,
        &me.trash,
        MAX_TRASH,
        &game.card_data,
        registry,
    );

    // --- Opp trash ---
    write_card_ids(
        &mut t,
        OFF_OPP_TRASH,
        &opp.trash,
        MAX_TRASH,
        &game.card_data,
        registry,
    );

    // --- My security (face-down = 0.0; face-up cards written when
    // `face_up_security` is populated by reveal effects). Matches Python's
    // `_write_security_ids`.
    write_security_ids(
        &mut t,
        OFF_MY_SECURITY,
        me,
        MAX_SECURITY,
        &game.card_data,
        registry,
    );

    // --- Opp security (face-down = 0.0). Face-up reveals against the
    // opponent populate `opp.face_up_security`, so mirror the my-security
    // writer.
    write_security_ids(
        &mut t,
        OFF_OPP_SECURITY,
        opp,
        MAX_SECURITY,
        &game.card_data,
        registry,
    );

    // --- My breeding ---
    // Breeding slot has no PermanentHandle (it's not in battle_area), so
    // per-source DP and OPT state fall back to 0.0. Python computes them
    // identically since eggs / in-training Digimon rarely have active
    // effects, but this is a minor residual gap for any that do.
    if let Some(ref perm) = me.breeding_area {
        write_slot(
            &mut t,
            OFF_MY_BREEDING,
            perm,
            None,
            game,
            &game.card_data,
            registry,
        );
    }

    // --- Opp breeding ---
    if let Some(ref perm) = opp.breeding_area {
        write_slot(
            &mut t,
            OFF_OPP_BREEDING,
            perm,
            None,
            game,
            &game.card_data,
            registry,
        );
    }

    // --- Revealed cards ---
    write_card_ids(
        &mut t,
        OFF_REVEALED,
        &game.revealed_cards,
        MAX_REVEALED,
        &game.card_data,
        registry,
    );

    // --- Selection context ---
    // Slot +0: phase tensor value (populated whenever the engine is parked
    //   in any selection/interrupt phase, even if pending_selection is None —
    //   so RL policies can observe "we're in SelectTarget" from the phase
    //   signal alone).
    // Slots +1/+2: valid_count (normalized) + selecting_player, populated
    //   only when a PendingSelection is installed. Matches Python's
    //   `tensor.py:108-120`.
    if game.current_phase.is_selection_phase() {
        t[OFF_SELECTION] = game.current_phase.tensor_value();
    }
    if let Some(sel) = &game.pending_selection {
        t[OFF_SELECTION + 1] =
            sel.valid_action_ids.len() as f32 / crate::action::space::ACTION_SPACE_SIZE as f32;
        t[OFF_SELECTION + 2] = sel.selecting_player as f32;
    }

    t
}

// ─── Helpers ──────────────────────────────────────────────────────────

/// Resolve "me" and "opponent" player IDs from the observer's perspective.
fn resolve_perspective(game: &Game, observer: PlayerId) -> (PlayerId, PlayerId) {
    let opp = game.next_clockwise(observer);
    (observer, opp)
}

/// Memory relative to player (positive = their favor).
fn get_memory_for(game: &Game, player_id: PlayerId) -> i16 {
    if player_id == game.turn_player() {
        game.memory
    } else {
        -game.memory
    }
}

/// Write permanent slot data into the tensor.
///
/// When `handle` is `Some`, per-source DP and OPT state are computed via
/// Game helpers (matches Python). When `None` (e.g. the breeding slot —
/// not part of `battle_area`), those fields fall back to 0.0 since there's
/// no stable PermanentHandle for them.
fn write_slot(
    tensor: &mut [f32],
    base: usize,
    perm: &Permanent,
    handle: Option<PermanentHandle>,
    game: &Game,
    card_data: &[CardData],
    registry: &CardRegistry,
) {
    let slot_layout = tensor_profiles::standard::v1::PROFILE.slot_layout;
    let top_card_offset = slot_layout.header_offset(TensorSlotHeaderField::TopCardId);
    let dp_offset = slot_layout.header_offset(TensorSlotHeaderField::Dp);
    let suspended_offset = slot_layout.header_offset(TensorSlotHeaderField::Suspended);
    let opt_total_offset = slot_layout.header_offset(TensorSlotHeaderField::OptTotal);
    let opt_used_offset = slot_layout.header_offset(TensorSlotHeaderField::OptUsed);
    let linked_count_offset = slot_layout.header_offset(TensorSlotHeaderField::LinkedCount);
    let source_count_offset = slot_layout.header_offset(TensorSlotHeaderField::SourceCount);
    let source_card_id_offset = slot_layout.source_offset(TensorSourceField::CardId);
    let source_opt_state_offset = slot_layout.source_offset(TensorSourceField::OptState);
    let source_dp_contribution_offset =
        slot_layout.source_offset(TensorSourceField::DpContribution);
    let top = perm.top_card();

    tensor[base + top_card_offset] = registry.get_index(&top.card_id(card_data)) as f32;

    tensor[base + dp_offset] = perm.base_dp(card_data).unwrap_or(0) as f32 / DP_NORM;

    tensor[base + suspended_offset] = if perm.is_suspended { 1.0 } else { 0.0 };

    if let Some(h) = handle {
        tensor[base + opt_total_offset] = game.opt_total(h) as f32;
        tensor[base + opt_used_offset] = game.opt_used(h) as f32;
    }

    tensor[base + linked_count_offset] = perm.linked_cards.len() as f32;

    tensor[base + source_count_offset] = perm.card_sources.len() as f32;

    // Sources: [card_id, opt_state, dp_contribution] × MAX_SOURCES
    let src_base = base + slot_layout.source_start;
    for (j, src) in perm
        .card_sources
        .iter()
        .take(slot_layout.max_sources)
        .enumerate()
    {
        let off = src_base + j * SOURCE_ENTRY_SIZE;
        tensor[off + source_card_id_offset] = registry.get_index(&src.card_id(card_data)) as f32;
        if let Some(h) = handle {
            tensor[off + source_opt_state_offset] = game.source_opt_state(h, j);
            tensor[off + source_dp_contribution_offset] =
                game.source_dp_contribution(h, j) as f32 / DP_NORM;
        }
    }
}

/// Write a battle area (list of permanents) into the tensor.
fn write_field(
    tensor: &mut [f32],
    start: usize,
    player: PlayerId,
    slots: usize,
    game: &Game,
    card_data: &[CardData],
    registry: &CardRegistry,
) {
    let permanents = &game.player(player).battle_area;
    for (i, perm) in permanents.iter().take(slots).enumerate() {
        let handle = PermanentHandle {
            player,
            index: i as u8,
        };
        write_slot(
            tensor,
            start + i * SLOT_SIZE,
            perm,
            Some(handle),
            game,
            card_data,
            registry,
        );
    }
}

/// Write a player's security stack into the tensor. Only face-up cards
/// (those whose `card_index` is in `player.face_up_security`) emit their
/// registry index; face-down slots stay 0.0. Mirrors Python's
/// `_write_security_ids`.
fn write_security_ids(
    tensor: &mut [f32],
    start: usize,
    player: &Player,
    limit: usize,
    card_data: &[CardData],
    registry: &CardRegistry,
) {
    for (i, card) in player.security.iter().take(limit).enumerate() {
        if player.face_up_security.contains(&card.card_index) {
            tensor[start + i] = registry.get_index(&card.card_id(card_data)) as f32;
        }
    }
}

/// Write card IDs from a card list into the tensor (1 float per card).
fn write_card_ids(
    tensor: &mut [f32],
    start: usize,
    cards: &[crate::card_source::CardSource],
    limit: usize,
    card_data: &[CardData],
    registry: &CardRegistry,
) {
    for (i, card) in cards.iter().take(limit).enumerate() {
        tensor[start + i] = registry.get_index(&card.card_id(card_data)) as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Task 2.2: the relocated v1 builder must produce the same shape and
    /// the same observable layout offsets as the pre-move baseline. Deeper
    /// numeric parity is asserted by the integration suite under
    /// `tests/mask_and_tensor/`, which exercises the full v1 layout against
    /// hand-built game states.
    #[test]
    fn relocated_builder_yields_v1_tensor_size() {
        // We don't construct a full Game here — that path needs the engine's
        // card-data store and would duplicate test scaffolding from
        // `tests/infra/`. Instead, assert the layout constants the builder
        // closes over still resolve to v1 values.
        assert_eq!(TENSOR_SIZE, 1375, "v1 TENSOR_SIZE drifted");
        assert_eq!(FIELD_SLOTS, 14, "v1 FIELD_SLOTS drifted");
        assert_eq!(SLOT_SIZE, 40, "v1 SLOT_SIZE drifted");
        assert_eq!(MAX_HAND, 20, "v1 MAX_HAND drifted");
        assert_eq!(MAX_TRASH, 45, "v1 MAX_TRASH drifted");
        assert_eq!(MAX_SECURITY, 10, "v1 MAX_SECURITY drifted");
        assert_eq!(MAX_REVEALED, 10, "v1 MAX_REVEALED drifted");
        // Section offsets — the layout constants used by the writer.
        assert_eq!(OFF_MY_BATTLE, 10);
        assert_eq!(OFF_OPP_BATTLE, 570);
        assert_eq!(OFF_MY_HAND, 1130);
        assert_eq!(OFF_OPP_HAND, 1150);
        assert_eq!(OFF_MY_TRASH, 1170);
        assert_eq!(OFF_OPP_TRASH, 1215);
        assert_eq!(OFF_MY_SECURITY, 1260);
        assert_eq!(OFF_OPP_SECURITY, 1270);
        assert_eq!(OFF_MY_BREEDING, 1280);
        assert_eq!(OFF_OPP_BREEDING, 1320);
        assert_eq!(OFF_REVEALED, 1360);
        assert_eq!(OFF_SELECTION, 1370);
    }
}
