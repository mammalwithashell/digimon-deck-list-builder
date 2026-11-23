//! Source: `data/card_bundles/BT27-020.md` (printed text and official Q&A).
//!
//! Effect: [On Play] By placing 1 level 4 or lower [Aquatic] or [DATA SQUAD]
//! trait Digimon card from your hand as any of your Digimon's bottom
//! digivolution card, or face down under any of your [DATA SQUAD] trait
//! Tamers, <Draw 1>. (Rule) Trait: Has [Aquatic] Type.
//! Inherited Effect: [When Attacking] [Once Per Turn] Trash the top or bottom
//! digivolution card of 1 of your opponent's Digimon.
//! Official Q&A: the card is placed on the bottom of the cards under the Tamer.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-020";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(tamer("DS-T", "Squad Tamer", &["DATA SQUAD"]))
        .add_card(tamer("PLAIN-T", "Plain Tamer", &[]))
        .add_card(mon("AQ4", "Aq", 4, 5, &["Aquatic"]))
        .add_card(mon("DS3", "Ds", 3, 3, &["DATA SQUAD"]))
        .add_card(mon("DS5", "Ds5", 5, 7, &["DATA SQUAD"]))
        .add_card(mon("PLAIN4", "Plain", 4, 5, &["Beast"]))
        .add_card(tamer("DS-T-CARD", "Squad Tamer Card", &["DATA SQUAD"]))
        .add_card(mon("HOST", "Host", 4, 5, &[]))
        .add_card(mon("A", "A", 3, 3, &[]))
        .add_card(mon("B", "B", 4, 5, &[]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .hand(0, hand)
        .memory(10)
        .start()
}

#[test]
fn rule_trait_has_aquatic() {
    let r = runner(&[]);
    let card = r.compiled_card(CARD).unwrap();
    assert!(card.traits.iter().any(|t| t == "Aquatic"));
}

#[test]
fn on_play_offers_only_level_four_or_lower_aquatic_or_data_squad_digimon() {
    let mut r = runner(&[CARD, "AQ4", "DS3", "DS5", "PLAIN4", "DS-T-CARD"]);
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["AQ4", "DS3"]);
}

#[test]
fn on_play_places_as_bottom_digivolution_card_of_a_digimon_and_draws() {
    let mut r = runner(&[CARD, "AQ4"]);
    let host = r.place_stack(0, &["A", "HOST"]);
    play_hand(&mut r, CARD);
    r.choose_prompt_card("AQ4", None).unwrap();
    // No [DATA SQUAD] Tamer: the destination choice collapses to "a Digimon".
    let v = r.pending_selection_view().expect("Digimon pick");
    assert!(!v.is_optional);
    assert_eq!(candidates(&r), vec!["BT27-020", "HOST"]);
    r.choose_prompt_card("HOST", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, host), vec!["AQ4", "A"]);
    assert_eq!(face_down(&r, host), vec![false, false]);
    assert_eq!(hand_ids(&r, 0), vec!["FILL"]);
}

#[test]
fn on_play_places_face_down_at_bottom_of_a_data_squad_tamer_and_draws() {
    let mut r = runner(&[CARD, "DS3"]);
    let t = r.place_stack(0, &["A", "DS-T"]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    let plain = r.place_on_field(0, "PLAIN-T", Some(0));
    play_hand(&mut r, CARD);
    r.choose_prompt_card("DS3", None).unwrap();
    let _ = plain;
    choose_branch(&mut r, 1);
    if r.pending_selection().is_some() {
        assert_eq!(candidates(&r), vec!["DS-T"], "non-DATA SQUAD Tamer excluded");
        r.choose_prompt_card("DS-T", None).unwrap();
    }
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t), vec!["DS3", "A"]);
    assert_eq!(face_down(&r, t), vec![true, true]);
    assert_eq!(hand_ids(&r, 0), vec!["FILL"]);
}

#[test]
fn on_play_destination_choice_offers_digimon_or_data_squad_tamer() {
    let mut r = runner(&[CARD, "DS3"]);
    let t = r.place_on_field(0, "DS-T", Some(0));
    let host = r.place_stack(0, &["A", "HOST"]);
    play_hand(&mut r, CARD);
    r.choose_prompt_card("DS3", None).unwrap();
    let v = r.pending_selection_view().expect("destination choice");
    assert_eq!(v.effect_choices.as_ref().map(|c| c.len()), Some(2), "{v:?}");
    choose_branch(&mut r, 0);
    r.choose_prompt_card("HOST", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, host), vec!["DS3", "A"]);
    assert!(source_ids(&r, t).is_empty());
}

#[test]
fn on_play_decline_draws_nothing() {
    let mut r = runner(&[CARD, "AQ4"]);
    play_hand(&mut r, CARD);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert_eq!(hand_ids(&r, 0), vec!["AQ4"]);
}

#[test]
fn on_play_without_eligible_hand_card_does_nothing() {
    let mut r = runner(&[CARD, "DS5", "PLAIN4"]);
    play_hand(&mut r, CARD);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.hand_size(0), 2);
}

fn attack_setup(r: &mut DebugRunner) -> (u8, PermanentHandle) {
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    let victim = r.place_stack(1, &["A", "B", "TOP"]);
    (a, victim)
}

#[test]
fn inherited_trashes_top_digivolution_card_when_top_is_chosen() {
    let mut r = runner(&[]);
    let (a, victim) = attack_setup(&mut r);
    attack_security(&mut r, a);
    r.choose_prompt_card("TOP", None).unwrap();
    choose_branch(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, victim), vec!["A"]);
    assert_eq!(top_id(&r, victim), "TOP");
    assert!(trash_ids(&r, 1).contains(&"B".to_string()), "trash {:?}", trash_ids(&r, 1));
}

#[test]
fn inherited_trashes_bottom_digivolution_card_when_bottom_is_chosen() {
    let mut r = runner(&[]);
    let (a, victim) = attack_setup(&mut r);
    attack_security(&mut r, a);
    r.choose_prompt_card("TOP", None).unwrap();
    choose_branch(&mut r, 1);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, victim), vec!["B"]);
    assert!(trash_ids(&r, 1).contains(&"A".to_string()));
}

#[test]
fn inherited_ignores_opponent_digimon_without_digivolution_cards() {
    let mut r = runner(&[]);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    r.place_on_field(1, "TOP", Some(0));
    attack_security(&mut r, a);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    let victim = r.place_stack(1, &["A", "B", "TOP"]);
    attack_security(&mut r, a);
    assert!(r.pending_selection().is_some(), "OPT not spent without a target");
    assert_eq!(candidates(&r), vec!["TOP"]);
    r.choose_prompt_card("TOP", None).unwrap();
    choose_branch(&mut r, 0);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, victim), vec!["A"]);
}

#[test]
fn inherited_target_pick_is_mandatory_and_once_per_turn() {
    let mut r = runner(&[]);
    let (a, victim) = attack_setup(&mut r);
    attack_security(&mut r, a);
    let v = r.pending_selection_view().expect("opponent Digimon pick");
    assert!(!v.is_optional);
    r.choose_prompt_card("TOP", None).unwrap();
    choose_branch(&mut r, 0);
    let _ = r.auto_resolve();
    let other = r.place_stack(1, &["A", "TOP"]);
    attack_security(&mut r, a);
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, other), vec!["A"], "once per turn");
}
