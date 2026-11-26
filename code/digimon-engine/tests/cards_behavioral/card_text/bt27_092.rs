//! Source: `data/card_bundles/BT27-092.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! [Security] Play this card without paying the cost. [Start of Your Main Phase] By placing 1 [Kurata's Army] trait card from your hand face down under this Tamer, ＜Draw 1＞ and gain 1 memory. [Main] [Once Per Turn] By trashing 2 bottom face-down cards from under any of your Tamers, this Tamer may digivolve into a [Kurata's Army] trait Digimon card in the hand or trash with the cost reduced by 2.
//!
//! Inherited Effect
//! [End of Attack] [Once Per Turn] By choosing 1 of your Digimon, delete it and 1 of your opponent's Digimon with as much DP as this Digimon or less.
//!
//! Official Q&A
//! - The card is placed on the bottom of the cards under the Tamer.

use super::bt27_common::*;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-092";

#[test]
fn security_plays_tamer_for_free() {
    security_plays_self("BT27-092");
}

#[test]
fn start_of_main_stashes_at_the_bottom_then_draws_and_gains_memory() {
    for accept in [false, true] {
        let mut r = builder(CARD).hand(0, &[TAMER, HIGH]).memory(5).start();
        let host = r.place_stack(0, &[BOUNDARY, CARD]);
        let memory = r.memory();
        let deck_size = r.game.players[0].deck.len();
        r.game.enter_main_phase();
        r.assert_prompt(SelectionKind::Hand, true, 0);
        assert_eq!(candidates(&r), [TAMER]);
        if accept {
            r.choose_prompt_card(TAMER, None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        let sources = &r.game.players[0].battle_area[host.index as usize].card_sources;
        if accept {
            assert!(sources[0].face_down);
            assert_eq!(ids(&r, sources), [TAMER, BOUNDARY, CARD]);
        } else {
            assert_eq!(ids(&r, sources), [BOUNDARY, CARD]);
        }
        assert_eq!(
            r.game.players[0].deck.len(),
            deck_size - usize::from(accept)
        );
        assert_eq!(r.memory(), memory + i16::from(accept));
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn inherited_end_of_attack_can_delete_a_chosen_ally_and_dp_bounded_enemy() {
    for accept in [false, true] {
        let mut r = start(builder(CARD));
        let host = r.place_stack(0, &[CARD, BOUNDARY]);
        r.place_on_field(0, EVOLUTION, Some(0));
        r.place_on_field(1, HIGH, Some(0));
        r.place_on_field(1, BOUNDARY, Some(0));
        action(&mut r, 0, encode_attack(host.index as u16, SECURITY_TARGET));
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), [BOUNDARY, EVOLUTION]);
        if accept {
            r.choose_prompt_card(EVOLUTION, None).unwrap();
            r.assert_prompt(SelectionKind::OppField, false, 0);
            assert_eq!(candidates(&r), [BOUNDARY]);
            r.choose_prompt_card(BOUNDARY, None).unwrap();
            assert_eq!(ids(&r, &r.game.players[0].trash), [EVOLUTION]);
        } else {
            action(&mut r, 0, PASS);
            assert_eq!(r.trash_size(0), 0);
        }
        let remaining: Vec<_> = r.game.players[1]
            .battle_area
            .iter()
            .map(|permanent| permanent.top_card().card_id(&r.game.card_data))
            .collect();
        assert_eq!(
            remaining,
            if accept {
                &[HIGH][..]
            } else {
                &[HIGH, BOUNDARY]
            }
        );
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn main_may_pay_two_bottom_sources_to_digivolve_the_tamer() {
    use digimon_engine::action::space::{FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START};
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-026")
            .unwrap()
            .hand(0, &["BT27-026"]),
    );
    let host = r.place_stack(0, &[HIGH, BOUNDARY, CARD]);
    for source in &mut r.game.players[0].battle_area[0].card_sources[..2] {
        source.face_down = true;
    }
    let before = r.memory();
    action(&mut r, 0, FIELD_EFFECT_START + FIELD_EFFECT_SLOT_FOR_MAIN);
    for _ in 0..2 {
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        r.choose_prompt_card(CARD, None).unwrap();
    }
    r.choose_prompt_card("BT27-026", None).unwrap();
    // BT27-026's optional [When Digivolving] triggers on Tamer digivolution.
    if r.pending_selection().is_some() {
        r.decline_prompt().unwrap();
    }
    assert_eq!(before, r.memory());
    assert_eq!(
        ids(
            &r,
            &r.game.players[0].battle_area[host.index as usize].card_sources
        ),
        [CARD, "BT27-026"]
    );
    assert_eq!(ids(&r, &r.game.players[0].trash), [HIGH, BOUNDARY]);
    assert!(r.pending_selection().is_none());
}
