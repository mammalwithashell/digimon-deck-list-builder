//! Default observation tensor entry point.
//!
//! `tensor::build_tensor` and `tensor::TENSOR_SIZE` report the engine's
//! canonical default profile — currently `standard_lite_deck_v2` (8850).
//! Profile-explicit callers should reach for
//! `observation::build_observation_tensor(.., ObservationProfileId::*)` or
//! the profile-specific builders (`tensor_v1::build_tensor_standard_compact_v1`,
//! `tensor_v2_lite::build_tensor_standard_lite_v2`,
//! `tensor_v2_lite_deck::build_tensor_standard_lite_deck_v2`,
//! `tensor_v2_full::build_tensor_standard_full_v2`).
//!
//! Card identities are integer registry indices (float-cast); the
//! `nn.Embedding` lookup happens inside the features extractor on the GPU.

use crate::card_registry::CardRegistry;
use crate::enums::PlayerId;
use crate::game::Game;
use crate::observation::{build_observation_tensor, default_observation_profile};

// ─── Default Profile Re-exports ───────────────────────────────────────
//
// Top-level `TENSOR_SIZE` constant agrees with `default_profile()` per the
// `engine-default-observation-profile` spec. Other layout constants that
// used to be re-exported (v1's `SLOT_SIZE`, `OFF_MY_BATTLE`, etc.) are now
// reachable only via their explicit module paths
// (`tensor_profiles::standard::v1::*`) — they describe the v1 layout, not
// the engine default, and confusing the two led to a real bug class
// (desktop builds rejecting modern models at the v1-shaped gate).

pub use crate::tensor_profiles::standard::v2_lite_deck::TENSOR_SIZE;

/// Per-DP normalization constant (shared across all standard profiles).
pub const DP_NORM: f32 = 30000.0;

// ─── Tensor Builder ───────────────────────────────────────────────────

/// Build a flat observation tensor from the given player's perspective
/// against the engine default profile.
///
/// This is a thin wrapper over the observation dispatcher; callers that
/// want a specific profile should call
/// `observation::build_observation_tensor(.., ObservationProfileId::*)`
/// directly and not rely on the engine default.
pub fn build_tensor(game: &Game, player_id: PlayerId, registry: &CardRegistry) -> Vec<f32> {
    build_observation_tensor(game, player_id, registry, default_observation_profile())
}

// ─── Tensor Layout Metadata ───────────────────────────────────────────

/// Compute which tensor positions hold card IDs vs scalar values for the
/// engine default profile.
pub fn compute_positions() -> (Vec<usize>, Vec<usize>) {
    crate::tensor_profiles::standard::DEFAULT_PROFILE.positions()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor_profiles;

    #[test]
    fn tensor_size_matches_default_profile() {
        assert_eq!(TENSOR_SIZE, tensor_profiles::default_profile().tensor_size);
    }

    #[test]
    fn default_profile_is_standard_lite_deck_v2() {
        let p = tensor_profiles::default_profile();
        assert_eq!(p.id, "standard_lite_deck_v2");
    }

    #[test]
    fn tensor_size_is_8850() {
        assert_eq!(TENSOR_SIZE, 8850);
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
        let mut all: Vec<usize> = card_pos.iter().chain(scalar_pos.iter()).copied().collect();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), TENSOR_SIZE);
        assert_eq!(*all.first().unwrap(), 0);
        assert_eq!(*all.last().unwrap(), TENSOR_SIZE - 1);
    }
}
