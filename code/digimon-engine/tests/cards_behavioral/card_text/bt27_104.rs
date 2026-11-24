//! Source: `data/card_bundles/BT27-104.md` (printed text).
//!
//! [Start of Your Main Phase] By placing 1 card from your hand under this Tamer, ＜Draw 1＞ and gain 1 memory.
//! [End of Your Turn] By suspending this Tamer, activate 1 of the effects below:
//! ・1 of your Digimon may digivolve into a [Royal Knight] trait card under this Tamer with the cost reduced by 2.
//! ・You may use 1 use cost 5 or higher [Royal Knight] trait Option card from under this Tamer with the cost reduced by 3.
//! (Rule) Name: Also treated as [King Drasil_7D6].
//! [Security] Play this card without paying the cost.

use super::common::*;
use digimon_engine::enums::{Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;
use digimon_engine::owned_card_choice::CardChoiceOperation;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-104";

#[test]
fn main_phase_stash_payment_is_optional_and_gives_draw_and_memory() {
    for accept in [false, true] {
        let mut r = builder(CARD).hand(0, &["TEXT-3"]).memory(5).start();
        let tamer = r.place_on_field(0, CARD, Some(0));
        r.game.enter_main_phase();
        r.assert_prompt(SelectionKind::Hand, true, 0);
        if accept {
            r.choose_prompt_card("TEXT-3", None).unwrap();
            assert_eq!(r.memory(), 6);
            assert_eq!(
                ids(
                    &r,
                    &r.game.players[0].battle_area[tamer.index as usize].card_sources
                ),
                ["TEXT-3", CARD]
            );
            assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-FILLER"]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.memory(), 5);
            assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-3"]);
        }
    }
}

#[test]
fn end_of_turn_evolution_only_offers_receivers_for_sources_under_this_tamer() {
    let mut knight = digimon("ROYAL-5", 5, 7000, false);
    knight.traits.push("Royal Knight".into());
    let mut elsewhere = digimon("ROYAL-4", 4, 5000, false);
    elsewhere.traits.push("Royal Knight".into());
    let mut r = start(builder(CARD).add_card(knight).add_card(elsewhere));
    let tamer = r.place_stack(0, &["ROYAL-5", CARD]);
    r.place_stack(0, &["ROYAL-4", "TEXT-TAMER"]);
    let host = r.place_on_field(0, "TEXT-4", Some(0));
    r.place_on_field(0, "TEXT-3", Some(0));
    r.pass_turn();
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    assert_eq!(candidates(&r), ["TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    r.assert_prompt(
        SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        },
        false,
        0,
    );
    let picks = r.pending_selection_view().unwrap().valid_action_ids;
    assert_eq!(picks.len(), 1);
    action(&mut r, 0, picks[0]);
    assert_eq!(r.dp_of(host), Some(7000));
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(r.memory(), 5);
}

#[test]
fn evolution_probe_checks_payment_and_can_use_full_cost_if_reduction_is_blocked() {
    for (printed, blocked, payable) in [(9, false, true), (9, true, false), (4, true, true)] {
        let mut knight = digimon("ROYAL-5", 5, 7000, false);
        knight.traits.push("Royal Knight".into());
        for route in &mut knight.evo_costs {
            route.memory_cost = printed;
        }
        let mut r = start(builder(CARD).add_card(knight));
        r.place_stack(0, &["ROYAL-5", CARD]);
        let host = r.place_on_field(0, "TEXT-4", Some(0));
        if blocked {
            r.game.modifiers.add_player_modifier(
                0,
                PlayerModifierEntry::simple(
                    ModifierType::CannotReduceCost,
                    0,
                    Expiry::Permanent,
                    None,
                    0,
                ),
            );
        }
        r.pass_turn();
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        r.execute_branch(0).unwrap();
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(0).unwrap();
        if payable {
            r.assert_prompt(SelectionKind::OwnField, true, 0);
            r.choose_prompt_card("TEXT-4", None).unwrap();
            r.assert_prompt(
                SelectionKind::SourceMulti {
                    min: 1,
                    max: 1,
                    picked: 0,
                },
                false,
                0,
            );
            let action_id = r.pending_selection_view().unwrap().valid_action_ids[0];
            action(&mut r, 0, action_id);
            assert_eq!(r.dp_of(host), Some(7000));
            assert_eq!(
                r.memory(),
                3 + if blocked { printed } else { printed - 2 } as i16
            );
        } else {
            assert!(r.pending_selection().is_none());
            assert_eq!(r.dp_of(host), Some(4000));
        }
    }
}

#[test]
fn declining_end_of_turn_effect_does_not_suspend_king_drasil() {
    let mut r = start(builder(CARD));
    let tamer = r.place_on_field(0, CARD, Some(0));
    r.pass_turn();
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.decline_prompt().unwrap();
    let tamer = &r.game.players[0].battle_area[tamer.index as usize];
    assert!(!tamer.is_suspended);
}

#[test]
fn security_plays_tamer_without_paying_its_cost() {
    let mut r = start(builder(CARD).security(1, &[CARD]));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    r.attack_player(attacker, 1, false);
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        CARD
    );
    assert_eq!(r.memory(), 10);
}

#[test]
fn end_of_turn_option_uses_only_a_qualifying_card_under_this_tamer() {
    const OPTION: &str = "TEXT-ROYAL-OPTION";
    let mut r = start(
        builder(CARD)
            .from_dsl_yaml(
                r#"
card: TEXT-ROYAL-OPTION
name: Royal Command
kind: option
color: [white]
cost: 5
traits: [Royal Knight]
effects:
  - text: "[Main] Draw 1."
    when: main
    process:
      - draw: { of: you, count: 1 }
"#,
            )
            .unwrap()
            .hand(0, &[OPTION]),
    );
    let tamer = r.place_stack(0, &[OPTION, CARD]);
    r.pass_turn();
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.execute_branch(0).unwrap();
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(1).unwrap();
    r.assert_prompt(
        SelectionKind::SourceMulti {
            min: 0,
            max: 1,
            picked: 0,
        },
        true,
        0,
    );
    let view = r.pending_selection_view().unwrap().card_choice.unwrap();
    let picks: Vec<_> = view
        .rows
        .iter()
        .filter(|row| matches!(row.operation, CardChoiceOperation::CommitCard))
        .collect();
    assert_eq!(picks.len(), 1);
    assert_eq!(picks[0].card_id.as_deref(), Some(OPTION));
    action(&mut r, 0, picks[0].action_id);
    assert_eq!(ids(&r, &r.game.players[0].trash), [OPTION]);
    assert_eq!(ids(&r, &r.game.players[0].hand), [OPTION, "TEXT-FILLER"]);
    assert_eq!(
        r.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .len(),
        1
    );
    assert_eq!(r.memory(), 5);
}
