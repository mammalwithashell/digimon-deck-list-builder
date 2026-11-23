//! 4-22 Option COLOR REQUIREMENTS -- the "all of its colors" rule.
//!
//! 4-22-2 "To meet color requirements, you must have a Digimon or Tamer on your
//! field that's the same color as the Option card you want to use."
//! 4-22-3 "An Option card with multiple colors can't be used unless the color
//! requirements are met for ALL of its colors."
//! 4-22-4 "A multicolor Digimon or multicolor Tamer can meet the color
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

/// THE BUG: a RED-only board must NOT satisfy a RED/BLUE Option (4-22-3).
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
        "4-22-3: a red/blue Option can't be used off a red-only board -- the \
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

/// 4-22-4: ONE multicolor Digimon may cover several of the Option's colors, so
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
        "4-22-4: a multicolor Digimon meets the color requirements for multiple \
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
        "4-22-2: a red Option needs a red Digimon or Tamer on the field"
    );
}

/// 3-4-6: "The field is divided into the breeding area and the battle area", so
/// a Digimon in the BREEDING area is on your field and satisfies 4-22-2. The
/// 3-4-7 restrictions ("can't be affected by effects", "can't be chosen for
/// effects") are scoped to EFFECTS; a color requirement is a rules gate on
/// USING a card.
///
/// DCGO agrees: CardSource.cs:308-310 iterates `Owner.GetFieldPermanents()`,
/// and Player.cs:669-685 returns the whole `FieldPermanents` array unfiltered --
/// `GetBattleAreaPermanents` (Player.cs:621-637) is the one that narrows with
/// `isBattleAreaFrameID`. A review claimed the opposite; it was wrong, and this
/// test exists so the next reader does not have to re-litigate it.
#[test]
fn a_breeding_area_digimon_satisfies_the_color_requirement() {
    let mut r = DebugRunner::builder()
        .add_card(option_card("R-OPTION", &[CardColor::Red]))
        .add_card(digimon("RED-EGG", &[CardColor::Red]))
        .hand(0, &["R-OPTION"])
        .memory(10)
        .start();
    r.place_in_breeding(0, "RED-EGG");

    assert!(
        r.game.player(0).battle_area.is_empty(),
        "the point of this test is that the BATTLE area is empty -- if the \
         Digimon landed there instead, it proves nothing about breeding"
    );
    assert!(
        option_is_playable(&r),
        "4-19-2 + 3-4-6: a red Digimon in the breeding area is on your field, \
         so it meets a red Option's color requirement"
    );
}

// ─── Colours as currently modified (scan finding F04) ────────────────────────
//
// 4-22-2 asks for "a Digimon or Tamer on your field that's the same color as the
// Option card". A permanent's colour is its colour as effects currently leave
// it: DCGO CardSource.cs:308-310 tests `permanent.TopCard.CardColors`, and the
// `CardColors` getter (CardSource.cs:446-476) folds every
// `IChangeCardColorEffect` into the printed `BaseCardColors`. The mask used to
// read the top card's PRINTED colours, so a colour gained by effect (Omnimon:
// Merciful Mode (EX13-077) "[All Turns] This Digimon gains all colors in its
// digivolution cards") never counted, and a colour removed by a colour-change
// effect still did. These tests run on the production card data.

fn on_big_stack<F: FnOnce() + Send + 'static>(f: F) {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(f)
        .expect("spawn large-stack thread")
        .join()
        .expect("test thread panicked");
}

fn production_card(card_id: &str) -> CardData {
    digimon_engine::deck_tools::full_card_data()
        .get(card_id)
        .unwrap_or_else(|| panic!("{card_id} present in production cards.json"))
        .clone()
}

const OMNIMON_MERCIFUL: &str = "EX13-077"; // Omnimon: Merciful Mode, white
const SANMYOJIN_ARRIVAL: &str = "EX12-070"; // Sanmyojin Arrival, yellow/green/blue Option
const HAMMER_SPARK: &str = "ST2-13"; // Hammer Spark, blue Option, no Use Req.
const GOMAMON: &str = "ST2-02"; // Gomamon, blue Lv.3, no text
const TAPIRMON: &str = "ST3-03"; // Tapirmon, yellow Lv.3, no text
const KUNEMON: &str = "ST4-05"; // Kunemon, green Lv.3, no text

/// A runner on production data: Omnimon: Merciful Mode (EX13-077) with its
/// YAML effects, the three vanilla sources, and `option` (production data,
/// with its own YAML so its Use Req. predicate is live) in hand.
fn omnimon_runner(option: &str) -> DebugRunner {
    let mut b = DebugRunner::builder()
        .add_card(production_card(OMNIMON_MERCIFUL))
        .add_card(production_card(option))
        .add_card(production_card(GOMAMON))
        .add_card(production_card(TAPIRMON))
        .add_card(production_card(KUNEMON))
        .dsl_card(OMNIMON_MERCIFUL)
        .expect("EX13-077 YAML in the embedded pack");
    b = b.dsl_card(option).expect("Option YAML in the embedded pack");
    b.hand(0, &[option]).memory(10).start()
}

