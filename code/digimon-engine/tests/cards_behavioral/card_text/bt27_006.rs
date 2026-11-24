//! BT27-006: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.2 / cost 0
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.2 w/[Gemmon] in text: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with the [ALYSION] trait and 1 red Tamer card among them to the hand. Return the rest to the bottom of the deck.
//!
//! ### Inherited Effect
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

use super::bt27_gemmon_fixtures::{fixture, gemmon_text_digivolution};
use super::common::{action, candidates, ids, start};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::selection::SelectionKind;

#[test]
fn gemmon_alternate_digivolution_finds_gemmon_in_requirements() {
    gemmon_text_digivolution("BT27-006", 2, 0);
}

#[test]
fn gemmon_adds_alysion_and_red_tamer() {
    let mut r = start(
        fixture(&["BT27-006"])
            .hand(0, &["BT27-006"])
            .deck(0, &["RED-TAMER", "BLUE-TAMER", "ALYSION"]),
    );
    action(&mut r, 0, PLAY_HAND_START);
    for (bucket_index, card) in ["ALYSION", "RED-TAMER"].into_iter().enumerate() {
        r.assert_prompt(
            SelectionKind::RevealBucket {
                bucket_index: bucket_index as u8,
                min: 1,
                max: 1,
                picked: 0,
            },
            false,
            0,
        );
        assert_eq!(candidates(&r), [card]);
        r.choose_prompt_card(card, None).unwrap();
    }
    assert_eq!(ids(&r, &r.game.players[0].hand), ["ALYSION", "RED-TAMER"]);
    r.assert_prompt(SelectionKind::OrderedPermutation { remaining: 1 }, false, 0);
    let remainder = r.pending_selection().unwrap().valid_action_ids[0];
    action(&mut r, 0, remainder);
    assert_eq!(r.deck_size(0), 1);
}

#[test]
fn gemmon_inherited_dp_applies_only_on_your_turn() {
    let mut r = start(fixture(&["BT27-006"]));
    let gemmon = r.place_stack(0, &["BT27-006", "FILLER"]);
    assert_eq!(r.effective_dp(gemmon), Some(4000));
    action(&mut r, 0, PASS);
    assert_eq!(r.effective_dp(gemmon), Some(2000));
}

#[test]
fn gemmon_search_can_take_a_tamer_without_an_alysion_match() {
    let mut r = start(
        fixture(&["BT27-006"])
            .hand(0, &["BT27-006"])
            .deck(0, &["RED-TAMER", "FILLER", "ICE"]),
    );
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["RED-TAMER"]);
    r.choose_prompt_card("RED-TAMER", None).unwrap();
    for (card, remaining) in [("FILLER", 2), ("ICE", 1)] {
        r.assert_prompt(SelectionKind::OrderedPermutation { remaining }, false, 0);
        r.choose_prompt_card(card, None).unwrap();
    }
    assert_eq!(ids(&r, &r.game.players[0].hand), ["RED-TAMER"]);
}
