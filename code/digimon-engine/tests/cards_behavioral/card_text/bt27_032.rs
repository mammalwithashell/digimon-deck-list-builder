//! Source: `data/card_bundles/BT27-032.md` (printed text and official Q&A).
//!
//! <Reboot> <Blocker> <Evade>
//! [When Digivolving] For every 4 cards your opponent's hand has, 1 of their
//! Digimon or Tamers can't suspend until their turn ends.
//! [All Turns] [Once Per Turn] When effects add to your opponent's hand, or
//! effects trash cards under your Tamers, you may activate 1 [Main] effect on
//! this card's Option side.
//! Option side: <Use Req. ([DATA SQUAD] trait)> [Main] Return 1 of your
//! opponent's highest level Digimon to the hand. <Arts Digivolve>
//! Q&A: the activated [Main] is an Option card effect.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{EffectSourceKind, Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;

use super::bt27_fx::*;
use super::common::candidates;

const CARD: &str = "BT27-032";

fn runner(hand: &[&str], opp_hand: usize) -> DebugRunner {
    let mut black = mon("BLACK5", "Black", 5, 7, &[]);
    black.colors = vec![digimon_engine::enums::CardColor::Black];
    let mut red_gao = mon("GAO", "MachGaogamon", 5, 7, &[]);
    red_gao.colors = vec![digimon_engine::enums::CardColor::Red];
    let mut red_sq = mon("SQ5", "Squad", 5, 7, &["DATA SQUAD"]);
    red_sq.colors = vec![digimon_engine::enums::CardColor::Red];
    let mut red5 = mon("RED5", "Red", 5, 7, &[]);
    red5.colors = vec![digimon_engine::enums::CardColor::Red];
    DebugRunner::builder()
        .dsl_card(CARD)
        .unwrap()
        .dsl_card("BT27-030")
        .unwrap()
        .add_card(mon("BLUE5", "Blue", 5, 7, &[]))
        .add_card(black)
        .add_card(red_gao)
        .add_card(red_sq)
        .add_card(red5)
        .add_card(mon("L4", "Four", 4, 5, &[]))
        .add_card(mon("L6", "Six", 6, 9, &[]))
        .add_card(mon("L6B", "SixB", 6, 9, &[]))
        .add_card(tamer("OT", "OppTamer", &[]))
        .add_card(tamer("T", "Tamer", &["DATA SQUAD"]))
        .add_card(mon("FD", "Fd", 3, 3, &[]))
        .add_card(mon("FILL", "Fill", 3, 3, &[]))
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .hand(0, hand)
        .hand(1, &vec!["FILL"; opp_hand])
        .memory(10)
        .start()
}

fn on_field(r: &mut DebugRunner) -> PermanentHandle {
    r.place_stack(0, &["BLUE5", CARD])
}

fn find(r: &DebugRunner, p: u8, id: &str) -> PermanentHandle {
    let i = field_ids(r, p).iter().position(|x| x == id).expect("on field");
    PermanentHandle { player: p, index: i as u8 }
}

#[test]
fn printed_keywords_are_reboot_blocker_evade() {
    let mut r = runner(&[], 0);
    let h = on_field(&mut r);
    for k in [Keyword::Reboot, Keyword::Blocker, Keyword::Evade] {
        assert!(r.game.has_keyword(h, k), "{k:?}");
    }
}

#[test]
fn digivolves_from_blue_or_black_level_5_for_4_and_gaogamon_or_data_squad_for_3() {
    for (base, cost) in [("BLUE5", 4), ("BLACK5", 4), ("GAO", 3), ("SQ5", 3)] {
        let mut r = runner(&[CARD], 0);
        let b = r.place_on_field(0, base, Some(0));
        let mem = r.memory();
        assert!(digivolve_hand(&mut r, CARD, b), "{base}");
        while r.pending_selection().is_some() {
            r.decline_prompt().unwrap();
        }
        assert_eq!(top_id(&r, b), CARD);
        assert_eq!(mem - r.memory(), cost, "{base}");
    }
    let mut r = runner(&[CARD], 0);
    let b = r.place_on_field(0, "RED5", Some(0));
    assert!(!digivolve_hand(&mut r, CARD, b), "red level 5 without the name or trait");
}

#[test]
fn when_digivolving_locks_one_digimon_or_tamer_per_four_opponent_hand_cards() {
    for (n, picks) in [(3usize, 0usize), (4, 1), (7, 1), (8, 2)] {
        let mut r = runner(&[CARD], n);
        let b = r.place_on_field(0, "BLUE5", Some(0));
        r.place_on_field(1, "L4", Some(0));
        r.place_on_field(1, "OT", None);
        r.place_on_field(1, "L6", Some(0));
        assert!(digivolve_hand(&mut r, CARD, b));
        if picks == 0 {
            assert!(r.pending_selection().is_none(), "{n}: no lock");
            continue;
        }
        let mut c = candidates(&r);
        c.sort();
        assert_eq!(c, vec!["L4", "L6", "OT"], "Digimon or Tamers");
        r.choose_prompt_card("OT", None).unwrap();
        if picks == 2 {
            r.choose_prompt_card("L6", None).unwrap();
        }
        let _ = r.auto_resolve();
        let ot = find(&r, 1, "OT");
        let l6 = find(&r, 1, "L6");
        let l4 = find(&r, 1, "L4");
        assert!(r.modifiers().has(ot, ModifierType::CannotSuspend));
        assert_eq!(r.modifiers().has(l6, ModifierType::CannotSuspend), picks == 2);
        assert!(!r.modifiers().has(l4, ModifierType::CannotSuspend));
        if n == 8 {
            r.end_turn();
            assert!(r.modifiers().has(ot, ModifierType::CannotSuspend), "through their turn");
            r.end_turn();
            assert!(!r.modifiers().has(ot, ModifierType::CannotSuspend), "ends with their turn");
        }
    }
}

