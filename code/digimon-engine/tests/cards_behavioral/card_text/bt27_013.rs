//! Source: `data/card_bundles/BT27-013.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.4 / cost 4
//!   - Black Lv.4 / cost 4
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Ginryumon]/Lv.4 w/[Chronicle] trait: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! [On Play] [When Digivolving] ＜De-Digivolve 1＞ 1 of your opponent's Digimon. Then, if during an attack, delete 1 of your opponent's Digimon with as much DP as this Digimon or less. [Your Turn] When your opponent's security stack is removed from, this Digimon may digivolve into a [Chronicle] trait Digimon card in the hand or trash.
//!
//! ### Inherited Effect
//! [Opponent's Turn] [Once Per Turn] When one of your opponent's Digimon attacks, you may change the attack target to 1 of your [Chronicle] trait Digimon.
//!
//! ## Official Q&A
//! - It refers to the period from when a Digimon declares an attack until the attack ends. Note that the end of an attack is also included in the period “during an attack.”

const CARD: &str = "BT27-013";

#[test]
fn inherited_redirect_can_choose_a_chronicle_digimon_or_keep_the_security_attack() {
    use digimon_engine::action::space::encode_attack;
    for redirect in [false, true] {
        let mut chronicle = digimon("CHRONICLE", 5, 7000, false);
        chronicle.traits = vec!["Chronicle".into()];
        let mut r = start(builder(CARD).add_card(chronicle));
        r.place_stack(0, &[CARD, "TEXT-6"]);
        r.place_on_field(0, "CHRONICLE", Some(0));
        let attacker = r.place_on_field(1, "TEXT-3", Some(0));
        next_turn(&mut r);
        action(&mut r, 1, encode_attack(attacker.index as u16, 14));
        assert_eq!(candidates(&r), ["CHRONICLE"]);
        if redirect {
            r.choose_prompt_card("CHRONICLE", None).unwrap();
            assert_eq!(r.battle_area_size(1), 0);
            assert_eq!(r.security_count(0), 5);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.security_count(0), 4);
        }
        assert!(r.pending_selection().is_none());
    }
}

use super::common::*;
use digimon_engine::action::space::PLAY_HAND_START;

#[test]
fn on_play_de_digivolves_but_does_not_delete_outside_an_attack() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    r.place_stack(1, &["TEXT-3", "TEXT-4"]);
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-4"]);
}

#[test]
fn opponent_security_check_offers_legal_chronicle_evolution_from_hand_or_trash() {
    use digimon_engine::action::space::encode_attack;
    use digimon_engine::owned_card_choice::CardChoiceZone;
    for pay_hand in [None, Some(true), Some(false)] {
        let mut evolution = digimon("CHRON-EVO", 6, 12000, false);
        evolution.traits = vec!["Chronicle".into()];
        let mut r = start(
            builder(CARD)
                .add_card(evolution)
                .hand(0, &["CHRON-EVO", "TEXT-6"]),
        );
        r.inject_trash(0, "CHRON-EVO");
        let attacker = r.place_on_field(0, CARD, Some(0));
        action(&mut r, 0, encode_attack(attacker.index as u16, 14));
        assert_eq!(candidates(&r), ["CHRON-EVO", "CHRON-EVO"]);
        if let Some(from_hand) = pay_hand {
            let row = r.pending_selection_view().unwrap().card_choice.unwrap().rows.into_iter()
                .find(|row| match row.zone {
                    Some(CardChoiceZone::Hand) => from_hand,
                    Some(CardChoiceZone::Trash) => !from_hand,
                    _ => false,
                }).unwrap();
            action(&mut r, 0, row.action_id);
            assert_eq!(r.memory(), 6);
            assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), [CARD, "CHRON-EVO"]);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.memory(), 10);
            assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), [CARD]);
        }
        assert_eq!(r.security_count(1), 4);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn digivolving_during_an_attack_de_digivolves_before_filtering_the_deletion() {
    use digimon_engine::action::space::encode_attack;
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-009")
            .unwrap()
            .hand(0, &[CARD]),
    );
    let attacker = r.place_on_field(0, "BT27-009", Some(0));
    r.place_stack(1, &["TEXT-3", "TEXT-4"]);
    r.place_on_field(1, "TEXT-6", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    assert_eq!(candidates(&r), [CARD]);
    r.choose_prompt_card(CARD, None).unwrap();
    assert_eq!(candidates(&r), ["TEXT-4", "TEXT-6"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-4", "TEXT-3", "TEXT-FILLER"]);
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn digivolving_at_end_of_attack_still_applies_the_attack_only_deletion() {
    use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
    use digimon_engine::selection::SelectionKind;

    let mut r = start(builder(CARD).from_dsl_yaml(r#"
card: END-EVO
name: Ginryumon
kind: digimon
level: 4
dp: 6000
color: [red]
cost: 5
effects:
  - when: end_of_attack
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: evolution
          filter: { card_number_is: BT27-013, can_digivolve_from_source: true }
          prompt: Choose the evolution at end of attack
      - effect_initiated_digivolve: { target: source, source: evolution, cost: printed }
"#).unwrap().hand(0, &[CARD, CARD]));
    let attacker = r.place_on_field(0, "END-EVO", Some(0));
    r.place_stack(1, &["TEXT-3", "TEXT-4"]);
    action(&mut r, 0, encode_attack(attacker.index.into(), 14));
    assert_eq!(r.security_count(1), 4);
    assert_eq!(candidates(&r), [CARD, CARD]);
    r.choose_prompt_card(CARD, Some(0)).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-3"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-FILLER", "TEXT-4", "TEXT-3"]);
    assert!(r.game.pending_attack.is_none());

    r.place_stack(1, &["TEXT-3", "TEXT-4"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(ids(&r, &r.game.players[1].battle_area[0].card_sources), ["TEXT-3"]);
}
