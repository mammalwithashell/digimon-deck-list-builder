//! BT26-074 Cerberusmon — Lv.5 Purple/Black, Dark Animal/Titan/TS.
//!
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] If it's your
//! turn, by trashing 1 card in your hand, you may use 1 Option card with the
//! [Titan] trait from your trash with the cost reduced by 2.
//! Inherited: [On Deletion] Delete 1 of your opponent's Digimon with the
//! lowest level.
//!
//! DCGO: BT26/Purple/BT26_074.cs.

use super::support::*;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, EffectTiming};

const CARD_ID: &str = "BT26-074";

fn setup() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT26-074")
        .from_dsl_yaml(TITAN_OPTION_YAML)
        .expect("titan option")
        .add_card(filler("FILLER"))
        .add_card(digimon("OPP3", "Opp3", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP3B", "Opp3b", CardColor::Red, 3, 3, &[]))
        .add_card(digimon("OPP5", "Opp5", CardColor::Red, 5, 7, &[]))
        .deck(0, &["FILLER"; 6])
        .deck(1, &["FILLER"; 6])
        .memory(5)
        .start();
    r.set_first_player(0);
    r
}

#[test]
fn bt26_074_trash_hand_then_use_titan_option_from_trash_at_minus_two() {
    let mut r = setup();
    let cerb = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "T-TITANOPT");
    push_hand(&mut r, 0, "FILLER");
    let deck_before = r.deck_size(0);
    fire(&mut r, EffectTiming::OnPlay, cerb);
    let v = r.pending_selection_view().expect("optional hand cost");
    assert!(v.is_optional);
    pick_hand(&mut r, 0, "FILLER");
    let v = r.pending_selection_view().expect("optional option pick");
    assert!(v.is_optional);
    pick_trash(&mut r, 0, "T-TITANOPT");
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 5 - (4 - 2), "use cost 4 reduced by 2");
    assert_eq!(r.deck_size(0), deck_before - 1, "the Option's [Main] ran");
}

#[test]
fn bt26_074_declined_cost_refunds_once_per_turn() {
    let mut r = setup();
    let cerb = r.place_on_field(0, CARD_ID, Some(0));
    push_trash(&mut r, 0, "T-TITANOPT");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenAttacking, cerb);
    pass(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(r.hand_size(0), 1);
    fire(&mut r, EffectTiming::WhenAttacking, cerb);
    assert!(
        r.pending_selection_view().is_some(),
        "use refunded ⇒ offered again"
    );
}

#[test]
fn bt26_074_paid_cost_spends_once_per_turn() {
    let mut r = setup();
    let cerb = r.place_on_field(0, CARD_ID, Some(0));
    push_hand(&mut r, 0, "FILLER");
    push_hand(&mut r, 0, "FILLER");
    fire(&mut r, EffectTiming::WhenDigivolving, cerb);
    pick_hand(&mut r, 0, "FILLER");
    let _ = r.auto_resolve(); // no Option in trash: nothing more
    assert_eq!(r.hand_size(0), 1);
    fire(&mut r, EffectTiming::WhenDigivolving, cerb);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.hand_size(0), 1);
}

#[test]
fn bt26_074_not_on_opponents_turn() {
    let mut r = setup();
    let cerb = r.place_on_field(1, CARD_ID, Some(0));
    push_hand(&mut r, 1, "FILLER");
    fire(&mut r, EffectTiming::OnPlay, cerb);
    let _ = r.auto_resolve();
    assert!(r.pending_selection_view().is_none());
    assert_eq!(r.hand_size(1), 1);
}

#[test]
fn bt26_074_inherited_on_deletion_deletes_lowest_level() {
    let mut r = setup();
    let carrier = r.place_stack(0, &[CARD_ID, "FILLER"]);
    r.place_on_field(1, "OPP3", Some(0));
    r.place_on_field(1, "OPP3B", Some(0));
    r.place_on_field(1, "OPP5", Some(0));
    fire(&mut r, EffectTiming::OnDeletion, carrier);
    let v = r.pending_selection_view().expect("delete pick");
    assert!(!v.is_optional);
    assert_eq!(non_pass(&r).len(), 2, "only the two level-3s");
    pick_first(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1).len(), 2);
    assert!(field_ids(&r, 1).contains(&"OPP5".to_string()));
}
