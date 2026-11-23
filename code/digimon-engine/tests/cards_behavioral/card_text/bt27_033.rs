//! BT27-033: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Blue Lv.5 / cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Iceclad＞ ＜Piercing＞ ＜Blocker＞ [On Play] [When Digivolving] Trash the top 2 digivolution cards of all of your opponent's Digimon. Then, until their turn ends, none of their Digimon without digivolution cards can digivolve or suspend. [End of Your Turn] [Once Per Turn] If your opponent has no Digimon with digivolution cards, 1 of your [Ice-Snow] trait Digimon may gain ＜Rush＞ for the turn and attack.
//!
//! ### Special Play Condition
//! Assembly -5: Lv.5 × Lv.4 × Lv.3, all w/[Ice-Snow] trait
//!
//! ## Official Q&A
//! - No, you can't. A new attack declaration can't be made during an attack. In this example, when you have an attacking Digimon, you can't make them attack even if you activate ED01-10 [Bond of Snowflakes]'s effect. Related Cards

use super::bt27_gemmon_fixtures::{assemble, fixture};
use super::common::{action, ids, start};
use digimon_engine::action::space::{
    encode_attack, encode_digivolve, PASS, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::debug_runner::make_test_card_with_level;
use digimon_engine::enums::{CardColor, Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;

#[test]
fn yukinamon_trashes_the_top_two_sources_from_each_opponent() {
    let mut r = start(fixture(&["BT27-033"]).hand(0, &["BT27-033"]).memory(15));
    r.place_stack(1, &["ALYSION", "ICE", "ICE", "FILLER"]);
    r.place_stack(1, &["ICE", "FILLER"]);
    r.place_stack(1, &["ICE", "BLUE-TAMER"]);
    action(&mut r, 0, PLAY_HAND_START);
    let area = &r.game.players[1].battle_area;
    assert_eq!(ids(&r, &area[0].card_sources), ["ALYSION", "FILLER"]);
    assert_eq!(ids(&r, &area[1].card_sources), ["FILLER"]);
    assert_eq!(ids(&r, &area[2].card_sources), ["ICE", "BLUE-TAMER"]);
}

#[test]
fn yukinamon_only_locks_sourceless_digimon_and_cannot_attack_if_sources_remain() {
    let mut base = make_test_card_with_level("BLUE-5", "Blue Base", 5);
    base.colors = vec![CardColor::Blue];
    let mut r = start(fixture(&["BT27-033"]).add_card(base).hand(0, &["BT27-033"]));
    let yukinamon = r.place_stack(0, &["BLUE-5"]);
    let sourced = r.place_stack(1, &["ALYSION", "ICE", "ICE", "FILLER"]);
    let sourceless = r.place_stack(1, &["ICE", "FILLER"]);
    action(&mut r, 0, encode_digivolve(0, u16::from(yukinamon.index)));
    for modifier in [ModifierType::CannotDigivolve, ModifierType::CannotSuspend] {
        assert!(!r.game.modifiers.has(sourced, modifier));
        assert!(r.game.modifiers.has(sourceless, modifier));
    }
    for keyword in [Keyword::Iceclad, Keyword::Piercing, Keyword::Blocker] {
        assert!(r.game.has_keyword(yukinamon, keyword));
    }
    action(&mut r, 0, PASS);
    assert_eq!(r.turn_player(), 1);
    assert!(r.pending_selection().is_none());
    for modifier in [ModifierType::CannotDigivolve, ModifierType::CannotSuspend] {
        assert!(r.game.modifiers.has(sourceless, modifier));
    }
    action(&mut r, 1, PASS);
    assert_eq!(r.turn_player(), 0);
    for modifier in [ModifierType::CannotDigivolve, ModifierType::CannotSuspend] {
        assert!(!r.game.modifiers.has(sourceless, modifier));
    }
}

#[test]
fn yukinamon_end_of_turn_can_give_a_new_ice_snow_digimon_rush_and_attack() {
    for accept in [false, true] {
        let mut r = start(fixture(&["BT27-033"]).hand(0, &["ICE"]));
        r.place_stack(0, &["BT27-033"]);
        action(&mut r, 0, PLAY_HAND_START);
        action(&mut r, 0, PASS);
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        if accept {
            r.choose_prompt_card("ICE", None).unwrap();
            action(&mut r, 0, encode_attack(1, SECURITY_TARGET));
        } else {
            r.decline_prompt().unwrap();
        }
        assert_eq!(r.security_count(1), if accept { 2 } else { 3 });
    }
}

#[test]
fn yukinamon_assembles_three_ice_snow_levels() {
    let builder = fixture(&["BT27-033", "BT27-029", "BT27-027", "BT27-022"]);
    assemble(builder, "BT27-033", ["BT27-029", "BT27-027", "BT27-022"]);
}
