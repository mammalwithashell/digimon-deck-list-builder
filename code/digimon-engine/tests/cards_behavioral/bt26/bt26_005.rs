//! BT26-005 Pinamon — Lv.2 Purple Digi-Egg, Bird/DATA SQUAD.
//!
//! Inherited: [On Deletion] By trashing the bottom face-down card from under
//! any of your Tamers, you may play 1 play cost 5 or lower [Avian] or
//! [DATA SQUAD] trait card from your trash without paying the cost.
//!
//! DCGO: BT26/Purple/BT26_005.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-005";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-005")
        .add_card(filler("FILLER"))
        .add_card(tamer("TAMER", "Tamer", CardColor::Purple, &["DATA SQUAD"]))
        .add_card(digimon("AVIAN5", "Peckmon", CardColor::Purple, 4, 5, &["Avian"]))
        .add_card(digimon("DS6", "Big", CardColor::Purple, 5, 6, &["DATA SQUAD"]))
        .add_card(digimon("PLAIN", "Plain", CardColor::Purple, 3, 3, &["Beast"]))
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
