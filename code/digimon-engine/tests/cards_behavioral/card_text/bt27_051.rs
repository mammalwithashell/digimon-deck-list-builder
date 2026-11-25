//! Source: `data/card_bundles/BT27-051.md` (printed text).
//!
//! ### Effect
//! ＜Fortitude＞ [On Play] [When Digivolving] This Digimon may battle 1 of your opponent's Digimon. [On Deletion] ＜Recovery +1＞ (Rule) Trait: Has [Holy Beast] Type.
//!
//! ### Inherited Effect
//! [On Deletion] ＜Recovery +1＞
//!
//! ## Official Q&A
//! - This effect immediately initiates a battle. If this effect is activated, the chosen Digimon battle according to the standard rules.

use super::common::*;
use digimon_engine::selection::SelectionKind;

const CARD: &str = "BT27-051";

#[test]
fn on_play_may_start_a_battle_and_recover_on_deletion() {
    for accept in [false, true] {
        let mut r = start(builder(CARD).hand(0, &[CARD]));
        r.place_on_field(1, "TEXT-6", Some(0));
        r.play(0, 0);
        r.assert_prompt(SelectionKind::OppField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-6"]);
        if accept {
            r.choose_prompt_card("TEXT-6", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.security_count(0), if accept { 6 } else { 5 });
        assert_eq!(r.trash_size(0), usize::from(accept));
    }
}

#[test]
fn inherited_recovery_puts_the_top_deck_card_into_security() {
    let mut r = start(builder(CARD));
    let carrier = r.place_stack(0, &[CARD, "TEXT-5"]);
    let enemy = r.place_on_field(1, "TEXT-6", Some(0));
    r.battle_digimon(carrier, enemy);
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.security_count(0), 6);
    assert_eq!(r.deck_size(0), 29);
}

#[test]
fn fortitude_replays_loaderleomon_if_it_had_a_digivolution_card() {
    let mut r = start(builder(CARD));
    let carrier = r.place_stack(0, &["TEXT-4", CARD]);
    let enemy = r.place_on_field(1, "TEXT-6", Some(0));
    r.battle_digimon(carrier, enemy);
    r.assert_prompt(SelectionKind::TriggerOrder, false, 0);
    let choices = r.pending_selection_view().unwrap().effect_choices.unwrap();
    assert_eq!(choices.len(), 2);
    let recover = choices
        .iter()
        .position(|choice| choice.keyword.is_none())
        .unwrap();
    r.execute_branch(recover).unwrap();
    assert_eq!(r.security_count(0), 6);
    r.assert_prompt(SelectionKind::OppField, true, 0);
    r.decline_prompt().unwrap();
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(ids(&r, &r.game.players[0].trash), ["TEXT-4"]);
}
