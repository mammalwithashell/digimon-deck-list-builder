//! Source: `data/card_bundles/BT27-098.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! [All Turns] When any Digimon are deleted, by suspending this Tamer, you may place the top 2 cards of your deck face down under this Tamer. [End of All Turns] By trashing 5 bottom face-down cards from under any of your Tamers, you may play 1 Digimon card with [Gizmon] or [Belphemon] in its name from your hand or trash without paying the cost. After, place this Tamer as its bottom digivolution card.
//!
//! Security Effect
//! [Security] Play this card without paying the cost.
//!
//! Official Q&A
//! - The card is placed on the bottom of the cards under the Tamer.

use super::bt27_common::*;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::make_test_card;
use digimon_engine::selection::SelectionKind;

#[test]
fn security_plays_tamer_for_free() {
    security_plays_self("BT27-098");
}

#[test]
fn end_of_turn_pays_five_face_down_cards_then_places_kurata_under_the_played_digimon() {
    const CARD: &str = "BT27-098";
    const GIZMON: &str = "TEXT-KURATA-GIZMON";
    let mut r = start(
        builder(CARD)
            .add_card(make_test_card(GIZMON, "Gizmon: AT"))
            .hand(0, &[GIZMON, HIGH]),
    );
    let tamer = r.place_stack(0, &[BOUNDARY, BOUNDARY, BOUNDARY, BOUNDARY, BOUNDARY, CARD]);
    for card in &mut r.game.players[0].battle_area[tamer.index as usize].card_sources[..5] {
        card.face_down = true;
    }
    action(&mut r, 0, PASS);
    r.accept_optional_trigger().unwrap();
    for _ in 0..5 {
        r.assert_prompt(SelectionKind::OwnField, false, 0);
        assert_eq!(candidates(&r), [CARD]);
        r.choose_prompt_card(CARD, None).unwrap();
    }
    let kind = r.pending_selection().unwrap().kind.clone();
    assert!(matches!(kind, SelectionKind::UnionZone { .. }));
    r.assert_prompt(kind, true, 0);
    assert_eq!(candidates(&r), [GIZMON]);
    r.choose_prompt_card(GIZMON, None).unwrap();
    let field = &r.game.players[0].battle_area;
    assert_eq!(field.len(), 1);
    let stack: Vec<_> = field[0]
        .card_sources
        .iter()
        .map(|card| card.card_id(&r.game.card_data))
        .collect();
    assert_eq!(stack, [CARD, GIZMON]);
    assert_eq!(r.trash_size(0), 5);
    assert!(r.pending_selection().is_none());
}

#[test]
fn end_of_turn_is_optional_and_requires_five_face_down_cards() {
    for count in [4, 5] {
        let mut r = start(builder("BT27-098").hand(0, &[GIZMON]));
        let tamer = r.place_stack(
            0,
            &[BOUNDARY, BOUNDARY, BOUNDARY, BOUNDARY, HIGH, "BT27-098"],
        );
        for card in &mut r.game.players[0].battle_area[tamer.index as usize].card_sources[..count] {
            card.face_down = true;
        }
        action(&mut r, 0, PASS);
        if count == 5 {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.decline_optional_trigger().unwrap();
        }
        assert_eq!(r.trash_size(0), 0);
        assert_eq!(
            r.game.players[0].battle_area[tamer.index as usize]
                .card_sources
                .len(),
            6
        );
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn deletion_can_suspend_kurata_and_put_the_two_deck_cards_face_down_at_the_bottom() {
    let mut r = start(builder("BT27-098"));
    let tamer = r.place_stack(0, &[BOUNDARY, "BT27-098"]);
    let deck = &r.game.players[0].deck;
    let mut expected = deck[deck.len() - 2..]
        .iter()
        .map(|c| c.handle())
        .collect::<Vec<_>>();
    delete_in_battle(&mut r, GIZMON);
    r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
    r.execute_branch(0).unwrap();
    let tamer = &r.game.players[0].battle_area[tamer.index as usize];
    assert!(tamer.is_suspended);
    assert_eq!(tamer.card_sources.len(), 4);
    assert!(tamer.card_sources[..2].iter().all(|c| c.face_down));
    for c in &tamer.card_sources[..2] {
        let pos = expected.iter().position(|&h| h == c.handle()).unwrap();
        expected.remove(pos);
    }
    assert_eq!(tamer.card_sources[2].card_id(&r.game.card_data), BOUNDARY);
    assert!(r.pending_selection().is_none());
}

#[test]
fn deletion_placement_can_be_declined_and_cannot_pay_with_a_suspended_kurata() {
    for already_suspended in [false, true] {
        let mut r = start(builder("BT27-098"));
        let tamer = r.place_on_field(0, "BT27-098", Some(0));
        r.game.players[0].battle_area[tamer.index as usize].is_suspended = already_suspended;
        let deck_count = r.game.players[0].deck.len();
        delete_in_battle(&mut r, GIZMON);
        if !already_suspended {
            r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
            r.decline_optional_trigger().unwrap();
        }
        assert!(r.pending_selection().is_none());
        let tamer = &r.game.players[0].battle_area[tamer.index as usize];
        assert_eq!(tamer.is_suspended, already_suspended);
        assert_eq!(tamer.card_sources.len(), 1);
        assert_eq!(r.game.players[0].deck.len(), deck_count);
    }
}

#[test]
fn deletion_of_either_players_digimon_token_can_suspend_kurata_and_stash_two_cards() {
    const CARD: &str = "BT27-098";
    const TOKEN: &str = "TEXT-KURATA-TOKEN";
    for token_owner in [0, 1] {
        let mut token = make_test_card(TOKEN, "Digimon Token");
        token.card_kind = digimon_engine::enums::CardKind::Token;
        let mut r = start(builder(CARD).add_card(token));
        let kurata = r.place_on_field(0, CARD, Some(0));
        let victim = r.place_on_field(token_owner, TOKEN, Some(0));
        let deck_count = r.game.players[0].deck.len();

        r.game.delete_permanent_with_effects(victim);
        r.assert_prompt(SelectionKind::TriggerOrder, true, 0);
        r.execute_branch(0).unwrap();

        let kurata = &r.game.players[0].battle_area[kurata.index as usize];
        assert!(kurata.is_suspended);
        assert_eq!(kurata.card_sources.len(), 3);
        assert!(kurata.card_sources[..2].iter().all(|card| card.face_down));
        assert_eq!(r.game.players[0].deck.len(), deck_count - 2);
        assert!(r.pending_selection().is_none());
    }
}
