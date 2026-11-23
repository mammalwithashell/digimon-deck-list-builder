//! Source: `data/card_bundles/BT27-063.md` (printed text).
//!
//! ### Effect
//! ＜Blocker＞ [All Turns] [Once Per Turn] When this Digimon suspends, ＜De-Digivolve 1＞ 1 of your opponent's Digimon. [All Turns] When any of your Digimon with ＜Blocker＞ would leave the battle area other than by your effects, by suspending this Digimon, 1 of your Digimon doesn't leave.
//!
//! ### Inherited Effect
//! [Opponent's Turn] [Once Per Turn] When any of your Digimon suspend, you may play 1 level 5 or lower black Digimon card with ＜Blocker＞ from your hand without paying the cost.
//!
//! ## Official Q&A
//! - Yes.

use super::common::*;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-063";

#[test]
fn only_own_suspension_de_digivolves_an_opponent() {
    let mut r = start(builder(CARD));
    let carrier = r.place_on_field(0, CARD, Some(0));
    let other = r.place_on_field(0, "TEXT-3", Some(0));
    let enemy = r.place_stack(1, &["TEXT-3", "TEXT-4"]);
    r.game.suspend(other);
    assert!(r.pending_selection().is_none());
    r.game.suspend(carrier);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    assert_eq!(candidates(&r), ["TEXT-4"]);
    r.choose_prompt_card("TEXT-4", None).unwrap();
    assert_eq!(r.dp_of(enemy), Some(2000));
}

#[test]
fn inherited_suspend_on_the_opponents_turn_can_play_a_black_blocker_for_free() {
    let mut blocker = digimon("LOW-BLOCKER", 4, 4000, false);
    blocker.effect_text = "＜Blocker＞".into();
    let mut r = start(
        builder(CARD)
            .add_card(blocker)
            .hand(0, &["LOW-BLOCKER", "TEXT-5"]),
    );
    r.place_stack(0, &[CARD, "TEXT-5"]);
    let own = r.place_on_field(0, "TEXT-3", Some(0));
    let enemy = r.place_on_field(1, "TEXT-3", Some(0));
    next_turn(&mut r);
    r.game.suspend(enemy);
    assert!(r.pending_selection().is_none());
    let memory = r.memory();
    r.game.suspend(own);
    r.assert_prompt(SelectionKind::Hand, true, 0);
    assert_eq!(candidates(&r), ["LOW-BLOCKER"]);
    r.choose_prompt_card("LOW-BLOCKER", None).unwrap();
    assert_eq!(r.battle_area_size(0), 3);
    assert_eq!(r.memory(), memory);
}

#[test]
fn suspension_payment_prevents_a_blockers_battle_deletion() {
    let mut r = start(builder(CARD).dsl_card("BT27-059").unwrap());
    let source = r.place_on_field(0, CARD, Some(0));
    let blocker = r.place_on_field(0, "BT27-059", Some(0));
    r.place_on_field(0, "TEXT-3", Some(0));
    let enemy = r.place_on_field(1, "TEXT-6", Some(0));
    r.battle_digimon(blocker, enemy);
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("TEXT-6", None).unwrap();
    assert_eq!(r.battle_area_size(0), 3);
    assert!(r.game.players[0].battle_area[source.index as usize].is_suspended);
    assert!(!r.game.players[0].battle_area[blocker.index as usize].is_suspended);
}
