//! TEST-021: "[Security] Play this card without paying its cost."
//! Pilot for `play_from_security` — exercises the `security_played` bit
//! so the revealed card stays on the defender's field instead of being
//! trashed. Exercises the `pending_security` transient state.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test021;

impl CardEffect for Test021 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("[Security] Play self without paying cost")
            .process(|ctx| {
                ctx.play_from_security();
            })
            .build()]
    }
}
