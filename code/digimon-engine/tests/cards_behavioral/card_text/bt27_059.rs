//! Source: `data/card_bundles/BT27-059.md` (printed text).
//!
//! ### Effect
//! ＜Blocker＞ [On Play] [When Digivolving] Your opponent's ＜De-Digivolve＞ effects don't affect 1 of your Digimon until their turn ends. Then, if it's their turn, ＜De-Digivolve 1＞ 1 of their Digimon.
//!
//! ### Inherited Effect
//! [Opponent's Turn] [Once Per Turn] When any of your Digimon suspend, this Digimon may unsuspend.

use super::common::*;
use digimon_engine::enums::ModifierType;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-059";

#[test]
fn on_own_turn_clockmon_protects_a_chosen_digimon_without_de_digivolving_an_enemy() {
    let mut r = start(builder(CARD).hand(0, &[CARD]));
    let friendly = r.place_stack(0, &["TEXT-3", "TEXT-4"]);
    let enemy = r.place_stack(1, &["TEXT-3", "TEXT-5"]);
    r.play(0, 0);
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), [CARD, "TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert!(r
        .game
        .modifiers
        .has(friendly, ModifierType::CannotBeDeDigivolved));
    assert_eq!(r.dp_of(enemy), Some(7000));
    assert!(r.pending_selection().is_none());
}

#[test]
fn inherited_unsuspend_is_optional_on_the_opponents_turn_and_capped() {
    let mut r = start(builder(CARD));
    let carrier = r.place_stack(0, &[CARD, "TEXT-5"]);
    let first = r.place_on_field(0, "TEXT-3", Some(0));
    let second = r.place_on_field(0, "TEXT-4", Some(0));
    r.game.suspend(carrier);
    assert!(r.pending_selection().is_none());
    next_turn(&mut r);
    assert_eq!(r.turn_player(), 1);
    r.game.suspend(first);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    assert!(!r.game.players[0].battle_area[carrier.index as usize].is_suspended);
    r.game.suspend(carrier);
    r.game.suspend(second);
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0].battle_area[carrier.index as usize].is_suspended);
}
