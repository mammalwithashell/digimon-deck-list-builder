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

/// TEST-007: "End of your turn (Optional): Gain 2 memory."
/// Exercises `TriggerOrder` + decline-all with an optional queued effect.
pub struct Test007;

impl CardEffect for Test007 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("(Optional) Gain 2 memory at end of turn")
            .optional()
            .process(|ctx| {
                ctx.gain_memory(2);
            })
            .build()]
    }
}

/// TEST-011: "On Play: Trash 1 card from your hand, then draw 2."
/// Pilot for `select_hand` — mandatory (caller must pick a hand card),
/// with a trash-then-draw side effect inside the callback.
pub struct Test011;

impl CardEffect for Test011 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Trash 1 from hand, draw 2")
            .process(|ctx| {
                ctx.select_hand(
                    ctx.player,
                    "Trash a card from your hand",
                    /* is_optional = */ false,
                    |_game, _i| true, // any hand card is eligible
                    |ctx, hand_index| {
                        let me = ctx.player;
                        let player = ctx.game.player_mut(me);
                        if hand_index < player.hand.len() {
                            let card = player.hand.remove(hand_index);
                            player.trash.push(card);
                        }
                        ctx.draw(me, 2);
                    },
                );
            })
            .build()]
    }
}

/// TEST-012: "On Play (Choose one): Gain 2 memory / Draw 2 cards."
/// Pilot for `select_effect_choice` — mandatory branch pick with no PASS.
pub struct Test012;

impl CardEffect for Test012 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Choose: gain 2 memory / draw 2")
            .process(|ctx| {
                ctx.select_effect_choice(
                    "Choose one",
                    vec!["Gain 2 memory".to_string(), "Draw 2 cards".to_string()],
                    |ctx, choice| match choice {
                        0 => ctx.gain_memory(2),
                        1 => {
                            let me = ctx.player;
                            ctx.draw(me, 2);
                        }
                        _ => {}
                    },
                );
            })
            .build()]
    }
}

/// TEST-013: Blast-digivolve pilot.
/// Carries a single `.blast_digivolve()` flag + a `WhenDigivolving` hook
/// that grants +1 memory so tests can verify the post-stack trigger
/// fires. The card's `EvoCost` (populated via `make_test_blast_card` in
/// the test harness) controls what field targets are eligible.
pub struct Test013;

impl CardEffect for Test013 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("Blast-digivolve pilot (+1 memory)")
            .blast_digivolve()
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

/// TEST-014: Blast-digivolve pilot whose WhenDigivolving deletes the
/// current attacker. Used to test the Counter cascade where the blast
/// kills the attacker mid-attack, forcing the state machine to skip
/// BlockOpen/Battle and land in Cleanup.
pub struct Test014;

impl CardEffect for Test014 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("Blast: delete the attacker")
            .blast_digivolve()
            .process(|ctx| {
                if let Some(attacker) = ctx.game.pending_attack.as_ref().map(|pa| pa.attacker) {
                    ctx.delete_permanent(attacker);
                }
            })
            .build()]
    }
}

/// TEST-010: "On Play: Delete 1 of your opponent's Digimon. (Optional)"
/// Pilot for the selection subsystem — exercises `select_opponent_permanent`
/// with a mandatory/optional flip and a `delete_permanent` mutation inside
/// the resolution callback.
pub struct Test010;

impl CardEffect for Test010 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delete 1 opp Digimon (optional)")
            .process(|ctx| {
                ctx.select_opponent_permanent(
                    "Delete 1 of your opponent's Digimon",
                    /* is_optional = */ true,
                    // Only Digimon are valid targets — Tamer slots are skipped.
                    |game, handle| {
                        game.player(handle.player)
                            .battle_area
                            .get(handle.index as usize)
                            .map(|p| p.is_digimon(&game.card_data))
                            .unwrap_or(false)
                    },
                    |ctx, chosen| {
                        ctx.delete_permanent(chosen);
                    },
                );
            })
            .build()]
    }
}

/// TEST-020: "[Security] Draw 2 cards."
/// Pilot for `SecuritySkill` trigger-and-trash — exercises the
/// security-reveal → enqueue → drain → trash pipeline with a simple
/// draw effect. The revealed card ends up in the defender's trash.
pub struct Test020;

impl CardEffect for Test020 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("[Security] Draw 2")
            .process(|ctx| {
                let me = ctx.player;
                ctx.draw(me, 2);
            })
            .build()]
    }
}

/// TEST-021: "[Security] Play this card without paying its cost."
/// Pilot for `play_from_security` — exercises the `security_played` bit
/// so the revealed card stays on the defender's field instead of being
/// trashed. Exercises the `pending_security` transient state.
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

/// TEST-022: "[Security] Gain 3 memory."
/// Pilot for observer-timing parity — a second non-destructive effect
/// so the harness can distinguish "effect fired" (memory changed) from
/// "effect fired AND card is on field" (TEST-021's signature). Trashes
/// the revealed card after firing.
pub struct Test022;

impl CardEffect for Test022 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("[Security] Gain 3 memory")
            .process(|ctx| {
                ctx.gain_memory(3);
            })
            .build()]
    }
}

/// TEST-008: "End of your turn: Lose 3 memory."
/// Paired with TEST-006 in drainer tests to disambiguate resolution
/// order from final-state aggregates.
pub struct Test008;

impl CardEffect for Test008 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("Lose 3 memory at end of turn")
            .process(|ctx| {
                ctx.lose_memory(3);
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
    registry.insert("TEST-007", Arc::new(Test007));
    registry.insert("TEST-008", Arc::new(Test008));
    registry.insert("TEST-010", Arc::new(Test010));
    registry.insert("TEST-011", Arc::new(Test011));
    registry.insert("TEST-012", Arc::new(Test012));
    registry.insert("TEST-013", Arc::new(Test013));
    registry.insert("TEST-014", Arc::new(Test014));
    registry.insert("TEST-020", Arc::new(Test020));
    registry.insert("TEST-021", Arc::new(Test021));
    registry.insert("TEST-022", Arc::new(Test022));

    // Suppress unused warning on ModifierType (referenced via add_dp_modifier helper).
    let _ = ModifierType::ChangeDp;
}