#[test]
fn effect_adding_to_opponent_hand_may_activate_the_option_main_once_per_turn() {
    let mut r = runner(&[], 0);
    let me = on_field(&mut r);
    let l4 = r.place_on_field(1, "L4", Some(0));
    r.place_on_field(1, "L6", Some(0));
    r.place_on_field(1, "L6B", Some(0));
    effect_bounce(&mut r, 0, me, l4);
    let v = r.pending_selection_view().expect("may activate");
    assert!(v.is_optional);
    r.execute_action(0, v.valid_action_ids.iter().copied().find(|a| *a != digimon_engine::action::space::PASS).unwrap()).unwrap();
    let v = r.pending_selection_view().expect("highest-level pick");
    assert_eq!(v.source_kind, EffectSourceKind::Option, "Q&A: an Option card effect");
    let mut c = candidates(&r);
    c.sort();
    assert_eq!(c, vec!["L6", "L6B"], "only highest-level Digimon");
    r.choose_prompt_card("L6B", None).unwrap();
    let _ = r.auto_resolve();
    assert!(hand_ids(&r, 1).contains(&"L6B".to_string()));
    assert_eq!(field_ids(&r, 0), vec![CARD], "the Option side was not used");
    let l6 = find(&r, 1, "L6");
    effect_bounce(&mut r, 0, me, l6);
    assert!(r.pending_selection().is_none(), "once per turn, including its own bounce");
}

#[test]
fn declining_keeps_the_target_and_the_once_per_turn() {
    let mut r = runner(&[], 0);
    let me = on_field(&mut r);
    let l4 = r.place_on_field(1, "L4", Some(0));
    r.place_on_field(1, "L6", Some(0));
    effect_bounce(&mut r, 0, me, l4);
    r.decline_prompt().unwrap();
    let _ = r.auto_resolve();
    assert_eq!(field_ids(&r, 1), vec!["L6"]);
    let l6 = find(&r, 1, "L6");
    effect_bounce(&mut r, 0, me, l6);
    assert!(r.pending_selection_view().is_some_and(|v| v.is_optional), "declined, so still available");
}

#[test]
fn fires_on_the_opponents_turn_when_their_own_effect_adds_to_their_hand() {
    let mut r = runner(&[], 0);
    on_field(&mut r);
    let l4 = r.place_on_field(1, "L4", Some(0));
    r.place_on_field(1, "L6", Some(0));
    r.end_turn();
    effect_bounce(&mut r, 1, l4, l4);
    assert!(r.pending_selection_view().is_some_and(|v| v.is_optional && v.selecting_player == 0));
}

#[test]
fn own_hand_gains_and_rule_draws_do_not_trigger() {
    let mut r = runner(&[], 0);
    let me = on_field(&mut r);
    let mine = r.place_on_field(0, "L4", Some(0));
    let opp = r.place_on_field(1, "L6", Some(0));
    effect_bounce(&mut r, 1, opp, mine);
    assert!(r.pending_selection().is_none(), "my hand, not the opponent's");
    r.end_turn();
    assert!(r.pending_selection().is_none(), "the opponent's rule draw is not an effect");
    let _ = me;
}

#[test]
fn effects_trashing_cards_under_your_tamers_trigger_it() {
    let mut r = runner(&["BT27-030"], 0);
    on_field(&mut r);
    stocked(&mut r, "T", &["FD", "FD", "FD"]);
    r.place_on_field(1, "L6", Some(0));
    play_hand(&mut r, "BT27-030");
    choose_branch(&mut r, 0);
    pay_tamer_stock(&mut r);
    let mut saw = false;
    for _ in 0..6 {
        let Some(v) = r.pending_selection_view() else { break };
        if v.prompt.contains("BT27-032") {
            saw = true;
            let a = v.valid_action_ids.iter().copied().find(|a| *a != digimon_engine::action::space::PASS).unwrap();
            r.execute_action(0, a).unwrap();
            r.choose_prompt_card("L6", None).unwrap();
        } else {
            r.decline_prompt().unwrap();
        }
    }
    assert!(saw, "trashing face-down cards under a Tamer by effect");
    assert!(hand_ids(&r, 1).contains(&"L6".to_string()));
}

#[test]
fn using_the_option_side_needs_data_squad_pays_5_bounces_highest_then_offers_arts_digivolve() {
    let mut r = runner(&[CARD], 0);
    r.place_on_field(1, "L4", Some(0));
    r.place_on_field(1, "L6", Some(0));
    let i = hand_index(&r, 0, CARD);
    let _ = r.game.play_option_from_hand(0, i);
    assert!(r.pending_selection().is_none() && hand_ids(&r, 0) == vec![CARD], "no [DATA SQUAD] permanent");

    let mut r = runner(&[CARD], 0);
    r.place_on_field(0, "T", None);
    r.place_on_field(0, "SQ5", Some(0));
    r.place_on_field(1, "L4", Some(0));
    r.place_on_field(1, "L6", Some(0));
    let mem = r.memory();
    let i = hand_index(&r, 0, CARD);
    let _ = r.game.play_option_from_hand(0, i);
    assert_eq!(candidates(&r), vec!["L6"]);
    r.choose_prompt_card("L6", None).unwrap();
    assert_eq!(mem - r.memory(), 5);
    assert!(hand_ids(&r, 1).contains(&"L6".to_string()));
    let v = r.pending_selection_view().expect("Arts Digivolve offer");
    assert!(v.is_optional);
}
