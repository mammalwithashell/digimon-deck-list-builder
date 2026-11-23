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
