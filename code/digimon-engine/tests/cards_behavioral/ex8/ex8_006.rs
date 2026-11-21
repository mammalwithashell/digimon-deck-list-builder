//! EX8-006 DemiMeramon (Digi-Egg, Lv.2, Purple): inherited
//! [When Attacking] [Once Per Turn] If this Digimon has the [NSo] trait, by
//! trashing 1 card in your hand, delete 1 of your opponent's level 3 Digimon.
//!
//! DCGO EX8/Purple/EX8_006.cs: OnAllyAttack inherited, maxCount 1,
//! isOptional TRUE; CanActivate = carrier TopCard has the NSo trait AND hand
//! >= 1; hand pick canNoSelect:true; mandatory Lv.3 delete after a discard.
//!
//! Synthetic boards only; no recorded-game data.
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const ID: &str = "EX8-006";

fn mon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn base(hand: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX8-006 loads from the embedded pack")
        .add_card(mon("NSO-HOST", 4, &["NSo"]))
        .add_card(mon("PLAIN-HOST", 4, &[]))
        .add_card(mon("OPP-3", 3, &[]))
        .add_card(mon("OPP-4", 4, &[]))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(0, &["FILLER"; 10])
        .deck(1, &["FILLER"; 10])
        .hand(0, hand)
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r
}

fn attack(r: &mut DebugRunner, h: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

fn on_field(r: &DebugRunner, p: u8, id: &str) -> bool {
    r.game.players[p as usize]
        .battle_area
        .iter()
        .any(|x| x.top_card().card_id(&r.game.card_data) == id)
}

#[test]
fn ex8_006_metadata() {
    let r = base(&[]);
    let d = r.game.card_data.iter().find(|d| d.card_id == ID).unwrap();
    assert_eq!(d.card_name, "DemiMeramon");
    assert_eq!(d.card_kind, CardKind::DigiEgg);
    assert_eq!(d.level, Some(2));
    assert!(d.traits.contains(&"NSo".to_string()));
    assert_eq!(r.compiled_card(ID).unwrap().effects.len(), 1);
}

#[test]
fn ex8_006_nso_carrier_trashes_a_card_and_deletes_a_level_3() {
    let mut r = base(&["FILLER"]);
    let h = r.place_stack(0, &[ID, "NSO-HOST"]);
    r.place_on_field(1, "OPP-3", Some(0));
    r.place_on_field(1, "OPP-4", Some(0));
    attack(&mut r, h);
    let v = r.pending_selection_view().expect("optional activation");
    assert!(v.is_optional);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().expect("hand pick");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let v = r.pending_selection_view().expect("target pick");
    assert_eq!(v.valid_action_ids.len(), 1, "only the level 3 is legal");
    assert!(!v.is_optional, "the delete is mandatory");
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 0);
    assert_eq!(r.trash_size(0), 1);
    assert!(!on_field(&r, 1, "OPP-3"));
    assert!(on_field(&r, 1, "OPP-4"));
}

#[test]
fn ex8_006_non_nso_carrier_offers_nothing() {
    let mut r = base(&["FILLER"]);
    let h = r.place_stack(0, &[ID, "PLAIN-HOST"]);
    r.place_on_field(1, "OPP-3", Some(0));
    attack(&mut r, h);
    assert!(r.pending_selection().is_none(), "carrier lacks [NSo]");
    assert!(on_field(&r, 1, "OPP-3"));
}

#[test]
fn ex8_006_empty_hand_offers_nothing() {
    let mut r = base(&[]);
    let h = r.place_stack(0, &[ID, "NSO-HOST"]);
    r.place_on_field(1, "OPP-3", Some(0));
    attack(&mut r, h);
    assert!(r.pending_selection().is_none());
    assert!(on_field(&r, 1, "OPP-3"));
}

#[test]
fn ex8_006_decline_outright_deletes_nothing() {
    let mut r = base(&["FILLER"]);
    let h = r.place_stack(0, &[ID, "NSO-HOST"]);
    r.place_on_field(1, "OPP-3", Some(0));
    attack(&mut r, h);
    r.decline_optional_trigger().unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 1);
    assert!(on_field(&r, 1, "OPP-3"));
}

#[test]
fn ex8_006_accept_then_pick_nothing_deletes_nothing() {
    let mut r = base(&["FILLER"]);
    let h = r.place_stack(0, &[ID, "NSO-HOST"]);
    r.place_on_field(1, "OPP-3", Some(0));
    attack(&mut r, h);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().expect("hand pick");
    assert!(v.is_optional, "canNoSelect: true");
    r.execute_action(0, PASS).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.hand_size(0), 1);
    assert!(on_field(&r, 1, "OPP-3"));
}

#[test]
fn ex8_006_once_per_turn() {
    let mut r = base(&["FILLER", "FILLER"]);
    let h = r.place_stack(0, &[ID, "NSO-HOST"]);
    r.place_on_field(1, "OPP-3", Some(0));
    r.place_on_field(1, "OPP-3", Some(0));
    attack(&mut r, h);
    r.accept_optional_trigger().unwrap();
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    let v = r.pending_selection_view().unwrap();
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[1].battle_area.len(), 1);
    attack(&mut r, h);
    assert!(r.pending_selection().is_none(), "[Once Per Turn]");
}
