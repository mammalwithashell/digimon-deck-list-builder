//! Source: `data/card_bundles/BT27-101.md` (printed text and official Q&A).
//!
//! When this card would be used, by trashing the bottom face-down card from
//! under any of your Tamers, reduce the cost by 2.
//! [Main] By placing 1 [Peckmon] and 1 [Crowmon] from your trash as 1 of your
//! [Falcomon]'s bottom digivolution cards, that Digimon may digivolve into
//! [Ravemon] in the trash, ignoring digivolution requirements and without
//! paying the cost.
//! [Security] You may play 1 [Falcomon] or [Keenan Crier] from your hand or
//! trash without paying the cost. Then, add this card to the hand.
//! Q&A: the "by" condition needs both a [Peckmon] and a [Crowmon] placed.

use digimon_engine::debug_runner::DebugRunner;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-101";

pub fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(purple(mon("FALCO", "Falcomon", 3, 3, &[])))
        .add_card(mon("PECK", "Peckmon", 4, 5, &[]))
        .add_card(mon("CROW", "Crowmon", 5, 7, &[]))
        .add_card(mon("RAVE", "Ravemon", 6, 12, &[]))
        .add_card(tamer("KEENAN", "Keenan Crier", &["DATA SQUAD"]))
        .add_card(mon("ATK", "Atk", 5, 7, &[]))
        .add_card(mon("FD", "Fd", 3, 3, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .memory(10)
        .start()
}

fn use_card(r: &mut DebugRunner) {
    let i = hand_index(r, 0, CARD);
    let _ = r.game.play_option_from_hand(0, i);
}

fn stage(r: &mut DebugRunner) -> digimon_engine::permanent::PermanentHandle {
    let f = r.place_on_field(0, "FALCO", Some(0));
    for id in ["PECK", "CROW", "RAVE"] {
        r.inject_trash(0, id);
    }
    f
}

#[test]
fn main_places_peckmon_and_crowmon_then_falcomon_digivolves_into_ravemon_from_trash_free() {
    let mut r = runner(&[CARD]);
    let f = stage(&mut r);
    use_card(&mut r);
    let mem = r.memory();
    assert_eq!(candidates(&r), vec!["FALCO"]);
    r.choose_prompt_card("FALCO", None).unwrap();
    r.choose_prompt_card("PECK", None).unwrap();
    r.choose_prompt_card("CROW", None).unwrap();
    let v = r.pending_selection_view().expect("optional Ravemon pick");
    assert!(v.is_optional);
    r.choose_prompt_card("RAVE", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, f), "RAVE", "requirements ignored");
    let s = source_ids(&r, f);
    assert!(s.contains(&"PECK".to_string()) && s.contains(&"CROW".to_string()));
    assert_eq!(r.memory(), mem, "digivolve was free");
}

#[test]
fn main_is_all_or_nothing() {
    let mut r = runner(&[CARD]);
    let f = stage(&mut r);
    use_card(&mut r);
    r.choose_prompt_card("FALCO", None).unwrap();
    r.choose_prompt_card("PECK", None).unwrap();
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert!(source_ids(&r, f).is_empty(), "Q&A: only one placed means none");
    assert_eq!(top_id(&r, f), "FALCO");
}

#[test]
fn digivolving_is_optional_after_placing() {
    let mut r = runner(&[CARD]);
    let f = stage(&mut r);
    use_card(&mut r);
    r.choose_prompt_card("FALCO", None).unwrap();
    r.choose_prompt_card("PECK", None).unwrap();
    r.choose_prompt_card("CROW", None).unwrap();
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(top_id(&r, f), "FALCO");
    assert_eq!(source_ids(&r, f).len(), 2);
}

#[test]
fn security_plays_falcomon_or_keenan_from_hand_or_trash_then_adds_itself_to_hand() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(purple(mon("FALCO", "Falcomon", 3, 3, &[])))
        .add_card(tamer("KEENAN", "Keenan Crier", &["DATA SQUAD"]))
        .add_card(mon("ATK", "Atk", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(1, &["FALCO", "FILL"])
        .security(1, &[CARD])
        .start();
    r.inject_trash(1, "KEENAN");
    let a = r.place_on_field(0, "ATK", Some(0)).index;
    attack_security(&mut r, a);
    let v = r.pending_selection_view().expect("optional play");
    assert_eq!(v.selecting_player, 1);
    assert!(v.is_optional);
    let mut c = candidates(&r);
    c.sort();
    assert_eq!(c, vec!["FALCO", "KEENAN"]);
    r.choose_prompt_card("KEENAN", None).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).contains(&"KEENAN".to_string()));
    assert!(hand_ids(&r, 1).contains(&CARD.to_string()));
}

#[test]
fn security_play_may_be_declined_and_still_adds_itself_to_hand() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(purple(mon("FALCO", "Falcomon", 3, 3, &[])))
        .add_card(mon("ATK", "Atk", 5, 7, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(1, &["FALCO"])
        .security(1, &[CARD])
        .start();
    let a = r.place_on_field(0, "ATK", Some(0)).index;
    attack_security(&mut r, a);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 1).is_empty());
    assert!(hand_ids(&r, 1).contains(&CARD.to_string()));
}

fn finish(r: &mut DebugRunner) {
    while let Some(v) = r.pending_selection_view() {
        if v.is_optional {
            r.decline_prompt().unwrap();
        } else {
            r.execute_action(v.selecting_player, v.valid_action_ids[0]).unwrap();
        }
    }
}

#[test]
fn trashing_a_face_down_tamer_card_reduces_the_use_cost_by_two() {
    let mut r = runner(&[CARD]);
    let t = stocked(&mut r, "KEENAN", &["FD"]);
    r.place_on_field(0, "FALCO", Some(0));
    let mem = r.memory();
    use_card(&mut r);
    let v = r.pending_selection_view().expect("optional reduction");
    assert!(v.is_optional);
    r.execute_branch(0).unwrap();
    finish(&mut r);
    assert!(source_ids(&r, t).is_empty());
    assert_eq!(r.memory(), mem - 3);
}

#[test]
fn reduction_may_be_declined_and_needs_a_face_down_card() {
    let mut r = runner(&[CARD]);
    let t = stocked(&mut r, "KEENAN", &["FD"]);
    r.place_on_field(0, "FALCO", Some(0));
    let mem = r.memory();
    use_card(&mut r);
    r.decline_prompt().unwrap();
    finish(&mut r);
    assert_eq!(source_ids(&r, t).len(), 1);
    assert_eq!(r.memory(), mem - 5);

    let mut r = runner(&[CARD]);
    stocked(&mut r, "KEENAN", &[]);
    r.place_on_field(0, "FALCO", Some(0));
    let mem = r.memory();
    use_card(&mut r);
    finish(&mut r);
    assert_eq!(r.memory(), mem - 5, "no offer without a face-down card");
}
