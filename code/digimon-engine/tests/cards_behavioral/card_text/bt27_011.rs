//! BT27-011: official Bandai card text.
//!
//! ### Standard digivolve cost circles
//!   - Red Lv.3 / cost 2
//! ### Special digivolution condition (DNA / alt-digivolve black text)
//!   [Digivolve] Lv.3 w/[Gemmon] in text: Cost 2
//!
//! ## Printed text (official)
//! ### Effect
//! ＜Raid＞ [When Moving] [When Digivolving] 1 of your Digimon gets +3000 DP for the turn. Then, if you have a Digimon with 8000 DP or more, ＜Draw 1＞.
//!
//! ### Inherited Effect
//! ＜Raid＞ (When this Digimon attacks, you may change the attack target to 1 of your opponent's unsuspended Digimon with the highest DP.)
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

use super::bt27_gemmon_fixtures::{fixture, gemmon_text_digivolution};
use super::common::{action, candidates, start};
use digimon_engine::action::space::encode_digivolve;
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn liskmon_alternate_digivolution_finds_gemmon_in_requirements() {
    gemmon_text_digivolution("BT27-011", 3, 2);
}

#[test]
fn liskmon_draws_after_buff_reaches_threshold() {
    let mut r = start(fixture(&["BT27-011", "BT27-006"]).hand(0, &["BT27-011"]));
    let stack = r.place_stack(0, &["BT27-006"]);
    action(&mut r, 0, encode_digivolve(0, stack.index as u16));
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    assert_eq!(candidates(&r), ["BT27-011"]);
    r.choose_prompt_card("BT27-011", None).unwrap();
    assert_eq!(r.effective_dp(stack), Some(9000));
    assert_eq!(r.hand_size(0), 2);
}

#[test]
fn liskmon_moving_grants_dp_but_does_not_draw_below_8000() {
    let mut r = start(fixture(&["BT27-011"]));
    r.place_in_breeding(0, "BT27-011");
    assert!(r.move_from_breeding(0));
    r.assert_prompt(SelectionKind::OwnField, false, 0);
    r.choose_prompt_card("BT27-011", None).unwrap();
    let liskmon = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 0,
    };
    assert_eq!(r.effective_dp(liskmon), Some(7000));
    assert_eq!(r.hand_size(0), 0);
    assert!(r.game.has_keyword(liskmon, Keyword::Raid));
    let inherited = r.place_stack(0, &["BT27-011", "FILLER"]);
    assert!(r.game.has_keyword(inherited, Keyword::Raid));
}
