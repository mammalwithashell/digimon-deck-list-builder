//! Official 16-7 and Bandai CRM4 clarification: an effect battle during an
//! attack can earn Piercing, but there is only one security transition.
//! https://en.digimoncard.com/rule/revised/ ("May battle" Effects and Piercing).
//! Ending an attack skips to EndOfAttack: 15-15-1-1; official Q5804.
use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{Expiry, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::game::Game;
use digimon_engine::selection::SelectionKind;
use std::sync::Arc;

/// The effect battle is part of the On Attack effect's processing, so the
/// battle's `<Piercing>` (a battle-result trigger) and the end-of-battle reward
/// pend in ONE pool once that effect finishes (CR 4.3 15-8-3-2, 15-4-3-2) and
/// the turn player chooses their order (15-4-3-5-1). Resolve that order with
/// `<Piercing>` first or second; every branch must give the same outcome.
fn order_pool(game: &mut Game, piercing_first: bool) {
    let selection = game.pending_selection.as_ref().expect("pooled triggers");
    assert_eq!(selection.kind, SelectionKind::TriggerOrder);
    let choices = selection.effect_choices.as_ref().expect("order entries");
    assert_eq!(choices.len(), 2, "Piercing and the battle reward");
    assert_eq!(
        choices.iter().filter(|c| c.keyword == Some(Keyword::Piercing)).count(),
        1
    );
    let action = choices
        .iter()
        .find(|c| (c.keyword == Some(Keyword::Piercing)) == piercing_first)
        .unwrap()
        .action_id;
    let actor = selection.selecting_player;
    assert!(build_action_mask(game, actor)[action as usize] > 0.5);
    game.resolve_selection(actor, action).unwrap();
}

/// Answer the battle reward's optional prompt (accept or decline).
fn answer_reward(game: &mut Game, accept: bool) {
    let selection = game.pending_selection.as_ref().expect("battle reward");
    assert!(build_action_mask(game, selection.selecting_player)[PASS as usize] > 0.5, "reward is optional");
    let actor = selection.selecting_player;
    let action = if accept {
        *selection
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .unwrap()
    } else {
        PASS
    };
    assert!(build_action_mask(game, actor)[action as usize] > 0.5);
    game.resolve_selection(actor, action).unwrap();
}

struct BattleOnAttack {
    other: bool,
    reward: bool,
    remove_winner: bool,
    cancel: bool,
}
impl CardEffect for BattleOnAttack {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let other = self.other;
        let remove_winner = self.remove_winner;
        let cancel = self.cancel;
        let mut effects = vec![Effect::on_attack(card)
            .process(move |ctx| {
                let fighter = if other {
                    PermanentHandle {
                        player: 0,
                        index: 1,
                    }
                } else {
                    ctx.source_permanent.unwrap()
                };
                ctx.battle_digimon(
                    fighter,
                    PermanentHandle {
                        player: 1,
                        index: 0,
                    },
                );
            })
            .build()];
        if self.reward {
            effects.push(
                Effect::end_of_battle(card)
                    .name("battle win reward")
                    .condition(|ctx| {
                        ctx.battle_winner() == ctx.source_permanent
                            && ctx
                                .game
                                .current_trigger_context
                                .as_ref()
                                .is_some_and(|t| !t.battle_deleters.is_empty())
                    })
                    .optional()
                    .needs_outer_optional_prompt()
                    .process(move |ctx| {
                        if cancel {
                            ctx.game.cancel_pending_attack_from_effect();
                        } else if remove_winner {
                            ctx.delete_permanent(ctx.source_permanent.unwrap());
                        } else {
                            ctx.game.memory += 1;
                        }
                    })
                    .build(),
            );
        }
        effects
    }
}
fn setup(other: bool, reward: bool) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut attacker = make_test_card("ATK", "Attacker");
    attacker.dp = Some(12000);
    let mut victim = make_test_card("DEF", "Victim");
    victim.dp = Some(1000);
    let mut r = DebugRunner::builder()
        .add_card(attacker)
        .add_card(victim)
        .add_card(make_test_card("SEC", "Security"))
        .deck(0, &["SEC"; 8])
        .deck(1, &["SEC"; 8])
        .security(1, &["SEC"; 5])
        .memory(5)
        .start();
    r.register_effect(
        "ATK",
        Arc::new(BattleOnAttack {
            other,
            reward,
            remove_winner: false,
            cancel: false,
        }),
    );
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Piercing, Expiry::Permanent, 0);
    if other {
        let partner = r.place_on_field(0, "ATK", Some(0));
        r.game
            .modifiers
            .grant_keyword(partner, Keyword::Piercing, Expiry::Permanent, 0);
    }
    let target = r.place_on_field(1, "DEF", Some(0));
    r.game.players[1].battle_area[0].is_suspended = true;
    (r, atk, target)
}
#[test]
fn attacking_effect_battler_checks_after_target_disappears_and_cloned_reward() {
    let (mut r, atk, target) = setup(false, true);
    r.attack_digimon(atk, target, false);
    assert_eq!(
        r.game.players[1].security.len(),
        5,
        "wait for battle reward"
    );
    let snapshot = r.game.clone();
    for accept in [true, false] {
        for piercing_first in [true, false] {
            let mut game = snapshot.clone();
            order_pool(&mut game, piercing_first);
            assert_eq!(game.players[1].security.len(), 5, "check waits for the pool");
            answer_reward(&mut game, accept);
            assert!(game.pending_selection.is_none());
            assert_eq!(
                game.players[1].security.len(),
                4,
                "owed Piercing survives vanished target"
            );
            assert_eq!(game.memory, snapshot.memory + i16::from(accept));
            assert!(game.pending_attack.is_none());
        }
    }
}
#[test]
fn effect_battle_then_player_attack_has_only_one_security_transition() {
    let (mut r, atk, _) = setup(false, false);
    r.attack_player(atk, 1, false);
    assert_eq!(r.game.players[1].security.len(), 4);
    assert!(r.game.pending_attack.is_none());
}
#[test]
fn nonattacking_effect_battler_does_not_grant_attacker_piercing() {
    let (mut r, atk, target) = setup(true, false);
    r.attack_digimon(atk, target, false);
    assert_eq!(r.game.players[1].security.len(), 5);
}
#[test]
fn effect_battle_then_normal_battle_still_checks_only_once() {
    let (mut r, atk, _) = setup(false, false);
    let second = r.place_on_field(1, "DEF", Some(0));
    r.game.players[1].battle_area[1].is_suspended = true;
    r.attack_digimon(atk, second, false);
    assert_eq!(r.game.players[1].security.len(), 4);
    assert!(r.game.pending_attack.is_none());
}

