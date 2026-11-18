//! EX5-045 Chuumon — Lv.3 black/yellow Rookie, DP 1000, cost 3
//! (digivolve: black Lv.2 / 1, yellow Lv.2 / 1).
//!
//! Printed text (official Bandai DB bundle + DCGO EX5_045.cs):
//! - [On Play] If it's your opponent's turn, reveal the top 3 cards of your
//!   deck. You may play 1 Digimon card with [Sukamon] in its name among them
//!   without paying the cost. Trash the rest.
//! - Inherited: [On Deletion] If this Digimon had [Sukamon] or [Etemon] in its
//!   name, you may play 1 [Chuumon] from your trash suspended without paying
//!   the cost.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn runner(deck_top_first_ids: &[&str]) -> DebugRunner {
    let r = DebugRunner::builder()
        .dsl_card("EX5-045")
        .expect("EX5-045 loads")
        .add_card(digimon("SUKA", "Sukamon", 4, 4000, 4, vec![CardColor::Black], &[]))
        .add_card(digimon("SUKA2", "Sukamon", 4, 4000, 4, vec![CardColor::Black], &[]))
        .add_card(card("SUKA-OPT", "Sukamon Option", CardKind::Option, vec![CardColor::Black]))
        .add_card(digimon("ETE", "Etemon", 5, 6000, 6, vec![CardColor::Black], &[]))
        .add_card(digimon("CHUU", "Chuumon", 3, 1000, 3, vec![CardColor::Black], &[]))
        .add_card(digimon("OTHER", "Other Digimon", 3, 2000, 3, vec![CardColor::Black], &[]))
        .add_card(digimon("HOST", "Host Rookie", 3, 2000, 3, vec![CardColor::Black], &[]))
        .deck(0, &top_first(deck_top_first_ids))
        .deck(1, &["OTHER"; 5])
        .memory(10)
        .start();
    into_main(r, 0, 10)
}

fn opponents_turn(r: &mut DebugRunner) {
    r.game.turn_player_idx = 1;
    r.game.memory_pair = (1, 0);
}

fn delete(r: &mut DebugRunner, h: digimon_engine::permanent::PermanentHandle) {
    r.game.delete_permanent_with_effects(h);
    r.game.drain_effect_queue();
}

#[test]
fn ex5_045_on_play_during_the_opponents_turn_plays_a_sukamon_digimon_free_and_trashes_the_rest() {
    let mut r = runner(&["OTHER", "SUKA", "ETE", "OTHER"]);
    let chuu = r.place_on_field(0, "EX5-045", Some(0));
    opponents_turn(&mut r);
    r.fire_on_play(0, chuu.index as usize);
    let view = r.pending_selection_view().expect("the play pick is a prompt");
    assert!(view.is_optional, "'you may play': declinable");
    assert_eq!(view.valid_action_ids.len(), 1, "only the Sukamon Digimon (not Etemon / OTHER)");
    let memory = r.memory();
    pick_reveal(&mut r, 0, "SUKA");
    r.auto_resolve().unwrap();
    assert!(field_ids(&r, 0).contains(&"SUKA".to_string()), "Sukamon played");
    assert_eq!(r.memory(), memory, "without paying the cost");
    let mut trash = trash_ids(&r, 0);
    trash.sort();
    assert_eq!(trash, vec!["ETE", "OTHER"], "the rest of the reveal is trashed");
    assert_eq!(deck_top_first(&r, 0), vec!["OTHER"], "the 4th card was never revealed");
}

#[test]
fn ex5_045_on_play_can_decline_the_play_and_still_trashes_all_three() {
    let mut r = runner(&["SUKA", "ETE", "OTHER", "OTHER"]);
    let chuu = r.place_on_field(0, "EX5-045", Some(0));
    opponents_turn(&mut r);
    r.fire_on_play(0, chuu.index as usize);
    r.execute_action(0, PASS).expect("decline the play");
    r.auto_resolve().unwrap();
    assert_eq!(field_ids(&r, 0), vec!["EX5-045"], "nothing was played");
    assert_eq!(trash_ids(&r, 0).len(), 3, "all three revealed cards were trashed");
}

#[test]
fn ex5_045_on_play_does_nothing_on_your_own_turn() {
    let mut r = runner(&["SUKA", "ETE", "OTHER", "OTHER"]);
    let chuu = r.place_on_field(0, "EX5-045", Some(0));
    r.fire_on_play(0, chuu.index as usize);
    assert!(r.pending_selection().is_none(), "[On Play] needs the opponent's turn");
    assert_eq!(r.deck_size(0), 4, "no reveal happened");
}

#[test]
fn ex5_045_on_play_requires_a_sukamon_name_digimon_not_an_option_or_etemon() {
    let mut r = runner(&["SUKA-OPT", "ETE", "OTHER", "OTHER"]);
    let chuu = r.place_on_field(0, "EX5-045", Some(0));
    opponents_turn(&mut r);
    r.fire_on_play(0, chuu.index as usize);
    r.auto_resolve().unwrap();
    assert_eq!(field_ids(&r, 0), vec!["EX5-045"]);
    assert_eq!(trash_ids(&r, 0).len(), 3, "no legal play: the whole reveal is trashed");
}

#[test]
fn ex5_045_inherited_on_deletion_plays_a_chuumon_from_trash_suspended() {
    let mut r = runner(&["OTHER"; 4]);
    r.inject_trash(0, "CHUU");
    let host = r.place_stack(0, &["EX5-045", "SUKA"]);
    delete(&mut r, host);
    // The declinable trash pick is the 'you may play' decision itself.
    let view = r.pending_selection_view().expect("trash pick");
    assert!(view.is_optional, "'you may play'");
    // A [Chuumon] is in the trash (the injected one and the EX5-045 source).
    assert!(!view.valid_action_ids.is_empty());
    let pick = view.valid_action_ids.iter().copied().find(|a| *a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
    let _ = r.auto_resolve();
    let chuumon: Vec<_> = r.game.players[0]
        .battle_area
        .iter()
        .filter(|p| p.top_card().card_id(&r.game.card_data) != "")
        .collect();
    assert_eq!(chuumon.len(), 1, "one Chuumon was played");
    assert!(chuumon[0].is_suspended, "played SUSPENDED");
}

#[test]
fn ex5_045_inherited_on_deletion_needs_the_deleted_digimon_to_have_sukamon_or_etemon_in_its_name() {
    let mut r = runner(&["OTHER"; 4]);
    r.inject_trash(0, "CHUU");
    let host = r.place_stack(0, &["EX5-045", "HOST"]);
    delete(&mut r, host);
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty(), "a plain host: no Chuumon is played");
}

#[test]
fn ex5_045_inherited_on_deletion_also_works_for_an_etemon_named_host() {
    let mut r = runner(&["OTHER"; 4]);
    r.inject_trash(0, "CHUU");
    let host = r.place_stack(0, &["EX5-045", "ETE"]);
    delete(&mut r, host);
    assert!(r.pending_is_optional(), "an Etemon-named host also qualifies");
}

#[test]
fn ex5_045_inherited_on_deletion_can_be_declined() {
    let mut r = runner(&["OTHER"; 4]);
    r.inject_trash(0, "CHUU");
    let host = r.place_stack(0, &["EX5-045", "SUKA"]);
    delete(&mut r, host);
    let view = r.pending_selection_view().expect("trash pick");
    r.execute_action(view.selecting_player, PASS).expect("decline the play");
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).is_empty());
}
