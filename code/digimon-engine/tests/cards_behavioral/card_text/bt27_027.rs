//! BT27-027: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Blue Lv.3 / cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Iceclad＞ [On Play] [When Digivolving] Trash the bottom 2 digivolution cards of 1 of your opponent's Digimon. Then, you may place 1 level 4 or lower [Ice-Snow] trait Digimon card from your hand as any of your Digimon's bottom digivolution card. (Rule) Trait: Has [Ice-Snow] Type.
//!
//! ### Inherited Effect
//! [End of Attack] [Once Per Turn] Gain 1 memory.

use super::bt27_gemmon_fixtures::fixture;
use super::common::{action, ids, start};
use digimon_engine::action::space::{encode_attack, PLAY_HAND_START, SECURITY_TARGET};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn tenkomon_trashes_bottom_sources_and_can_put_ice_snow_under_itself() {
    let mut r = start(fixture(&["BT27-027"]).hand(0, &["BT27-027", "ICE"]));
    r.place_stack(1, &["FILLER", "ICE", "ALYSION", "FILLER"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::OppField, false, 0);
    r.choose_prompt_card("FILLER", None).unwrap();
    assert_eq!(ids(&r, &r.game.players[1].trash), ["FILLER", "ICE"]);
    r.assert_prompt(SelectionKind::Hand, true, 0);
    r.choose_prompt_card("ICE", None).unwrap();
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("BT27-027", None).unwrap();
    assert_eq!(
        ids(&r, &r.game.players[0].battle_area[0].card_sources),
        ["ICE", "BT27-027"]
    );
}

#[test]
fn tenkomon_can_decline_placing_a_card_when_there_is_nothing_to_trash() {
    let mut r = start(fixture(&["BT27-027"]).hand(0, &["BT27-027", "ICE"]));
    action(&mut r, 0, PLAY_HAND_START);
    r.assert_prompt(SelectionKind::Hand, true, 0);
    r.decline_prompt().unwrap();
    assert_eq!(ids(&r, &r.game.players[0].hand), ["ICE"]);
    let tenkomon = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert!(r.game.has_keyword(tenkomon, Keyword::Iceclad));
}

#[test]
fn tenkomon_inherited_gains_memory_at_end_of_attack_once_per_turn() {
    let mut r = start(fixture(&["BT27-027"]).memory(4));
    let carrier = r.place_stack(0, &["BT27-027", "ICE"]);
    action(
        &mut r,
        0,
        encode_attack(carrier.index as u16, SECURITY_TARGET),
    );
    assert_eq!(r.memory(), 5);
    r.game.players[0].battle_area[carrier.index as usize].is_suspended = false;
    action(
        &mut r,
        0,
        encode_attack(carrier.index as u16, SECURITY_TARGET),
    );
    assert_eq!(r.memory(), 5);
}