#[test]
fn removed_attacker_cannot_process_owed_piercing_on_clone() {
    let (mut r, atk, target) = setup(false, true);
    r.register_effect(
        "ATK",
        Arc::new(BattleOnAttack {
            other: false,
            reward: true,
            remove_winner: true,
            cancel: false,
        }),
    );
    r.attack_digimon(atk, target, false);
    let snapshot = r.game.clone();
    for piercing_first in [true, false] {
        let mut game = snapshot.clone();
        order_pool(&mut game, piercing_first);
        answer_reward(&mut game, true);
        assert!(game.pending_selection.is_none());
        assert!(game.players[0].battle_area.is_empty());
        assert_eq!(game.players[1].security.len(), 5);
        assert!(game.pending_attack.is_none());
    }
}

#[test]
fn ended_attack_skips_owed_piercing_on_clone() {
    let (mut r, atk, target) = setup(false, true);
    r.register_effect(
        "ATK",
        Arc::new(BattleOnAttack {
            other: false,
            reward: true,
            remove_winner: false,
            cancel: true,
        }),
    );
    r.attack_digimon(atk, target, false);
    let snapshot = r.game.clone();
    for piercing_first in [true, false] {
        let mut game = snapshot.clone();
        order_pool(&mut game, piercing_first);
        answer_reward(&mut game, true);
        assert!(game.pending_selection.is_none());
        assert_eq!(game.players[1].security.len(), 5);
        assert!(game.pending_attack.is_none());
    }
}
