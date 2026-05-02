//! TEST-027: Inherited `+dp_modifier` applied to the opposing security
//! Digimon's DP during the security battle (§2.5e). Used as a digivolution
//! source under the attacker — its effect carries the
//! `applies_to_opponent_security_dp` flag so `attacker_security_dp_adjustment`
//! picks it up. Negative `dp_modifier` lowers the security Digimon's DP
//! (favoring the attacker) — this mirrors Python's sign convention.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test027;

impl CardEffect for Test027 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::inherited(card)
            .name("[Inherited] Opposing security Digimon -3000 DP")
            .dp_modifier(-3000)
            .applies_to_opponent_security_dp()
            .build()]
    }
}
