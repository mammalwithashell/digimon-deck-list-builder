//! 4-19 Option COLOR REQUIREMENTS -- the "all of its colors" rule.
//!
//! 4-19-2 "To meet color requirements, you must have a Digimon or Tamer on your
//! field that's the same color as the Option card you want to use."
//! 4-19-3 "An Option card with multiple colors can't be used unless the color
//! requirements are met for ALL of its colors."
//! 4-19-4 "A multicolor Digimon or multicolor Tamer can meet the color
//! requirements for multiple colors."
//!
//! `action::mask::option_color_match_available` used to ask whether any SINGLE
//! permanent shared any ONE of the Option's colors, so a mono-color board could
//! pay for a multicolor Option. 63 Option cards in data/cards.json print more
//! than one color, so this was reachable. DCGO agrees with the manual:
//! CardSource.cs:307-310 uses `colorsToCheck.Every(...)`.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn option_card(id: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c.level = None;
    c.dp = None;
    c.play_cost = 0;
    c.colors = colors.to_vec();
    c
}

fn digimon(id: &str, colors: &[CardColor]) -> CardData {
    let mut c = make_test_card_with_level(id, id, 4);
    c.card_kind = CardKind::Digimon;
    c.colors = colors.to_vec();
    c.play_cost = 3;
    c.dp = Some(5000);
    c
}

/// Can player 0 legally use the Option in hand slot 0?
fn option_is_playable(r: &DebugRunner) -> bool {
    use digimon_engine::action::build_action_mask;
    use digimon_engine::action::space::PLAY_HAND_START;
    build_action_mask(&r.game, 0)[PLAY_HAND_START as usize] > 0.0
}

/// THE BUG: a RED-only board must NOT satisfy a RED/BLUE Option (4-19-3).
#[test]
fn multicolor_option_needs_every_color_not_just_one() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("RB-OPTION", &[CardColor::Red, CardColor::Blue]))
        .add_card(digimon("RED-MON", &[CardColor::Red]))
        .hand(0, &["RB-OPTION"])
        .memory(10)
        .start();
    r.place_on_field(0, "RED-MON", Some(0));

    assert!(
        !option_is_playable(&r),
        "4-19-3: a red/blue Option can't be used off a red-only board -- the \
         requirement must be met for ALL of its colors, not any one of them"
    );
}

/// CONTROL for the test above: the same Option becomes legal the moment the
/// missing color is on the field. Without this, the assertion above would pass
/// just as happily if the mask never offered the Option at all.
#[test]
fn multicolor_option_is_playable_once_every_color_is_present() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("RB-OPTION", &[CardColor::Red, CardColor::Blue]))
        .add_card(digimon("RED-MON", &[CardColor::Red]))
        .add_card(digimon("BLUE-MON", &[CardColor::Blue]))
        .hand(0, &["RB-OPTION"])
        .memory(10)
        .start();
    r.place_on_field(0, "RED-MON", Some(0));
    r.place_on_field(0, "BLUE-MON", Some(0));

    assert!(
        option_is_playable(&r),
        "CONTROL FAILED: with both colors on the field the Option must be \
         playable -- if this is false the negative test above proves nothing"
    );
}

/// 4-19-4: ONE multicolor Digimon may cover several of the Option's colors, so
/// the fix must not have become "one permanent per color".
#[test]
fn one_multicolor_digimon_covers_several_option_colors() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("RB-OPTION", &[CardColor::Red, CardColor::Blue]))
        .add_card(digimon("RB-MON", &[CardColor::Red, CardColor::Blue]))
        .hand(0, &["RB-OPTION"])
        .memory(10)
        .start();
    r.place_on_field(0, "RB-MON", Some(0));

    assert!(
        option_is_playable(&r),
        "4-19-4: a multicolor Digimon meets the color requirements for multiple \
         colors, so one red/blue Digimon alone satisfies a red/blue Option"
    );
}

/// A mono-color Option is unaffected by the change -- the common case must not
/// have regressed into needing anything extra.
#[test]
fn mono_color_option_still_matches_a_single_same_color_digimon() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("R-OPTION", &[CardColor::Red]))
        .add_card(digimon("RED-MON", &[CardColor::Red]))
        .hand(0, &["R-OPTION"])
        .memory(10)
        .start();
    r.place_on_field(0, "RED-MON", Some(0));

    assert!(
        option_is_playable(&r),
        "a mono-color Option off a matching board is the ordinary case"
    );
}

/// ...and a mono-color Option still fails against the wrong color.
#[test]
fn mono_color_option_still_rejected_by_a_different_colored_board() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("R-OPTION", &[CardColor::Red]))
        .add_card(digimon("BLUE-MON", &[CardColor::Blue]))
        .hand(0, &["R-OPTION"])
        .memory(10)
        .start();
    r.place_on_field(0, "BLUE-MON", Some(0));

    assert!(
        !option_is_playable(&r),
        "4-19-2: a red Option needs a red Digimon or Tamer on the field"
    );
}
