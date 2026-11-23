//! BT27-029: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Blue Lv.4 / cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Iceclad＞ [On Play] [When Digivolving] Trash any 3 digivolution cards from your opponent's Digimon. Then, if they don't have a Digimon with digivolution cards, this Digimon gains ＜Blocker＞ and can't be deleted other than in battle until their turn ends. (Rule) Trait: Has [Ice-Snow] Type.
//!
//! ### Inherited Effect
//! [End of Your Turn] [Once Per Turn] Trash the bottom 2 digivolution cards of 1 of your opponent's Digimon. If this effect didn't trash, this [Ice-Snow] trait Digimon may unsuspend.
//!
//! ## Official Q&A
//! - Yes.

use super::bt27_gemmon_fixtures::fixture;
use super::common::{action, ids, start};
use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
fn sekkamon_trashes_available_sources_and_gains_blocker() {
    let mut r = start(fixture(&["BT27-029"]).hand(0, &["BT27-029"]));
    r.place_stack(1, &["ICE", "FILLER"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(
        SelectionKind::SourceMulti {
            min: 1,
            max: 1,
            picked: 0,
        },
        false,
        0,
    );
    r.choose_prompt_card("ICE", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["ICE"]);
    let sekkamon = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(r.game.has_keyword(sekkamon, Keyword::Iceclad));
    assert!(r.game.has_keyword(sekkamon, Keyword::Blocker));
    r.game
        .delete_permanent_with_cause(sekkamon, ReplacementCause::OpponentEffect);
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    r.game
        .delete_permanent_with_cause(sekkamon, ReplacementCause::Battle);
    assert!(r.game.players[0].battle_area.is_empty());
}

#[test]
fn sekkamon_inherited_unsuspends_only_an_ice_snow_carrier_when_nothing_was_trashed() {
    for (top, sourced, accept) in [
        ("ICE", false, true),
        ("ICE", false, false),
        ("ICE", true, false),
        ("FILLER", false, false),
    ] {
        let mut r = start(fixture(&["BT27-029"]));
        let carrier = r.place_stack(0, &["BT27-029", top]);
        r.game.players[0].battle_area[carrier.index as usize].is_suspended = true;
        if sourced {
            r.place_stack(1, &["ALYSION", "FILLER"]);
        } else {
            r.place_stack(1, &["FILLER"]);
        }
        action(&mut r, 0, PASS);
        r.choose_prompt_card("FILLER", None).unwrap();
        if top == "ICE" && !sourced {
            r.assert_prompt(SelectionKind::OwnField, true, 0);
            if accept {
                r.choose_prompt_card(top, None).unwrap();
            } else {
                r.decline_prompt().unwrap();
            }
        }
        assert_eq!(r.game.players[0].battle_area[0].is_suspended, !accept);
        assert_eq!(r.game.players[1].trash.len(), usize::from(sourced));
    }
}

#[test]
fn sekkamon_inherited_can_unsuspend_with_no_opposing_digimon() {
    let mut r = start(fixture(&["BT27-029"]));
    let carrier = r.place_stack(0, &["BT27-029", "ICE"]);
    r.game.players[0].battle_area[carrier.index as usize].is_suspended = true;
    action(&mut r, 0, PASS);
    r.assert_prompt(SelectionKind::OwnField, true, 0);
    r.choose_prompt_card("ICE", None).unwrap();
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}
