//! BT27-022: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Blue Lv.2 / cost 0
//!
//! ## Printed text (official)
//! ### Effect
//! [On Play] Reveal the top 3 cards of your deck. Add 1 [Ice-Snow] or [ALYSION] trait card and 1 blue Tamer card among them to the hand. Return the rest to the bottom of the deck. (Rule) Trait: Has [Ice-Snow] Type.
//!
//! ### Inherited Effect
//! [When Attacking] If this Digimon has the [Ice-Snow] trait, you may change the attack target to 1 of your opponent's unsuspended Digimon with the fewest digivolution cards.
//!
//! ## Official Q&A
//! - You choose 1 Digimon from among them.

use super::bt27_gemmon_fixtures::fixture;
use super::common::{action, candidates, ids, start};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

#[test]
fn renamon_adds_ice_snow_and_blue_tamer() {
    let mut r = start(
        fixture(&["BT27-022"])
            .hand(0, &["BT27-022"])
            .deck(0, &["BLUE-TAMER", "RED-TAMER", "ICE"]),
    );
    action(&mut r, 0, PLAY_HAND_START);
    for (bucket_index, card) in ["ICE", "BLUE-TAMER"].into_iter().enumerate() {
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
    assert_eq!(ids(&r, &r.game.players[0].hand), ["ICE", "BLUE-TAMER"]);
    r.assert_prompt(SelectionKind::OrderedPermutation { remaining: 1 }, false, 0);
    let remainder = r.pending_selection().unwrap().valid_action_ids[0];
    action(&mut r, 0, remainder);
    assert_eq!(r.deck_size(0), 1);
}

#[test]
fn inherited_renamon_can_redirect_to_the_fewest_unsuspended_sources() {
    let mut r = start(fixture(&["BT27-022"]));
    let attacker = r.place_stack(0, &["BT27-022", "ICE"]);
    r.place_stack(1, &["FILLER", "ALYSION"]);
    r.place_stack(1, &["FILLER"]);
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::OppField, true, 0);
    assert_eq!(candidates(&r), ["FILLER"]);
    r.choose_prompt_card("FILLER", None).unwrap();
    assert_eq!(r.security_count(1), 3);
    assert_eq!(r.game.players[1].battle_area.len(), 1);
}

#[test]
fn renamon_redirect_can_be_declined_or_have_no_eligible_target() {
    for eligible in [false, true] {
        let mut r = start(fixture(&["BT27-022"]));
        let attacker = r.place_stack(0, &["BT27-022", "ICE"]);
        let defender = r.place_stack(1, &["FILLER"]);
        r.game.players[1].battle_area[defender.index as usize].is_suspended = !eligible;
        action(
            &mut r,
            0,
            encode_attack(attacker.index as u16, SECURITY_TARGET),
        );
        if eligible {
            r.assert_prompt(SelectionKind::OppField, true, 0);
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.security_count(1), 2);
    }
}
