//! Source: `data/card_bundles/BT27-028.md` (printed text).
//!
//! <Decode (Lv.4 or lower w/[Aquatic]/[DATA SQUAD] trait)> [On Play] [When
//! Digivolving] 1 of your opponent's Digimon or Tamers can't suspend until
//! their turn ends. (Rule) Trait: Has [Aquatic] Type.
//! Inherited: <Decode (Lv.4 or lower w/[Aquatic]/[DATA SQUAD] trait)>.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-028";

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("AQ4", "Aq", 4, 5, &["Aquatic"]))
        .add_card(mon("DS3", "Ds", 3, 3, &["DATA SQUAD"]))
        .add_card(mon("DS5", "Ds5", 5, 7, &["DATA SQUAD"]))
        .add_card(mon("PLAIN4", "Plain", 4, 5, &[]))
        .add_card(tamer("OPP-T", "Opp Tamer", &[]))
        .add_card(mon("TOP", "Top", 6, 9, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, &[CARD])
        .memory(10)
        .start()
}

#[test]
fn rule_trait_has_aquatic() {
    let r = runner();
    assert!(r.compiled_card(CARD).unwrap().traits.iter().any(|t| t == "Aquatic"));
}

#[test]
fn on_play_locks_a_chosen_opposing_digimon_or_tamer() {
    for pick in ["TOP", "OPP-T"] {
        let mut r = runner();
        let d = r.place_on_field(1, "TOP", Some(0));
        let t = r.place_on_field(1, "OPP-T", Some(0));
        play_hand(&mut r, CARD);
        let v = r.pending_selection_view().expect("lock pick");
        assert!(!v.is_optional);
        assert_eq!(candidates(&r), vec!["OPP-T", "TOP"]);
        r.choose_prompt_card(pick, None).unwrap();
        let _ = r.auto_resolve();
        let locked = if pick == "TOP" { d } else { t };
        let other = if pick == "TOP" { t } else { d };
        assert!(r.game.modifiers.has(locked, ModifierType::CannotSuspend));
        assert!(!r.game.modifiers.has(other, ModifierType::CannotSuspend));
    }
}

fn bounce(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    let src = r.place_on_field(1, "FILL", Some(0));
    opponent_bounces(r, src, h);
}

fn accept_decode(r: &mut DebugRunner) {
    let v = r.pending_selection_view().expect("optional Decode prompt");
    assert_eq!(v.kind, digimon_engine::selection::SelectionKind::Replacement);
    assert!(v.is_optional);
    r.execute_action(0, v.valid_action_ids[0]).unwrap();
}

#[test]
fn decode_may_be_declined_and_the_digimon_leaves() {
    let mut r = runner();
    let h = r.place_stack(0, &["DS3", CARD]);
    bounce(&mut r, h);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
}

#[test]
fn decode_offers_only_matching_sources_and_plays_one_free() {
    let mut r = runner();
    let h = r.place_stack(0, &["AQ4", "DS3", "DS5", "PLAIN4", CARD]);
    let before = r.memory();
    bounce(&mut r, h);
    accept_decode(&mut r);
    assert_eq!(candidates(&r), vec!["AQ4", "DS3"]);
    r.choose_prompt_card("DS3", None).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"DS3".to_string()));
    assert_eq!(r.memory(), before);
}

#[test]
fn inherited_decode_works_from_digivolution_cards() {
    let mut r = runner();
    let h = r.place_stack(0, &["DS3", CARD, "TOP"]);
    bounce(&mut r, h);
    accept_decode(&mut r);
    assert_eq!(candidates(&r), vec!["DS3"]);
    r.choose_prompt_card("DS3", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 0), vec!["DS3"]);
}

#[test]
fn decode_does_not_apply_when_deleted_in_battle() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(mon("DS3", "Ds", 3, 3, &["DATA SQUAD"]))
        .add_card(wall())
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start();
    let h = r.place_stack(0, &["DS3", CARD]);
    lose_battle(&mut r, h.index);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert!(!field_ids(&r, 0).contains(&"DS3".to_string()));
}
