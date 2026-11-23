//! Source: `data/card_bundles/ST23-13.md` (printed text and official Q&A).
//!
//! Standard digivolve cost circles:
//! (none — DigiEgg / Lv.2 base, or DNA/special only)
//! Effect:
//! [Start of Your Main Phase] [On Play] You may place the top card of your deck face down under this Tamer. Then, if your opponent has a Digimon, gain 1 memory. [Your Turn] When effects trash cards from under this Tamer, by suspending this Tamer, 1 of your [Glowing Dawn] trait Digimon gets +3000 DP until your opponent's turn ends.
//! Security Effect:
//! [Security] Play this card without paying the cost.
//! Official Q&A:
//! The card is placed on the bottom of the cards under the Tamer.

use super::common::*;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "ST23-13";

#[test]
fn on_play_may_place_deck_top_face_down_then_gains_memory_only_with_opponent_digimon() {
    for accept in [false, true] {
        for opponent in [None, Some("TEXT-3"), Some("TEXT-TAMER")] {
            let mut r = start(builder(CARD).hand(0, &[CARD]));
            if let Some(id) = opponent {
                r.place_on_field(1, id, Some(0));
            }
            let top = r.game.players[0].deck.last().unwrap().handle();
            action(&mut r, 0, PLAY_HAND_START);
            r.assert_prompt(SelectionKind::EffectChoice, false, 0);
            r.execute_branch(if accept { 0 } else { 1 }).unwrap();
            assert!(r.pending_selection().is_none());
            assert_eq!(r.memory(), if opponent == Some("TEXT-3") { 7 } else { 6 });
            let sources = &r.game.players[0].battle_area[0].card_sources;
            assert_eq!(sources.len(), if accept { 2 } else { 1 });
            if accept {
                assert_eq!(sources[0].handle(), top);
                assert!(sources[0].face_down);
            }
        }
    }
}

#[test]
fn start_of_your_main_phase_places_new_card_at_bottom_even_with_existing_sources() {
    let mut r = start(builder(CARD));
    let tamer = stocked_tamer(&mut r, CARD);
    r.place_on_field(1, "TEXT-3", Some(0));
    next_turn(&mut r);
    next_turn(&mut r);
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(0).unwrap();
    let sources = &r.game.players[0].battle_area[tamer.index as usize].card_sources;
    assert_eq!(
        ids(&r, sources),
        ["TEXT-FILLER", "TEXT-BOTTOM", "TEXT-TOP", CARD]
    );
    assert!(sources[0].face_down);
    assert_eq!(r.memory(), 4);
}

#[test]
fn your_turn_effect_trashing_under_this_tamer_may_suspend_it_to_buff_only_glowing_dawn() {
    for accept in [false, true] {
        let mut r = start(
            builder(CARD)
                .dsl_card("ST23-12")
                .unwrap()
                .hand(0, &["ST23-12"]),
        );
        let t = stocked_tamer(&mut r, CARD);
        let target = r.place_on_field(0, "TEXT-4", Some(0));
        r.place_on_field(0, "TEXT-OTHER", Some(0));
        action(&mut r, 0, PLAY_HAND_START);
        r.assert_prompt(SelectionKind::Replacement, true, 0);
        r.accept_optional_trigger().unwrap();
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        r.choose_prompt_card(CARD, None).unwrap();
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        if accept {
            r.execute_branch(0).unwrap();
            r.assert_prompt(SelectionKind::OwnField, false, 0);
            assert_eq!(candidates(&r), ["ST23-12", "TEXT-4"]);
            r.choose_prompt_card("TEXT-4", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(
            r.game.players[0].battle_area[t.index as usize].is_suspended,
            accept
        );
        assert_eq!(r.dp_of(target), Some(if accept { 7000 } else { 4000 }));
        next_turn(&mut r);
        assert_eq!(r.dp_of(target), Some(if accept { 7000 } else { 4000 }));
        next_turn(&mut r);
        r.assert_prompt(SelectionKind::EffectChoice, false, 0);
        r.execute_branch(1).unwrap();
        assert_eq!(r.dp_of(target), Some(4000));
    }
}

#[test]
fn security_plays_tamer_without_paying_cost() {
    let mut r = start(builder(CARD).security(1, &[CARD]));
    let attacker = r.place_on_field(0, "TEXT-6", Some(0));
    action(&mut r, 0, encode_attack(attacker.index as u16, 14));
    r.assert_prompt(SelectionKind::EffectChoice, false, 1);
    r.execute_branch(1).unwrap();
    assert_eq!(
        r.game.players[1].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        CARD
    );
    assert_eq!(r.memory(), 9);
    assert!(!ids(&r, &r.game.players[1].trash).contains(&CARD.into()));
}
