//! Tensor building for the RL observation space.
//!
//! Produces a flat `f32` tensor from one player's perspective.
//! Card identities are integer registry indices (float-cast).
//! The `nn.Embedding` lookup happens inside the features extractor on the GPU.

use crate::card_data::CardData;
use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::permanent::{Permanent, PermanentHandle};
use crate::player::Player;
use crate::tensor_profile;

// ─── Tensor Layout Constants ──────────────────────────────────────────

pub const FIELD_SLOTS: usize = 14;
pub const MAX_HAND: usize = 20;
pub const MAX_TRASH: usize = 45;
pub const MAX_SECURITY: usize = 10;
pub const MAX_SOURCES: usize = 11;
pub const MAX_REVEALED: usize = 10;

pub const SOURCE_ENTRY_SIZE: usize = 3; // card_id + opt_state + dp_contribution
pub const SLOT_HEADER_SIZE: usize = 7; // top card ID + 6 scalar fields
pub const SLOT_SIZE: usize = SLOT_HEADER_SIZE + MAX_SOURCES * SOURCE_ENTRY_SIZE; // 40

pub const DP_NORM: f32 = 30000.0;

// Section sizes
pub const GLOBAL_SIZE: usize = 10;
pub const BATTLE_SIZE: usize = FIELD_SLOTS * SLOT_SIZE; // 560
pub const HAND_SIZE: usize = MAX_HAND; // 20
pub const TRASH_SIZE: usize = MAX_TRASH; // 45
pub const SECURITY_SIZE: usize = MAX_SECURITY; // 10
pub const BREEDING_SIZE: usize = SLOT_SIZE; // 40
pub const REVEALED_SIZE: usize = MAX_REVEALED; // 10
pub const SELECTION_SIZE: usize = 5;

/// Total tensor size: 10 + 560 + 560 + 20 + 20 + 45 + 45 + 10 + 10 + 40 + 40 + 10 + 5 = 1375
pub const TENSOR_SIZE: usize = GLOBAL_SIZE
    + BATTLE_SIZE * 2
    + HAND_SIZE * 2
    + TRASH_SIZE * 2
    + SECURITY_SIZE * 2
    + BREEDING_SIZE * 2
    + REVEALED_SIZE
    + SELECTION_SIZE;

// Section start offsets
pub const OFF_GLOBAL: usize = 0;
pub const OFF_MY_BATTLE: usize = OFF_GLOBAL + GLOBAL_SIZE; // 10
pub const OFF_OPP_BATTLE: usize = OFF_MY_BATTLE + BATTLE_SIZE; // 570
pub const OFF_MY_HAND: usize = OFF_OPP_BATTLE + BATTLE_SIZE; // 1130
pub const OFF_OPP_HAND: usize = OFF_MY_HAND + HAND_SIZE; // 1150
pub const OFF_MY_TRASH: usize = OFF_OPP_HAND + HAND_SIZE; // 1170
pub const OFF_OPP_TRASH: usize = OFF_MY_TRASH + TRASH_SIZE; // 1215
pub const OFF_MY_SECURITY: usize = OFF_OPP_TRASH + TRASH_SIZE; // 1260
pub const OFF_OPP_SECURITY: usize = OFF_MY_SECURITY + SECURITY_SIZE; // 1270
pub const OFF_MY_BREEDING: usize = OFF_OPP_SECURITY + SECURITY_SIZE; // 1280
pub const OFF_OPP_BREEDING: usize = OFF_MY_BREEDING + BREEDING_SIZE; // 1320
pub const OFF_REVEALED: usize = OFF_OPP_BREEDING + BREEDING_SIZE; // 1360
pub const OFF_SELECTION: usize = OFF_REVEALED + REVEALED_SIZE; // 1370

// ─── Tensor Builder ───────────────────────────────────────────────────

/// Build a flat observation tensor from the given player's perspective.
///
/// Layout (TENSOR_SIZE=1375):
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
pub fn build_tensor(game: &Game, player_id: PlayerId, registry: &CardRegistry) -> Vec<f32> {
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

// ─── Tensor Layout Metadata ───────────────────────────────────────────

/// Compute which tensor positions hold card IDs vs scalar values.
/// Used by the features extractor to split for embedding lookup.
pub fn compute_positions() -> (Vec<usize>, Vec<usize>) {
    tensor_profile::standard_v1_positions()
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
    let top = perm.top_card();

    // +0: top card ID
    tensor[base] = registry.get_index(&top.card_id(card_data)) as f32;

    // +1: DP (normalized)
    tensor[base + 1] = perm.base_dp(card_data).unwrap_or(0) as f32 / DP_NORM;

    // +2: suspended
    tensor[base + 2] = if perm.is_suspended { 1.0 } else { 0.0 };

    // +3: OPT total, +4: OPT used — raw counts (Python matches).
    if let Some(h) = handle {
        tensor[base + 3] = game.opt_total(h) as f32;
        tensor[base + 4] = game.opt_used(h) as f32;
    }

    // +5: linked card count
    tensor[base + 5] = perm.linked_cards.len() as f32;

    // +6: source count
    tensor[base + 6] = perm.card_sources.len() as f32;

    // Sources: [card_id, opt_state, dp_contribution] × MAX_SOURCES
    let src_base = base + SLOT_HEADER_SIZE;
    for (j, src) in perm.card_sources.iter().take(MAX_SOURCES).enumerate() {
        let off = src_base + j * SOURCE_ENTRY_SIZE;
        tensor[off] = registry.get_index(&src.card_id(card_data)) as f32;
        if let Some(h) = handle {
            tensor[off + 1] = game.source_opt_state(h, j);
            tensor[off + 2] = game.source_dp_contribution(h, j) as f32 / DP_NORM;
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

    #[test]
    fn tensor_size_is_1375() {
        assert_eq!(TENSOR_SIZE, 1375);
    }

    #[test]
    fn slot_size_is_40() {
        assert_eq!(SLOT_SIZE, 40);
    }

    #[test]
    fn section_offsets() {
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

    #[test]
    fn positions_cover_all_indices() {
        let (card_pos, scalar_pos) = compute_positions();
        assert_eq!(
            card_pos.len() + scalar_pos.len(),
            TENSOR_SIZE,
            "card positions ({}) + scalar positions ({}) must equal TENSOR_SIZE ({})",
            card_pos.len(),
            scalar_pos.len(),
            TENSOR_SIZE
        );
        // No overlap
        let mut all: Vec<usize> = card_pos.iter().chain(scalar_pos.iter()).copied().collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), TENSOR_SIZE);
        // Contiguous 0..TENSOR_SIZE
        assert_eq!(*all.first().unwrap(), 0);
        assert_eq!(*all.last().unwrap(), TENSOR_SIZE - 1);
    }
}
