//! Source: `data/card_bundles/BT27-045.md` (printed text).
//!
//! ### Effect
//! [On Play] Reveal the top 3 cards of your deck. Add 1 card with [Leomon] in its name and 1 [Unique], [Boss] or [ACCEL] trait card among them to the hand. Return the rest to the bottom of the deck. (Rule) Name: Treated as including [Leomon].
//!
//! ### Inherited Effect
//! [Your Turn] [Once Per Turn] When this Digimon wins a battle, gain 1 memory.
//!
//! ## Official Q&A
//! - It triggers when the battle is won. When the Digimon with this effect wins a battle, the opponent's Digimon that lost the battle is deleted, then the "when this Digimon wins a battle" effect can be activated.

use super::common::*;
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-045";

#[test]
fn reveal_adds_separate_leomon_name_and_matching_trait_cards() {
    let mut accel = make_test_card("ACCEL", "Fixture");
    accel.traits.push("ACCEL".into());
    let mut r = start(
        builder(CARD)
            .add_card(accel)
            .add_card(make_test_card("LEO", "Leomon"))
            .hand(0, &[CARD])
            .deck(0, &["TEXT-FILLER", "ACCEL", "LEO"]),
    );
    r.play(0, 0);
    assert_eq!(candidates(&r), ["LEO"]);
    r.choose_prompt_card("LEO", None).unwrap();
    assert_eq!(candidates(&r), ["ACCEL"]);
    r.choose_prompt_card("ACCEL", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].hand), ["LEO", "ACCEL"]);
    r.assert_prompt(SelectionKind::OrderedPermutation { remaining: 1 }, false, 0);
    assert_eq!(ids(&r, &r.game.revealed_cards), ["TEXT-FILLER"]);
    action(&mut r, 0, SEL_REVEAL_START);
    assert_eq!(ids(&r, &r.game.players[0].deck), ["TEXT-FILLER"]);
}

#[test]
fn overlapping_reveal_card_can_fill_the_first_matching_pick() {
    for name_pick in ["BOTH", "LEO"] {
        let mut both = make_test_card("BOTH", "ACCEL Leomon");
        both.traits.push("ACCEL".into());
        let mut r = start(
            builder(CARD)
                .add_card(both)
                .add_card(make_test_card("LEO", "Leomon"))
                .hand(0, &[CARD])
                .deck(0, &["TEXT-FILLER", "LEO", "BOTH"]),
        );
        r.play(0, 0);
        r.assert_prompt(
            SelectionKind::RevealBucket {
                bucket_index: 0,
                min: 1,
                max: 1,
                picked: 0,
            },
            false,
            0,
        );
        assert_eq!(candidates(&r), ["BOTH", "LEO"]);
        r.choose_prompt_card(name_pick, None).unwrap();
        let expected_hand = if name_pick == "LEO" {
            assert_eq!(candidates(&r), ["BOTH"]);
            r.choose_prompt_card("BOTH", None).unwrap();
            vec!["LEO", "BOTH"]
        } else {
            vec!["BOTH"]
        };
        assert_eq!(ids(&r, &r.game.players[0].hand), expected_hand);
        let remaining = (3 - expected_hand.len()) as u8;
        for remaining in (1..=remaining).rev() {
            r.assert_prompt(SelectionKind::OrderedPermutation { remaining }, false, 0);
            action(&mut r, 0, SEL_REVEAL_START);
        }
        let mut bottom = ids(&r, &r.game.players[0].deck);
        bottom.sort();
        assert_eq!(
            bottom,
            if name_pick == "LEO" {
                vec!["TEXT-FILLER"]
            } else {
                vec!["LEO", "TEXT-FILLER"]
            }
        );
    }
}

#[test]
fn inherited_battle_win_gains_memory_once_per_turn() {
    let mut r = start(builder(CARD).memory(5));
    let carrier = r.place_stack(0, &[CARD, "TEXT-5"]);
    for expected in [6, 6] {
        let enemy = r.place_on_field(1, "TEXT-3", Some(0));
        r.battle_digimon(carrier, enemy);
        assert_eq!(r.battle_area_size(1), 0);
        assert_eq!(r.memory(), expected);
    }
}
