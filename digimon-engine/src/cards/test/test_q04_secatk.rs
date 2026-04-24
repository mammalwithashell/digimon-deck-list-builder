//! TEST-Q04-SECATK: "On Play: This Digimon gets [Security Attack +1]
//! (Permanent)."
//!
//! Distilled from ZXavier's Digi Rulings quiz question 4 (Aldamon's
//! "Atomic Inferno / Holy Flame" interaction): does the attacker do an
//! *additional* security check after the first one resolves? The answer
//! hinges on rule 16-3 — [Security Attack +X] is a persistent effect
//! that adds X security checks to the attack. The synthetic card here
//! bakes that into an OnPlay modifier so a test can drive it through
//! the combat path without pulling in a real production card.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::{Expiry, ModifierType};

pub struct TestQ04Secatk;

impl CardEffect for TestQ04Secatk {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Security Attack +1 (permanent)")
            .process(|ctx| {
                if let Some(me) = ctx.source_permanent {
                    ctx.add_modifier(me, ModifierType::SecurityAttackChange, 1, Expiry::Permanent);
                }
            })
            .build()]
    }
}
