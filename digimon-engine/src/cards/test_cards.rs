//! Hand-written test cards used to validate the effect system.
//!
//! These are not real Digimon cards — they're synthetic cards with simple
//! effects that exercise specific code paths in the engine.

use std::sync::Arc;

use crate::card_source::CardHandle;
use crate::cards::CardEffectRegistry;
use crate::effect::{CardEffect, Effect};
use crate::enums::{Expiry, ModifierType};

/// TEST-001: "On Play: Gain 1 memory."
/// Exercises basic OnPlay effect with a memory mutation.
pub struct Test001;

impl CardEffect for Test001 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Gain 1 memory")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// TEST-002: "On Play: Draw 2 cards."
/// Exercises card draw via EffectContext.
pub struct Test002;

impl CardEffect for Test002 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Draw 2")
            .process(|ctx| {
                let me = ctx.player;
                ctx.draw(me, 2);
            })
            .build()]
    }
}

/// TEST-003: "On Play: All your Digimon get +1000 DP for the turn."
/// Exercises modifier registration with end-of-turn expiry.
pub struct Test003;

impl CardEffect for Test003 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Buff allies +1000 DP")
            .process(|ctx| {
                let me = ctx.player;
                let count = ctx.battle_area(me).len();
                for i in 0..count {
                    let h = crate::permanent::PermanentHandle {
                        player: me,
                        index: i as u8,
                    };
                    ctx.add_dp_modifier(h, 1000, Expiry::EndOfTurn);
                }
            })
            .build()]
    }
}

/// TEST-004: "When Digivolving: Gain 2 memory if opponent has any Digimon."
/// Exercises WhenDigivolving timing with a condition closure.
pub struct Test004;

impl CardEffect for Test004 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("Gain 2 memory if opp has Digimon")
            .condition(|ctx| {
                let opp = ctx.opponent_id();
                ctx.battle_area(opp).iter().any(|p| p.is_digimon(ctx.card_data()))
            })
            .process(|ctx| {
                ctx.gain_memory(2);
            })
            .build()]
    }
}

/// TEST-005: "On Deletion: Lose 1 memory."
/// Exercises OnDeletion timing.
pub struct Test005;

impl CardEffect for Test005 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("Lose 1 memory on deletion")
            .process(|ctx| {
                ctx.lose_memory(1);
            })
            .build()]
    }
}

/// TEST-006: "End of your turn: Gain 5 memory."
/// Exercises EndOfYourTurn timing and memory swing-back (§1.5).
pub struct Test006;

impl CardEffect for Test006 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("Gain 5 memory at end of turn")
            .process(|ctx| {
                ctx.gain_memory(5);
            })
            .build()]
    }
}

/// Register all test cards into the registry.
pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert("TEST-001", Arc::new(Test001));
    registry.insert("TEST-002", Arc::new(Test002));
    registry.insert("TEST-003", Arc::new(Test003));
    registry.insert("TEST-004", Arc::new(Test004));
    registry.insert("TEST-005", Arc::new(Test005));
    registry.insert("TEST-006", Arc::new(Test006));

    // Suppress unused warning on ModifierType (referenced via add_dp_modifier helper).
    let _ = ModifierType::ChangeDp;
}
