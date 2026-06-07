//! Security-stack placement operations (Tier 2) — `impl Game`.

#![allow(unused_imports)]
use super::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;
use rand::seq::SliceRandom;

impl Game {
    /// Move a card from `source` to `player_id`'s security stack at the given
    /// `position` (Top, Bottom, Random). If `face_up` is true, the card's
    /// `card_index` is inserted into `face_up_security` so subsequent reveals
    /// know it was placed face-up. Returns false if the source index is invalid.
    ///
    /// Does not fire `OnLoseSecurity`; successful placements fire
    /// `OnPlaceSecurity` observers after the card reaches the security stack.
    ///
    /// Phase 7 Task 4: fires `WhenWouldPlaceInSecurity` at entry. Subject
    /// carries the card handle via the source zone; cause is inferred.
    /// v1 redirect accepts `Zone::Trash` only (card goes to trash instead of
    /// the security stack); other redirect destinations are a `debug_assert!`
    /// + fallthrough.
    pub fn place_on_security(
        &mut self,
        player_id: PlayerId,
        source: crate::enums::CardSourceRef,
        position: crate::enums::StackPosition,
        face_up: bool,
    ) -> bool {
        self.place_on_security_observed(player_id, source, position, face_up, player_id)
    }
}
