//! Source: `data/card_bundles/BT27-054.md` (printed text).
//!
//! ### Effect
//! When this card would be played, if there are 2 or more suspended Digimon, reduce the cost by 4. [On Play] [When Digivolving] You may suspend 1 of your opponent's Digimon or Tamers. Then, 1 of their Digimon or Tamers can't unsuspend until their turn ends. [All Turns] [Once Per Turn] When this Digimon would leave the battle area, by deleting 1 other suspended Digimon, it doesn't leave.
//!
//! ## Official Q&A
//! - Yes, it can.

use super::common::*;
use digimon_engine::enums::ModifierType;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-054";

#[test]
fn suspended_digimon_reduce_play_cost_and_tamer_can_be_frozen() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let first = r.place_on_field(0, "TEXT-3", Some(0));
    let second = r.place_on_field(1, "TEXT-3", Some(0));
    let tamer = r.place_on_field(1, "TEXT-TAMER", Some(0));
    r.game.suspend(first);
    r.game.suspend(second);
    r.play(0, 0);
    assert_eq!(r.memory(), 3);
    r.assert_prompt(SelectionKind::OppField, true, 0);
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("TEXT-TAMER", None).unwrap();
    assert!(r.game.modifiers.has(tamer, ModifierType::CannotUnsuspend));
}

#[test]
fn can_delete_another_suspended_digimon_instead_of_leaving_once_per_turn() {
    let mut r = start(builder(CARD));
    let host = r.place_on_field(0, CARD, Some(0));
    let enemy = r.place_on_field(1, "TEXT-6", Some(0));
    let first = r.place_on_field(0, "TEXT-3", Some(0));
    let second = r.place_on_field(1, "TEXT-4", Some(0));
    r.game.suspend(first);
    r.game.suspend(second);
    r.battle_digimon(host, enemy);
    assert!(r.pending_selection().unwrap().is_optional);
    if r.pending_selection().unwrap().kind == SelectionKind::Replacement {
        r.accept_optional_trigger().unwrap();
    }
    assert_eq!(candidates(&r), ["TEXT-3", "TEXT-4"]);
    r.choose_prompt_card("TEXT-3", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-3"]);
    r.battle_digimon(host, enemy);
    assert!(r.pending_selection().is_none());
    assert_eq!(r.battle_area_size(0), 0);
}
