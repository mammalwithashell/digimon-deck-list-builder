//! BT5-107 Revive From the Darkness! — Option, Purple, cost 4.
//!
//! # Card text (official Bandai DB — data/card_bundles/BT5-107.md)
//! [Main] Delete 1 of your purple Digimon. Then, you may play 1 level 5 or
//! lower purple Digimon card from your trash without paying the cost. [On Play]
//! effects on Digimon this effect played don't activate.
//! Security Effect
//! [Security] Add this card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT5/Purple/BT5_107.cs — OptionSkill: a
//! MANDATORY delete of 1 own purple Digimon (when one exists), then a
//! `canNoSelect` trash pick (purple, HasLevel, level <= 5,
//! CanPlayAsNewPermanent) played with `activateETB: false`; SecuritySkill:
//! AddThisCardToHand.

use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};

#[path = "../ex8/nso_support.rs"]
mod nso_support;
use nso_support::*;

const CARD_ID: &str = "BT5-107";

fn builder() -> DebugRunnerBuilder {
    let mut gains = plain("ON-PLAY-GAIN", 4, 4000, CardColor::Purple);
    gains.effect_text = "[On Play] Gain 3 memory.".to_string();
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT5-107 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(plain("P-FIELD", 4, 4000, CardColor::Purple))
        .add_card(plain("P-FIELD-2", 4, 4000, CardColor::Purple))
        .add_card(plain("B-FIELD", 4, 4000, CardColor::Blue))
        .add_card({
            // A purple Tamer satisfies the Option's colour requirement without
            // being a purple DIGIMON (nothing to delete).
            let mut t = plain("P-TAMER", 0, 0, CardColor::Purple);
            t.card_kind = CardKind::Tamer;
            t.level = None;
            t.dp = None;
            t
        })
        .add_card(plain("P5-TRASH", 5, 5000, CardColor::Purple))
        .add_card(plain("P6-TRASH", 6, 6000, CardColor::Purple))
        .add_card(plain("B3-TRASH", 3, 3000, CardColor::Blue))
        .add_card(gains)
        .add_card(plain("ATK", 4, 4000, CardColor::Blue))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
}

fn use_option(r: &mut DebugRunner) -> String {
    let idx = push_hand(r, 0, CARD_ID);
    format!("{:?}", r.game.play_option_from_hand(0, idx))
}

#[test]
fn bt5_107_main_deletes_a_purple_digimon_then_plays_a_level_5_or_lower_purple_from_trash() {
    let mut r = builder().start();
    r.place_on_field(0, "P-FIELD", Some(0));
    r.place_on_field(0, "B-FIELD", Some(0));
    push_trash(&mut r, 0, "P5-TRASH");
    push_trash(&mut r, 0, "P6-TRASH");
    push_trash(&mut r, 0, "B3-TRASH");
    let res = use_option(&mut r);
    assert_ne!(res, "Invalid");
    // The delete is mandatory and only purple Digimon are legal.
    let view = r.pending_selection_view().expect("delete pick");
    assert!(!view.is_optional);
    pick_field(&mut r, 0, "P-FIELD");
    assert!(trash_ids(&r, 0).contains(&"P-FIELD".to_string()), "the purple Digimon was deleted");
    // The play is optional and limited to purple level <= 5.
    let view = r.pending_selection_view().expect("trash play pick");
    assert!(view.is_optional, "'you may play'");
    let offered = offered_ids(&r);
    assert!(offered.contains(&"P5-TRASH".to_string()), "{offered:?}");
    assert!(offered.contains(&"P-FIELD".to_string()), "the just-deleted purple Digimon is a legal pick: {offered:?}");
    assert!(!offered.contains(&"P6-TRASH".to_string()), "level 6: {offered:?}");
    assert!(!offered.contains(&"B3-TRASH".to_string()), "not purple: {offered:?}");
    let mem = r.memory();
    pick_trash(&mut r, "P5-TRASH");
    drain_declining(&mut r);
    assert!(field_ids(&r, 0).contains(&"P5-TRASH".to_string()));
    assert_eq!(r.memory(), mem, "without paying the cost");
}

#[test]
fn bt5_107_the_play_is_optional() {
    let mut r = builder().start();
    r.place_on_field(0, "P-FIELD", Some(0));
    push_trash(&mut r, 0, "P5-TRASH");
    use_option(&mut r);
    pick_field(&mut r, 0, "P-FIELD");
    pass(&mut r);
    drain_declining(&mut r);
    assert!(!field_ids(&r, 0).contains(&"P5-TRASH".to_string()));
}

#[test]
fn bt5_107_on_play_effects_of_the_played_digimon_do_not_activate() {
    let mut r = builder().start();
    r.place_on_field(0, "P-FIELD", Some(0));
    push_trash(&mut r, 0, "ON-PLAY-GAIN");
    let mem_before = r.memory();
    use_option(&mut r);
    pick_field(&mut r, 0, "P-FIELD");
    pick_trash(&mut r, "ON-PLAY-GAIN");
    drain_declining(&mut r);
    assert!(field_ids(&r, 0).contains(&"ON-PLAY-GAIN".to_string()));
    assert_eq!(r.memory(), mem_before - 4, "only the Option's cost was paid: no [On Play] memory gain");
}

#[test]
fn bt5_107_without_a_purple_digimon_nothing_is_deleted_but_the_play_is_still_offered() {
    let mut r = builder().start();
    r.place_on_field(0, "B-FIELD", Some(0));
    r.place_on_field(0, "P-TAMER", Some(0));
    push_trash(&mut r, 0, "P5-TRASH");
    use_option(&mut r);
    // DCGO: the delete is skipped when no purple Digimon exists; the trash play
    // is independent of it.
    let view = r.pending_selection_view().expect("trash play pick");
    assert!(view.is_optional);
    pick_trash(&mut r, "P5-TRASH");
    drain_declining(&mut r);
    assert!(field_ids(&r, 0).contains(&"B-FIELD".to_string()));
    assert!(field_ids(&r, 0).contains(&"P5-TRASH".to_string()));
}

#[test]
fn bt5_107_security_adds_this_card_to_the_hand() {
    let mut r = builder().start();
    let atk = r.place_on_field(1, "ATK", Some(0));
    let idx = data_idx(&r, CARD_ID);
    let next = r.game.next_card_index();
    r.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    r.set_first_player(1);
    r.attack_player(atk, 0, false);
    drain_declining(&mut r);
    assert!(hand_ids(&r, 0).contains(&CARD_ID.to_string()), "added to the hand: {:?}", hand_ids(&r, 0));
    assert!(!trash_ids(&r, 0).contains(&CARD_ID.to_string()));
}
