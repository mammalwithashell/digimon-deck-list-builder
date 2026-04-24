//! TEST-026: "[Security] Gain 3 memory" — duplicate of TEST-022's shape
//! used by the §2.5c Progress-skip pilot. When the attacker has
//! `Keyword::Progress` or `ModifierType::ImmunityToOpponentEffects`, this
//! effect must NOT fire (the observer is skipped entirely in the
//! `SecuritySkillDrain` phase).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test026;

impl CardEffect for Test026 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("[Security] Gain 3 memory (Progress-pilot)")
            .process(|ctx| ctx.gain_memory(3))
            .build()]
    }
}
