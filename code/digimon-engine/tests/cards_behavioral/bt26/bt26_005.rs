//! BT26-005 Pinamon — Lv.2 Purple Digi-Egg, Bird/DATA SQUAD.
//!
//! Inherited: [On Deletion] By trashing the bottom face-down card from under
//! any of your Tamers, you may play 1 play cost 5 or lower [Avian] or
//! [DATA SQUAD] trait card from your trash without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_005.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};

const CARD_ID: &str = "BT26-005";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-005")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Purple, &["DATA SQUAD"]))
        .add_card(digimon(
            "AVIAN5",
            "Peckmon",
            CardColor::Purple,
            4,
            5,
            &["Avian"],
        ))
        .add_card(digimon(
            "DS6",
            "Big",
            CardColor::Purple,
            5,
            6,
            &["DATA SQUAD"],
        ))
        .add_card(digimon(
            "PLAIN",
            "Plain",
            CardColor::Purple,
            3,
            3,
            &["Beast"],
        ))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_005_pays_face_down_cost_and_plays_from_trash() {
    let mut r = setup();
    let t = tamer_with_face_down(&mut r, 0, "TAMER", 1);
    push_trash(&mut r, 0, "AVIAN5");
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    r.accept_optional_trigger().expect("optional clause");
    pick_first(&mut r, 0); // the Tamer whose bottom FD card is trashed
    let v = r.pending_selection_view().expect("trash pick");
    assert!(v.is_optional, "'you may play'");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(sources(&r, t), 0, "bottom face-down card trashed");
    assert!(field_ids(&r, 0).contains(&"AVIAN5".to_string()));
}

#[test]
fn bt26_005_filter_cost_and_trait() {
    let mut r = setup();
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    push_trash(&mut r, 0, "DS6"); // cost 6 — too expensive
    push_trash(&mut r, 0, "PLAIN"); // wrong trait
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    r.accept_optional_trigger().expect("optional clause");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"DS6".to_string()));
    assert!(!field_ids(&r, 0).contains(&"PLAIN".to_string()));
}

/// DCGO BT26_005.cs:79 requires `HasPlayCost` — a Digi-Egg (no printed play
/// cost, cannot be played as a new permanent) is never a candidate, even with a
/// matching trait (Pinamon itself rides under a Digimon into the trash).
#[test]
fn bt26_005_excludes_digi_egg_from_trash() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-005")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Purple, &["DATA SQUAD"]))
        .add_card({
            let mut c = digimon("EGG", "Egg", CardColor::Purple, 2, 0, &["DATA SQUAD"]);
            c.card_kind = CardKind::DigiEgg;
            c
        })
        .add_card(digimon(
            "AVIAN5",
            "Peckmon",
            CardColor::Purple,
            4,
            5,
            &["Avian"],
        ))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    tamer_with_face_down(&mut r, 0, "TAMER", 1);
    push_trash(&mut r, 0, "EGG");
    push_trash(&mut r, 0, "AVIAN5");
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    r.accept_optional_trigger().expect("optional clause");
    pick_first(&mut r, 0); // Tamer
    // The Egg is pushed first, so `pick_first` lands on it if it is offered.
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 0).contains(&"AVIAN5".to_string()),
        "the Digi-Egg must not be a candidate; the Avian is the first offered"
    );
    assert!(!field_ids(&r, 0).contains(&"EGG".to_string()));
}

#[test]
fn bt26_005_no_prompt_without_face_down_source() {
    let mut r = setup();
    r.place_on_field(0, "TAMER", Some(0));
    push_trash(&mut r, 0, "AVIAN5");
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    let _ = r.auto_resolve();
    assert!(r.game.pending_selection.is_none());
    assert!(!field_ids(&r, 0).contains(&"AVIAN5".to_string()));
}

/// Official Q&A Q6958 (2026-08-18): "Does this card's inherited effect allow me
/// to play the card under the Tamer after trashing it?" -- "Yes, you can play
/// it." The bottom face-down card Pinamon's cost trashes from under Keenan
/// Crier (BT26-094) is Yoshino Fujieda (BT26-091, a cost-3 [DATA SQUAD]
/// Tamer); it is picked from the trash and played. Keenan's "effects trash
/// cards from under this Tamer" trigger waits for Pinamon's effect to finish
/// (§15-8-3-2), so it is PENDING while the play runs, and is offered after.
#[test]
fn bt26_005_q6958_plays_the_card_its_cost_just_trashed_from_under_a_tamer() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-005")
        .dsl_card("BT26-094")
        .expect("Keenan")
        .dsl_card("BT26-091")
        .expect("Yoshino")
        .add_card(filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(3)
        .start();
    r.set_first_player(0);
    // Keenan with two face-down cards; BT26-091 is the BOTTOM one.
    let keenan = r.place_stack(0, &["BT26-091", "FILLER", "BT26-094"]);
    {
        let stack = &mut r.game.players[0].battle_area[keenan.index as usize].card_sources;
        stack[0].face_down = true;
        stack[1].face_down = true;
    }
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    r.accept_optional_trigger().expect("Pinamon's optional clause");
    pick_first(&mut r, 0); // Keenan: the only Tamer with a face-down card
    assert!(
        trash_ids(&r, 0).contains(&"BT26-091".to_string()),
        "the bottom face-down card is trashed"
    );
    pick_trash(&mut r, 0, "BT26-091");
    assert!(
        field_ids(&r, 0).contains(&"BT26-091".to_string()),
        "Q6958: the card the cost just trashed is played (field {:?}, trash {:?})",
        field_ids(&r, 0),
        trash_ids(&r, 0)
    );
    assert!(!trash_ids(&r, 0).contains(&"BT26-091".to_string()));
    // Keenan's source-trash trigger resolves after the play (§15-8-3-2).
    let v = r
        .pending_selection_view()
        .expect("Keenan's pending trigger is offered after Pinamon's effect");
    assert!(v.is_optional, "Keenan's trigger is a 'by suspending' optional");
    r.accept_optional_trigger().expect("Keenan's trigger");
    let _ = r.auto_resolve();
    let k = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT26-094")
        .expect("Keenan still on the field");
    assert!(k.is_suspended, "Keenan's trigger resolved: he paid by suspending");
}
