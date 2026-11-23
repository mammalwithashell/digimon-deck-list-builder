//! Source: `data/card_bundles/ST23-01.md` (printed text).
//!
//! Standard digivolve cost circles:
//! (none — DigiEgg / Lv.2 base, or DNA/special only)
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] By trashing the bottom face-down card from under any of your Tamers, this Digimon may digivolve into a [Glowing Dawn] trait Digimon card in the hand with the cost reduced by 2.

use super::common::*;
use digimon_engine::action::space::encode_attack;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "ST23-01";

#[test]
fn inherited_when_attacking_may_digivolve_glowing_dawn_with_cost_reduced_by_two_once_per_turn() {
    let mut r = start(builder(CARD).hand(0, &["TEXT-5", "TEXT-6", "TEXT-OTHER"]));
    let base = r.place_stack(0, &[CARD, "TEXT-4"]);
    stocked_tamer(&mut r, "TEXT-TAMER");
    action(&mut r, 0, encode_attack(base.index as u16, 14));
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-TAMER"]);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["TEXT-5"]);
    r.choose_prompt_card("TEXT-5", None).unwrap();
    assert_eq!(r.memory(), 8);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
    assert_eq!(
        r.game.players[0].battle_area[base.index as usize]
            .top_card()
            .card_id(&r.game.card_data),
        "TEXT-5"
    );
    assert!(r.pending_selection().is_none());
    r.game.players[0].battle_area[base.index as usize].is_suspended = false;
    action(&mut r, 0, encode_attack(base.index as u16, 14));
    assert!(r.pending_selection().is_none());
    assert_eq!(r.memory(), 8);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-BOTTOM"]);
}

#[test]
fn inherited_when_attacking_can_decline_without_paying_and_cannot_evolve_without_cost() {
    for stocked in [false, true] {
        let mut r = start(builder(CARD).hand(0, &["TEXT-5"]));
        let base = r.place_stack(0, &[CARD, "TEXT-4"]);
        if stocked {
            stocked_tamer(&mut r, "TEXT-TAMER");
        }
        action(&mut r, 0, encode_attack(base.index as u16, 14));
        if stocked {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.decline_prompt().unwrap();
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(r.memory(), 10);
        assert!(r.game.players[0].trash.is_empty());
        assert_eq!(ids(&r, &r.game.players[0].hand), ["TEXT-5"]);
    }
}
