//! Source: `data/card_bundles/BT27-021.md` (printed conditions, effects, and official Q&A).
//!
//! ### Standard digivolve cost circles
//!   - Blue Lv.2 / cost 1
//!   - Red Lv.2 / cost 1
//!   - Green Lv.2 / cost 1
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] [Dracomon]: Cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [On Play] [When Digivolving] Reveal the top 3 cards of your deck. Add 1 Digimon card with [Examon] in its text and 1 such Option card among them to the hand. Return the rest to the bottom of the deck.
//!
//! ### Inherited Effect
//! [End of Your Turn] This Digimon and any of your other Digimon may DNA digivolve into a Digimon card in the hand.
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

const CARD: &str = "BT27-021";

use super::common::*;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::selection::SelectionKind;

#[test]
fn on_play_returns_revealed_cards_to_bottom_when_neither_search_category_matches() {
    let mut r = start(
        builder(CARD)
            .deck(0, &["TEXT-FILLER", "TEXT-3", "TEXT-4", "TEXT-5"])
            .hand(0, &[CARD]),
    );
    let before = r.game.players[0].deck.len();
    action(&mut r, 0, PLAY_HAND_START);
    for (remaining, expected) in [
        (3, vec!["TEXT-3", "TEXT-4", "TEXT-5"]),
        (2, vec!["TEXT-3", "TEXT-4"]),
        (1, vec!["TEXT-3"]),
    ] {
        r.assert_prompt(SelectionKind::OrderedPermutation { remaining }, false, 0);
        assert_eq!(candidates(&r), expected);
        r.choose_prompt_card(expected.last().unwrap(), None)
            .unwrap();
    }
    assert!(r.pending_selection().is_none());
    assert_eq!(r.game.players[0].deck.len(), before);
    assert_eq!(
        r.game.players[0]
            .deck
            .last()
            .unwrap()
            .card_id(&r.game.card_data),
        "TEXT-FILLER"
    );
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.battle_area_size(0), 1);
}

#[test]
fn on_play_adds_one_examon_text_digimon_and_one_examon_text_option() {
    use digimon_engine::enums::CardKind;
    let mut digimon_pick = digimon("EXA-DIGIMON", 3, 2000, false);
    digimon_pick.effect_text = "Digivolve into [Examon].".into();
    let mut option_pick = digimon("EXA-OPTION", 3, 2000, false);
    option_pick.card_kind = CardKind::Option;
    option_pick.effect_text = "Choose an [Examon].".into();
    let mut r = start(
        builder(CARD)
            .add_card(digimon_pick)
            .add_card(option_pick)
            .deck(0, &["TEXT-FILLER", "EXA-DIGIMON", "EXA-OPTION", "TEXT-3"])
            .hand(0, &[CARD]),
    );
    action(&mut r, 0, PLAY_HAND_START);
    assert_eq!(candidates(&r), ["EXA-DIGIMON"]);
    r.choose_prompt_card("EXA-DIGIMON", None).unwrap();
    assert_eq!(candidates(&r), ["EXA-OPTION"]);
    r.choose_prompt_card("EXA-OPTION", None).unwrap();
    if r.pending_selection().is_some() {
        assert_eq!(candidates(&r), ["TEXT-3"]);
        r.choose_prompt_card("TEXT-3", None).unwrap();
    }
    assert_eq!(ids(&r, &r.game.players[0].hand), ["EXA-DIGIMON", "EXA-OPTION"]);
    assert_eq!(ids(&r, &r.game.players[0].deck), ["TEXT-3", "TEXT-FILLER"]);
}

#[test]
fn inherited_end_of_turn_dna_offers_only_matching_partners_and_results() {
    use digimon_engine::action::space::PASS;
    use digimon_engine::debug_runner::make_test_dna_card;
    for accept in [false, true] {
        let result = make_test_dna_card("RESULT", "DNA Result", 5, 4, 0);
        let mismatch = make_test_dna_card("WRONG", "Unmatched DNA Result", 6, 4, 0);
        let mut r = start(builder(CARD).add_card(result).add_card(mismatch).hand(0, &["RESULT", "WRONG"]));
        r.place_stack(0, &[CARD, "TEXT-5"]);
        r.place_on_field(0, "TEXT-4", Some(0));
        r.place_on_field(0, "TEXT-3", Some(0));
        action(&mut r, 0, PASS);
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-4"]);
        if accept {
            r.choose_prompt_card("TEXT-4", None).unwrap();
            assert_eq!(candidates(&r), ["RESULT"]);
            r.choose_prompt_card("RESULT", None).unwrap();
            assert_eq!(ids(&r, &r.game.players[0].battle_area[0].card_sources), ["TEXT-4", CARD, "TEXT-5", "RESULT"]);
            assert_eq!(r.battle_area_size(0), 2);
        } else {
            r.decline_prompt().unwrap();
            assert_eq!(r.battle_area_size(0), 3);
        }
        assert!(r.pending_selection().is_none());
    }
}
