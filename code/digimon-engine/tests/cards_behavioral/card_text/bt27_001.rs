//! Source: `data/card_bundles/BT27-001.md` (printed text).
//!
//! ### Inherited Effect
//! [Your Turn] [Once Per Turn] When any of your Digimon with [Huckmon] in their texts suspend, ＜Draw 1＞.
//!
//! ## Official Q&A
//! - It refers to a card that contains the specified text or icon in its name, traits, effects, inherited effects, (Rule), digivolve requirements, DNA Digivolve, DigiXros requirements, Burst Digivolve, App Fusion, Link, or Assembly requirements. For example, "a card with [Knightmon] in its text" would include cards with the name [DarkKnightmon] and cards with the text [Knightmon] in their effects.

use super::common::*;
use digimon_engine::debug_runner::make_test_card;

const CARD: &str = "BT27-001";

#[test]
fn allied_huckmon_text_suspend_draws_once() {
    let mut r = start(builder(CARD).add_card(make_test_card("HUCK", "Huckmon")));
    r.place_stack(0, &[CARD, "TEXT-3"]);
    let huck = r.place_on_field(0, "HUCK", Some(0));
    r.game.suspend(huck);
    assert_eq!(r.hand_size(0), 1);
    let second = r.place_on_field(0, "HUCK", Some(0));
    r.game.suspend(second);
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn other_suspensions_do_not_draw() {
    let mut r = start(builder(CARD).add_card(make_test_card("HUCK", "Huckmon")));
    let carrier = r.place_stack(0, &[CARD, "TEXT-3"]);
    let enemy = r.place_on_field(1, "HUCK", Some(0));
    r.game.suspend(enemy);
    r.game.suspend(carrier);
    assert_eq!(r.hand_size(0), 0);
}
