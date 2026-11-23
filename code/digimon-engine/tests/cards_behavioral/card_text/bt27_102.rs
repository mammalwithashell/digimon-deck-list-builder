//! Source: `data/card_bundles/BT27-102.md` (printed text and official Q&A).
//!
//! Digivolution requirements (AUTHORITATIVE — official Bandai DB)
//! Standard digivolve cost circles
//! - (none — DigiEgg / Lv.2 base, or DNA/special only)
//!
//! Printed text (official)
//! Effect
//! [Main] You may play 1 [Kurata's Army] trait Tamer card from your hand or trash without paying the cost. Then, place this card in the battle area. [All Turns] When any Digimon on the field are deleted, ＜Delay＞. ・1 of your Tamers may digivolve into a [Kurata's Army] trait Digimon card in the hand or trash without paying the cost.
//!
//! Security Effect
//! [Security] You may play 1 play cost 5 or lower [Kurata's Army] trait card from your hand or trash without paying the cost.
//!
//! Official Q&A
//! - No. Digivolve from the Tamer as such, and do not treat it as if it is a digivolving Digimon. "When digivolving" effects and "when a Digimon digivolves" effects don't trigger. Additionally, if a "Digimon can't digivolve" effect has activated, you can still digivolve from a Tamer with this digivolve requirement.

use super::bt27_common::*;
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-102";

#[test]
fn main_can_decline_playing_a_tamer_but_still_places_the_option() {
    let mut r = start(builder(CARD).hand(0, &[CARD, TAMER, BOUNDARY]));
    r.place_on_field(0, TAMER, Some(0));
    let before = r.memory();
    action(&mut r, 0, PLAY_HAND_START);
    let kind = r.pending_selection().unwrap().kind.clone();
    assert!(matches!(kind, SelectionKind::UnionZone { .. }));
    r.assert_prompt(kind, true, 0);
    assert_eq!(candidates(&r), [TAMER]);
    r.decline_prompt().unwrap();
    assert_eq!(before - r.memory(), 3);
    assert_eq!(r.game.players[0].battle_area.len(), 2);
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        CARD
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn main_can_play_a_tamer_from_trash_without_paying_its_cost() {
    let mut r = start(builder(CARD).hand(0, &[CARD, HIGH]));
    r.place_on_field(0, TAMER, Some(0));
    r.inject_trash(0, KURATA);
    let before = r.memory();
    action(&mut r, 0, PLAY_HAND_START);
    let kind = r.pending_selection().unwrap().kind.clone();
    assert!(matches!(kind, SelectionKind::UnionZone { .. }));
    r.assert_prompt(kind, true, 0);
    assert_eq!(candidates(&r), [KURATA]);
    r.choose_prompt_card(KURATA, None).unwrap();
    assert_eq!(before - r.memory(), 3);
    assert_eq!(
        ids(&r, &r.game.players[0].battle_area[1].card_sources),
        [KURATA]
    );
    assert_eq!(
        ids(&r, &r.game.players[0].battle_area[2].card_sources),
        [CARD]
    );
    assert!(r.pending_selection().is_none());
}

#[test]
fn security_may_play_cost_five_or_less_from_hand_or_trash() {
    for choice in [Some(TAMER), Some(KURATA), None] {
        let mut r = start(builder(CARD).hand(1, &[TAMER, HIGH]).security(1, &[CARD]));
        r.inject_trash(1, KURATA);
        let attacker = r.place_on_field(0, HIGH, Some(0));
        let before = r.memory();
        action(
            &mut r,
            0,
            encode_attack(attacker.index as u16, SECURITY_TARGET),
        );
        let kind = r.pending_selection().unwrap().kind.clone();
        assert!(matches!(kind, SelectionKind::UnionZone { .. }));
        r.assert_prompt(kind, true, 1);
        assert_eq!(candidates(&r), [KURATA, TAMER]);
        if let Some(id) = choice {
            r.choose_prompt_card(id, None).unwrap();
            assert_eq!(
                ids(&r, &r.game.players[1].battle_area[0].card_sources),
                [id]
            );
        } else {
            r.decline_prompt().unwrap();
            assert!(r.game.players[1].battle_area.is_empty());
        }
        assert_eq!(r.memory(), before);
        assert!(r.pending_selection().is_none());
    }
}

#[test]
fn delay_may_digivolve_a_tamer_for_free_after_a_later_battle_deletion() {
    let mut r = start(
        builder(CARD)
            .dsl_card("BT27-026")
            .unwrap()
            .hand(0, &[CARD, "BT27-026"]),
    );
    let tamer = r.place_on_field(0, TAMER, Some(0));
    action(&mut r, 0, PLAY_HAND_START);
    let victim = r.place_on_field(1, BOUNDARY, Some(0));
    let attacker = r.place_on_field(0, HIGH, Some(0));
    next_turn(&mut r);
    next_turn(&mut r);
    r.game.players[1].battle_area[victim.index as usize].is_suspended = true;
    let before = r.memory();
    action(
        &mut r,
        0,
        encode_attack(attacker.index as u16, victim.index as u16),
    );
    r.accept_optional_trigger().unwrap();
    r.choose_prompt_card(TAMER, None).unwrap();
    r.choose_prompt_card("BT27-026", None).unwrap();
    assert_eq!(r.memory(), before);
    assert_eq!(
        ids(
            &r,
            &r.game.players[0].battle_area[tamer.index as usize].card_sources
        ),
        [TAMER, "BT27-026"]
    );
    assert_eq!(ids(&r, &r.game.players[0].trash), [CARD]);
    assert!(r.pending_selection().is_none());
}
