//! TEST-021: "[Security] Play this card without paying its cost."
//! Pilot for `play_pending_security` — exercises the `security_played` bit
//! so the revealed card stays on the defender's field instead of being
//! trashed. Exercises the `pending_security` transient state.
//!
//! Note: `play_pending_security` was renamed from `play_from_security` in
//! Phase 2f1 Task 3a to disambiguate from the new
//! `EffectContext::play_from_security(player)` (top-of-security-stack
//! play, BT12-091 et al.).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test021;

impl CardEffect for Test021 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("[Security] Play self without paying cost")
            .process(|ctx| {
                ctx.play_pending_security();
            })
            .build()]
    }
}
