//! Source: `data/card_bundles/BT27-074.md` (printed text and official Q&A).
//!
//! [When Moving] [On Play] By placing 1 card from your hand face down under any
//! of your [Keenan Crier]s, <Draw 1> and trash 1 card in your hand.
//! Inherited: [When Attacking] [Once Per Turn] <Draw 1> and trash 1 card in
//! your hand. Q&A: the card is placed on the bottom of the cards under the Tamer.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-074";
const KEENAN: &str = "KEENAN";

fn runner(hand: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .add_card(tamer(KEENAN, "Keenan Crier", &["DATA SQUAD"]))
        .add_card(tamer("OTHER-T", "Thomas H. Norstein", &["DATA SQUAD"]))
        .add_card(mon("X", "X", 3, 3, &[]))
        .add_card(mon("Y", "Y", 3, 3, &[]))
        .add_card(mon("OLD", "Old", 3, 3, &[]))
        .add_card(mon("TOP", "Top", 5, 7, &[]))
        .add_card(mon("DRAW", "Draw", 3, 3, &[]))
        .deck(0, &["DRAW"; 10])
        .deck(1, &["DRAW"; 10])
        .security(1, &["DRAW"; 5])
        .hand(0, hand)
        .memory(10)
        .start()
}

fn keenan(r: &mut DebugRunner) -> PermanentHandle {
    let t = r.place_stack(0, &["OLD", KEENAN]);
    r.game.players[0].battle_area[t.index as usize].card_sources[0].face_down = true;
    t
}

#[test]
fn on_play_places_face_down_at_bottom_of_keenan_then_draws_and_trashes_a_chosen_card() {
    let mut r = runner(&[CARD, "X", "Y"]);
    let t = keenan(&mut r);
    play_hand(&mut r, CARD);
    let v = r.pending_selection_view().expect("optional hand pick");
    assert!(v.is_optional);
    assert_eq!(candidates(&r), vec!["X", "Y"]);
    r.choose_prompt_card("X", None).unwrap();
    assert_eq!(candidates(&r), vec![KEENAN], "Tamer destination pick");
    r.choose_prompt_card(KEENAN, None).unwrap();
    let v = r.pending_selection_view().expect("mandatory trash");
    assert!(!v.is_optional);
    assert_eq!(candidates(&r), vec!["DRAW", "Y"]);
    r.choose_prompt_card("Y", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t), vec!["X", "OLD"]);
    assert_eq!(face_down(&r, t), vec![true, true]);
    assert_eq!(hand_ids(&r, 0), vec!["DRAW"]);
    assert_eq!(trash_ids(&r, 0), vec!["Y"]);
}

#[test]
fn on_play_decline_draws_and_trashes_nothing() {
    let mut r = runner(&[CARD, "X"]);
    let t = keenan(&mut r);
    play_hand(&mut r, CARD);
    r.decline_prompt().unwrap();
    assert!(r.pending_selection().is_none());
    assert_eq!(source_ids(&r, t), vec!["OLD"]);
    assert_eq!(hand_ids(&r, 0), vec!["X"]);
}

#[test]
fn on_play_needs_a_keenan_crier() {
    let mut r = runner(&[CARD, "X"]);
    r.place_on_field(0, "OTHER-T", Some(0));
    play_hand(&mut r, CARD);
    assert!(r.pending_selection().is_none());
    assert_eq!(hand_ids(&r, 0), vec!["X"]);
}

#[test]
fn when_moving_from_breeding_offers_the_same_placement() {
    let mut r = runner(&["X"]);
    let t = keenan(&mut r);
    r.place_in_breeding(0, CARD);
    r.set_phase(GamePhase::Breeding);
    assert!(r.move_from_breeding(0));
    r.choose_prompt_card("X", None).unwrap();
    r.choose_prompt_card(KEENAN, None).unwrap();
    r.choose_prompt_card("DRAW", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(source_ids(&r, t), vec!["X", "OLD"]);
    assert_eq!(trash_ids(&r, 0), vec!["DRAW"]);
}

#[test]
fn inherited_draws_then_trashes_once_per_turn() {
    let mut r = runner(&["X"]);
    let a = r.place_stack(0, &[CARD, "TOP"]).index;
    attack_security(&mut r, a);
    let v = r.pending_selection_view().expect("mandatory trash");
    assert!(!v.is_optional);
    assert_eq!(candidates(&r), vec!["DRAW", "X"]);
    r.choose_prompt_card("X", None).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DRAW"]);
    attack_security(&mut r, a);
    let _ = r.auto_resolve();
    assert_eq!(hand_ids(&r, 0), vec!["DRAW"], "once per turn");
}