/// Sanmyojin Arrival (EX12-070) is yellow/green/blue; its <Use Req.> needs a
/// [TB] trait Digimon, and there is none. Omnimon: Merciful Mode (EX13-077)
/// is printed white, but over yellow, green and blue sources it HAS all four
/// colours, so 4-22-3/4-22-4 are met by that one Digimon.
#[test]
fn a_gained_color_meets_a_multicolor_option_requirement_on_production_data() {
    on_big_stack(|| {
        let mut r = omnimon_runner(SANMYOJIN_ARRIVAL);
        r.place_stack(0, &[GOMAMON, TAPIRMON, KUNEMON, OMNIMON_MERCIFUL]);
        r.game.tick_declarative_effects();
        assert!(
            option_is_playable(&r),
            "4-22-2..4: Omnimon: Merciful Mode (EX13-077) gains yellow, green and \
             blue from its digivolution cards, so Sanmyojin Arrival (EX12-070) \
             meets its color requirement"
        );
    });
}

/// CONTROL: the same Omnimon with no digivolution cards is white only, and
/// with no [TB] Digimon the <Use Req.> does not apply either -- the Option
/// must stay masked, or the test above proves nothing.
#[test]
fn printed_white_omnimon_alone_does_not_meet_sanmyojin_arrival() {
    on_big_stack(|| {
        let mut r = omnimon_runner(SANMYOJIN_ARRIVAL);
        r.place_on_field(0, OMNIMON_MERCIFUL, Some(0));
        r.game.tick_declarative_effects();
        assert!(
            !option_is_playable(&r),
            "a white-only board with no [TB] Digimon can't use a \
             yellow/green/blue Option"
        );
    });
}

/// A single-colour Option: blue Hammer Spark (ST2-13) off an Omnimon whose
/// only blue comes from a Gomamon (ST2-02) digivolution card.
#[test]
fn a_gained_color_meets_a_single_color_option_requirement_on_production_data() {
    on_big_stack(|| {
        let mut r = omnimon_runner(HAMMER_SPARK);
        r.place_stack(0, &[GOMAMON, OMNIMON_MERCIFUL]);
        r.game.tick_declarative_effects();
        assert!(
            option_is_playable(&r),
            "4-22-2: Omnimon: Merciful Mode (EX13-077) over a blue source is \
             blue, so Hammer Spark (ST2-13) is usable"
        );
    });
}

#[test]
fn single_color_option_masked_when_no_permanent_has_that_color_now() {
    on_big_stack(|| {
        let mut r = omnimon_runner(HAMMER_SPARK);
        r.place_stack(0, &[TAPIRMON, OMNIMON_MERCIFUL]);
        r.game.tick_declarative_effects();
        assert!(
            !option_is_playable(&r),
            "white + gained yellow is not blue: Hammer Spark (ST2-13) stays masked"
        );
    });
}

/// The other direction: a colour removed by a colour-change effect no longer
/// counts. Gomamon (ST2-02) is printed blue; "change ... into a white Digimon"
/// (Sukamon line, e.g. BT11-043) is a `ChangeBaseCardColor` to white.
#[test]
fn a_color_lost_to_a_color_change_effect_no_longer_meets_the_requirement() {
    use digimon_engine::enums::{Expiry, ModifierType};
    use digimon_engine::modifiers::{ModifierEntry, ModifierPayload};
    on_big_stack(|| {
        let mut r = DebugRunner::builder()
            .add_card(production_card(HAMMER_SPARK))
            .add_card(production_card(GOMAMON))
            .hand(0, &[HAMMER_SPARK])
            .memory(10)
            .start();
        let gomamon = r.place_on_field(0, GOMAMON, Some(0));
        assert!(
            option_is_playable(&r),
            "CONTROL: a printed-blue Gomamon (ST2-02) meets Hammer Spark (ST2-13)"
        );
        r.game.modifiers.add(
            gomamon,
            ModifierEntry::simple(ModifierType::ChangeBaseCardColor, 0, Expiry::Permanent, 1)
                .with_payload(ModifierPayload::Colors {
                    value: vec![CardColor::White],
                    base: true,
                }),
        );
        assert!(
            !option_is_playable(&r),
            "4-22-2: Gomamon (ST2-02) is now white, so it no longer meets the \
             blue requirement of Hammer Spark (ST2-13)"
        );
    });
}
