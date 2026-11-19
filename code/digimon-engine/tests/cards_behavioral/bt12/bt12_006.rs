//! BT12-006 Monimon (Digi-Egg, Lv.2, Purple): inherited
//! [On Deletion] If this Digimon has <Save> in its text, <Draw 1>.
//!
//! DCGO BT12/Purple/BT12_006.cs: OnDestroyedAnyone inherited, mandatory;
//! CanActivate = library >= 1 AND the deleted carrier's TopCard.HasSaveText
//! (captured in the deletion event, so it is the pre-removal top card).
//!
//! Synthetic boards only; no recorded-game data.
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;

const ID: &str = "BT12-006";

fn digimon(id: &str, name: &str, text: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.effect_text = text.to_string();
    c
}

fn base(deck: usize) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT12-006 loads from the embedded pack")
        .add_card(digimon(
            "SAVE-HOST",
            "Save Host",
            "[On Deletion] ＜Save＞ (You may place this card under one of your Tamers.)",
        ))
        .add_card(digimon("MSAVE-HOST", "Material Host", "＜Material Save 2＞"))
        .add_card(digimon("PLAIN-HOST", "Plain Host", ""))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &vec!["FILLER"; deck])
        .deck(1, &["FILLER"; 10])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}

fn delete(r: &mut DebugRunner, h: PermanentHandle) {
    let _ = r
        .game
        .delete_permanents_batch(vec![h], ReplacementCause::OpponentEffect);
    r.game.drain_effect_queue();
}

#[test]
fn bt12_006_metadata_and_single_inherited_clause() {
    let r = base(5);
    let d = r.game.card_data.iter().find(|d| d.card_id == ID).unwrap();
    assert_eq!(d.card_name, "Monimon");
    assert_eq!(d.card_kind, CardKind::DigiEgg);
    assert_eq!(d.level, Some(2));
    assert!(d.traits.contains(&"CRT".to_string()));
    assert_eq!(r.compiled_card(ID).unwrap().effects.len(), 1);
}

#[test]
fn bt12_006_draws_one_when_deleted_carrier_has_save_text() {
    let mut r = base(5);
    let h = r.place_stack(0, &[ID, "SAVE-HOST"]);
    let hand = r.hand_size(0);
    let deck = r.deck_size(0);
    delete(&mut r, h);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), hand + 1, "<Draw 1>");
    assert_eq!(r.deck_size(0), deck - 1);
}

#[test]
fn bt12_006_draw_is_mandatory_and_orders_against_the_carriers_own_save() {
    // The Save-text carrier's own <Save> and Monimon's inherited draw trigger
    // together (DCGO's MultipleSkills): the owner orders them, and the draw is
    // a MANDATORY entry -- there is no optional confirm for it.
    let mut r = base(5);
    let h = r.place_stack(0, &[ID, "SAVE-HOST"]);
    let hand = r.hand_size(0);
    delete(&mut r, h);
    let v = r.pending_selection_view().expect("trigger-order prompt");
    let choices = v.effect_choices.expect("effect choices");
    let draw = choices
        .iter()
        .find(|c| c.label.contains(ID))
        .expect("Monimon's draw is one of the simultaneous triggers");
    assert!(!draw.is_optional, "the draw is mandatory");
    r.execute_action(0, draw.action_id).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), hand + 1, "<Draw 1> resolved");
}

#[test]
fn bt12_006_no_save_text_no_draw() {
    let mut r = base(5);
    let h = r.place_stack(0, &[ID, "PLAIN-HOST"]);
    let hand = r.hand_size(0);
    delete(&mut r, h);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), hand);
}

#[test]
fn bt12_006_material_save_is_not_save() {
    let mut r = base(5);
    let h = r.place_stack(0, &[ID, "MSAVE-HOST"]);
    let hand = r.hand_size(0);
    delete(&mut r, h);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), hand);
}

#[test]
fn bt12_006_empty_deck_does_not_draw() {
    // DCGO CanActivateCondition: LibraryCards.Count >= 1.
    let mut r = base(0);
    r.game.players[0].deck.clear();
    let h = r.place_stack(0, &[ID, "SAVE-HOST"]);
    delete(&mut r, h);
    r.auto_resolve().unwrap();
    assert!(!r.game_over(), "no draw from an empty deck, so no deck-out");
}

#[test]
fn bt12_006_reads_the_deleted_carrier_not_a_shifted_slot() {
    // A Save-text Digimon sits in a later slot. When the plain carrier at
    // slot 0 is deleted the field compacts and that Digimon shifts into index
    // 0: the gate must read the deleted carrier's last-known top card, never
    // the new occupant of the slot.
    let mut r = base(5);
    let h = r.place_stack(0, &[ID, "PLAIN-HOST"]);
    let _bystander = r.place_on_field(0, "SAVE-HOST", Some(0));
    let hand = r.hand_size(0);
    delete(&mut r, h);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.hand_size(0),
        hand,
        "bystander's <Save> text must not satisfy the gate"
    );
}

#[test]
fn bt12_006_only_fires_for_its_own_carrier() {
    let mut r = base(5);
    let _carrier = r.place_stack(0, &[ID, "SAVE-HOST"]);
    let other = r.place_on_field(0, "SAVE-HOST", Some(0));
    let hand = r.hand_size(0);
    delete(&mut r, other);
    r.auto_resolve().unwrap();
    assert_eq!(
        r.hand_size(0),
        hand,
        "another Digimon's deletion is not the carrier's [On Deletion]"
    );
}

#[test]
fn bt12_006_two_copies_draw_two() {
    let mut r = base(5);
    let h = r.place_stack(0, &[ID, ID, "SAVE-HOST"]);
    let hand = r.hand_size(0);
    delete(&mut r, h);
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), hand + 2);
}
