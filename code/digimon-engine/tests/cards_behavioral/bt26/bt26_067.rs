//! BT26-067 Wizardmon — Lv.4 Purple/Red, Wizard/Witchelny/Iliad/TS.
//!
//! [On Play] [When Digivolving] <Draw 1> and trash 1 card in your hand.
//! [End of Your Turn] If you have a blue or yellow Digimon, by returning this
//! Digimon to the bottom of the deck, you may play 1 red or blue Digimon card
//! with the [Iliad] trait from your trash with the cost reduced by 4.
//! Inherited: <Retaliation>.
//!
//! DCGO: BT26/Purple/BT26_067.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming, Keyword};

const CARD_ID: &str = "BT26-067";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-067")
        .add_card(filler("FILLER"))
        .add_card(digimon("DRAWN", "Drawn", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("KEEP", "Keep", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("TS3", "TS Three", CardColor::Green, 3, 3, &["TS"]))
        .add_card(digimon(
            "BLUE-ALLY",
            "Blue Ally",
            CardColor::Blue,
            4,
            5,
            &[],
        ))
        .add_card(digimon(
            "GREEN-ALLY",
            "Green Ally",
            CardColor::Green,
            4,
            5,
            &[],
        ))
        .add_card(digimon_colors(
            "ILIAD-BLUE",
            6,
            7,
            &[CardColor::Blue],
            &["Iliad"],
        ))
        .add_card(digimon_colors(
            "ILIAD-GREEN",
            6,
            7,
            &[CardColor::Green],
            &["Iliad"],
        ))
        .add_card(digimon_colors(
            "BLUE-NOT-ILIAD",
            6,
            7,
            &[CardColor::Blue],
            &["TS"],
        ))
        .deck(0, &["FILLER", "FILLER", "DRAWN"])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_067_ts_alt_digivolve_path_compiled() {
    let r = setup();
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.alt_paths.len(), 1, "[Digivolve] Lv.3 w/[TS]: Cost 2");
}

#[test]
fn bt26_067_on_play_draws_then_trashes_a_chosen_card() {
    let mut r = setup();
    push_hand(&mut r, 0, "KEEP");
    let w = r.place_on_field(0, CARD_ID, Some(0));
    fire(&mut r, EffectTiming::OnPlay, w);
    let v = r.pending_selection_view().expect("mandatory discard");
    assert!(!v.is_optional, "the trash is mandatory");
    assert_eq!(v.valid_action_ids.len(), 2);
    pick_hand(&mut r, 0, "DRAWN");
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["KEEP".to_string()]);
    assert_eq!(trash_ids(&r, 0), vec!["DRAWN".to_string()]);
}

#[test]
fn bt26_067_when_digivolving_draw_and_trash() {
    let mut r = setup();
    let w = r.place_stack(0, &["TS3", CARD_ID]);
    fire(&mut r, EffectTiming::WhenDigivolving, w);
    pick_hand(&mut r, 0, "DRAWN");
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(trash_ids(&r, 0), vec!["DRAWN".to_string()]);
}

#[test]
fn bt26_067_eot_bottom_decks_self_and_plays_iliad_from_trash_at_minus_four() {
    let mut r = setup();
    let w = r.place_stack(0, &["TS3", CARD_ID]);
    r.place_on_field(0, "BLUE-ALLY", Some(0));
    push_trash(&mut r, 0, "ILIAD-GREEN");
    push_trash(&mut r, 0, "BLUE-NOT-ILIAD");
    push_trash(&mut r, 0, "ILIAD-BLUE");
    let mem = r.memory();
    fire(&mut r, EffectTiming::EndOfYourTurn, w);
    assert!(r.pending_is_optional());
    r.accept_optional_trigger().expect("accept");
    assert_eq!(deck_ids(&r, 0)[0], CARD_ID, "bottom of the deck");
    assert!(!field_ids(&r, 0).contains(&CARD_ID.to_string()));
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional);
    assert_eq!(v.valid_action_ids.len(), 1, "only the blue [Iliad] Digimon");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"ILIAD-BLUE".to_string()));
    assert_eq!(r.memory(), mem - 3, "cost 7 reduced by 4");
}

#[test]
fn bt26_067_eot_requires_blue_or_yellow_digimon() {
    let mut r = setup();
    let w = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "GREEN-ALLY", Some(0));
    push_trash(&mut r, 0, "ILIAD-BLUE");
    fire(&mut r, EffectTiming::EndOfYourTurn, w);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt26_067_eot_declined_keeps_wizardmon() {
    let mut r = setup();
    let w = r.place_on_field(0, CARD_ID, Some(0));
    r.place_on_field(0, "BLUE-ALLY", Some(0));
    fire(&mut r, EffectTiming::EndOfYourTurn, w);
    r.decline_optional_trigger().expect("decline");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&CARD_ID.to_string()));
}

#[test]
fn bt26_067_inherits_retaliation() {
    let mut r = setup();
    let h = r.place_stack(0, &[CARD_ID, "TS3"]);
    assert!(r.game.has_keyword(h, Keyword::Retaliation));
}
