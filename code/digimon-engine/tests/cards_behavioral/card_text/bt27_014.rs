//! BT27-014: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.4 / cost 3
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.4 w/[Gemmon] in text: Cost 3
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Engage＞ [On Play] [When Digivolving] 1 of your Digimon gains ＜Piercing＞ and +3000 DP for the turn. [Your Turn] This Digimon with 12000 DP or more gains ＜Rush＞.
//!
//! ### Inherited Effect
//! [Your Turn] This Digimon with [Gemmon] in its text gains ＜Engage＞. (At the end of your turn, this Digimon may attack.)
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

use super::bt27_gemmon_fixtures::{fixture, gemmon_text_digivolution};
use super::common::{action, start};
use digimon_engine::action::space::{encode_digivolve, PASS, PLAY_HAND_START};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn reptilimon_alternate_digivolution_finds_gemmon_in_requirements() {
    gemmon_text_digivolution("BT27-014", 4, 3);
}

#[test]
fn reptilimon_gains_rush_when_its_digivolution_effect_reaches_12000_dp() {
    let mut r = start(fixture(&["BT27-006", "BT27-011", "BT27-014"]).hand(0, &["BT27-014"]));
    let reptilimon = r.place_stack(0, &["BT27-006", "BT27-011"]);
    action(&mut r, 0, encode_digivolve(0, reptilimon.index as u16));
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("BT27-014", None).unwrap();
    assert_eq!(r.effective_dp(reptilimon), Some(12000));
    assert!(r.game.has_keyword(reptilimon, Keyword::Piercing));
    assert!(r.game.has_keyword(reptilimon, Keyword::Rush));
}

#[test]
fn reptilimon_can_grant_piercing_elsewhere_without_gaining_rush() {
    let mut r = start(fixture(&["BT27-014"]).hand(0, &["BT27-014"]));
    let ally = r.place_stack(0, &["FILLER"]);
    action(&mut r, 0, PLAY_HAND_START);
    r.choose_prompt_card("FILLER", None).unwrap();
    assert_eq!(r.effective_dp(ally), Some(5000));
    assert!(r.game.has_keyword(ally, Keyword::Piercing));
    let reptilimon = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 1,
    };
    assert!(r.game.has_keyword(reptilimon, Keyword::Engage));
    assert!(!r.game.has_keyword(reptilimon, Keyword::Rush));
}

#[test]
fn reptilimon_inherited_engage_requires_gemmon_text_and_your_turn() {
    let mut r = start(fixture(&["BT27-006", "BT27-014"]));
    let matching = r.place_stack(0, &["BT27-014", "BT27-006"]);
    let nonmatching = r.place_stack(0, &["BT27-014", "FILLER"]);
    assert!(r.game.has_keyword(matching, Keyword::Engage));
    assert!(!r.game.has_keyword(nonmatching, Keyword::Engage));
    action(&mut r, 0, PASS);
    r.decline_prompt().unwrap();
    assert!(!r.game.has_keyword(matching, Keyword::Engage));
}
