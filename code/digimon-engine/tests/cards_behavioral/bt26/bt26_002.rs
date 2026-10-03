//! BT26-002 Budmon — Lv.2 Green Digi-Egg, Vegetation/DATA SQUAD.
//!
//! Inherited: [Your Turn] [Once Per Turn] When effects trash cards from under
//! your Tamers, <Draw 1>.
//!
//! DCGO: BT26/Green/BT26_002.cs. Driven through a real effect trash: ST24-12
//! Falcomon's [On Play] ("by trashing the bottom face-down card from under any
//! of your Tamers").

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::CardColor;

const CARD_ID: &str = "BT26-002";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-002")
        .dsl_card("ST24-12")
        .expect("ST24-12")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Green, &["DATA SQUAD"]))
        .add_card(digimon(
            "DS-DIGI",
            "DS",
            CardColor::Purple,
            4,
            5,
            &["DATA SQUAD"],
        ))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

fn trash_source_via_falcomon(r: &mut DebugRunner) {
    push_trash(r, 0, "DS-DIGI");
    let falco = r.place_on_field(0, "ST24-12", Some(0));
    r.fire_on_play(0, falco.index as usize);
    r.accept_optional_trigger().expect("accept Falcomon");
    pick_first(r, 0); // Tamer (cost)
    let _ = r.auto_resolve();
}

#[test]
fn bt26_002_draws_when_an_effect_trashes_from_under_a_tamer() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "FILLER"]); // Budmon as inherited source
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 2);
    let deck_before = r.deck_size(0);
    trash_source_via_falcomon(&mut r);
    assert_eq!(sources(&r, t), 1, "cost paid");
    assert_eq!(r.deck_size(0), deck_before - 1, "<Draw 1> fired");
}

#[test]
fn bt26_002_once_per_turn() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "FILLER"]);
    tamer_with_face_down(&mut r, 0, "TAMER", 3);
    let deck_before = r.deck_size(0);
    trash_source_via_falcomon(&mut r);
    trash_source_via_falcomon(&mut r);
    assert_eq!(r.deck_size(0), deck_before - 1, "only one draw per turn");
}

#[test]
fn bt26_002_not_on_opponents_turn() {
    let mut r = setup();
    r.place_stack(0, &[CARD_ID, "FILLER"]);
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 2);
    r.set_first_player(1); // it is now P1's turn
    assert_eq!(r.turn_player(), 1);
    let deck_before = r.deck_size(0);
    trash_source_via_falcomon(&mut r);
    assert_eq!(sources(&r, t), 1, "the effect trash still happened");
    assert_eq!(r.deck_size(0), deck_before, "[Your Turn] gate");
}
