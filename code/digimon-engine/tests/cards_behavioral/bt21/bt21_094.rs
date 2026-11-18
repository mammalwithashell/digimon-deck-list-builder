//! BT21-094 Armor Digivolution — Option, cost 3, blue.
//!
//! Printed text (official Bandai DB bundle + DCGO BT21_094.cs):
//! - [Main] Reveal the top 3 cards of your deck. Add 1 card with [Davis
//!   Motomiya] in its name and 1 card with the [Free] trait among them to the
//!   hand. Trash the rest. Then, place this card in the battle area.
//! - [All Turns] When the top stacked card of any of your [Armor Form] trait
//!   Digimon is trashed, <Delay> ... 1 of your Digimon may digivolve into a
//!   Digimon card with the [Armor Form] trait in the hand without paying the
//!   cost.
//! - Security Effect [Security] Activate this card's [Main] effects.
//!
//! Synthetic boards only (DCGO replay hunt round 2, cluster reveal-bucket-cards).
#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use crate::reveal_bucket_support::*;
use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger, EffectTiming};
use digimon_engine::permanent::{OptionState, PermanentHandle};
use digimon_engine::selection::{OptionPlayResult, SelectionKind};

fn armor_form(id: &str, level: u8) -> CardData {
    let mut c = digimon(id, "Armor Digimon", level, 5000, 5, vec![CardColor::Blue], &["Armor Form"]);
    if level == 4 {
        c = with_evo(c, CardColor::Blue, 3, 3);
    }
    c
}

fn builder(deck_top_first_ids: &[&str], hand: &[&str]) -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT21-094")
        .expect("BT21-094 loads")
        .add_card(card("DAVIS", "Davis Motomiya", CardKind::Tamer, vec![CardColor::Blue]))
        .add_card(card("DAVIS-OPT", "Davis Motomiya's Option", CardKind::Option, vec![CardColor::Blue]))
        .add_card(digimon("FREE3", "Free Rookie", 3, 3000, 3, vec![CardColor::Blue], &["Free"]))
        .add_card(digimon("PLAIN", "Plain Digimon", 3, 3000, 3, vec![CardColor::Blue], &[]))
        .add_card(armor_form("ARMOR-TOP", 4))
        .add_card(armor_form("ARMOR-EVO", 4))
        .add_card(digimon("LOW", "Low Rookie", 3, 3000, 3, vec![CardColor::Blue], &[]))
        .add_card(digimon("FILL", "Filler", 3, 1000, 3, vec![CardColor::Red], &[]))
        .deck(0, &top_first(deck_top_first_ids))
        .deck(1, &["FILL"; 6])
        .hand(0, hand)
        .memory(10)
}

fn play_94(r: &mut DebugRunner) {
    r.place_on_field(0, "LOW", Some(0));
    r.game.enter_main_phase();
    // (The synchronous path returns `Trashed` whatever the disposal; the
    // placement is asserted through the battle area.)
    let _ = r.game.play_option_from_hand(0, 0);
}

fn state_of_94(r: &DebugRunner) -> Option<OptionState> {
    r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "BT21-094")
        .map(|p| p.option_state)
}

#[test]
fn bt21_094_main_adds_a_davis_motomiya_name_card_and_a_free_trait_card_and_trashes_the_rest() {
    let mut r = builder(&["PLAIN", "DAVIS", "FREE3", "FILL"], &["BT21-094"]).start();
    play_94(&mut r);
    let view = r.pending_selection_view().expect("Davis bucket");
    assert!(!view.is_optional, "mandatory when a candidate exists");
    pick_reveal(&mut r, 0, "DAVIS");
    pick_reveal(&mut r, 0, "FREE3");
    r.auto_resolve().unwrap();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["DAVIS", "FREE3"]);
    assert_eq!(trash_ids(&r, 0), vec!["PLAIN"], "the rest of the reveal is TRASHED (not bottomed)");
    assert_eq!(deck_top_first(&r, 0), vec!["FILL"], "the 4th card was never revealed");
    assert!(state_of_94(&r).is_some(), "placed in the battle area");
}

#[test]
fn bt21_094_the_davis_pick_matches_any_card_kind_by_name() {
    let mut r = builder(&["DAVIS-OPT", "FREE3", "PLAIN"], &["BT21-094"]).start();
    play_94(&mut r);
    let view = r.pending_selection_view().unwrap();
    assert_eq!(view.valid_action_ids.len(), 1, "the named Option qualifies");
    pick_reveal(&mut r, 0, "DAVIS-OPT");
    pick_reveal(&mut r, 0, "FREE3");
    r.auto_resolve().unwrap();
    let mut hand = hand_ids(&r, 0);
    hand.sort();
    assert_eq!(hand, vec!["DAVIS-OPT", "FREE3"]);
}

#[test]
fn bt21_094_one_card_cannot_fill_both_picks() {
    let mut both = digimon("BOTH", "Davis Motomiya Free", 3, 3000, 3, vec![CardColor::Blue], &["Free"]);
    let mut r = builder(&["BOTH", "PLAIN", "PLAIN"], &["BT21-094"]).add_card(both).start();
    play_94(&mut r);
    pick_reveal(&mut r, 0, "BOTH");
    r.auto_resolve().unwrap();
    assert_eq!(hand_ids(&r, 0), vec!["BOTH"], "added once");
    assert_eq!(trash_ids(&r, 0).len(), 2, "the other two were trashed");
}

fn place_and_wait_a_turn(r: &mut DebugRunner) {
    play_94(r);
    r.auto_resolve().unwrap();
    r.end_turn();
    r.game.enter_main_phase();
    r.end_turn();
    assert_eq!(r.game.turn_player(), 0);
    r.game.enter_main_phase();
}

fn purge_top(r: &mut DebugRunner, host: PermanentHandle) {
    let top = r.game.players[host.player as usize].battle_area[host.index as usize]
        .top_card()
        .handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), host.player);
        ctx.armor_purge_top(host);
    }
    r.game.drain_effect_queue();
}

#[test]
fn bt21_094_delay_fires_when_the_top_card_of_your_armor_form_digimon_is_trashed() {
    let mut r = builder(&["FILL"; 8], &["BT21-094", "ARMOR-EVO"]).start();
    place_and_wait_a_turn(&mut r);
    let host = r.place_stack(0, &["LOW", "ARMOR-TOP"]);
    purge_top(&mut r, host);
    let view = r.pending_selection_view().expect("the Delay activation prompt");
    assert_eq!(view.kind, SelectionKind::Replacement);
    assert!(r.pending_is_optional(), "<Delay> is optional (CR 4.3 16-17-2)");
    r.execute_action(view.selecting_player, REPLACEMENT_ACCEPT).unwrap();
    let memory = r.memory();
    // Body: the Digimon to digivolve, then the Armor Form card from hand.
    let mut guard = 0;
    while let Some(view) = r.pending_selection_view() {
        guard += 1;
        assert!(guard < 6, "unexpected prompt loop");
        let pick = view.valid_action_ids.iter().copied().find(|a| *a != PASS).unwrap();
        r.execute_action(view.selecting_player, pick).unwrap();
    }
    assert!(
        field_ids(&r, 0).contains(&"ARMOR-EVO".to_string()),
        "digivolved into the [Armor Form] card from hand"
    );
    assert_eq!(r.memory(), memory, "without paying the cost");
    assert!(
        trash_ids(&r, 0).contains(&"BT21-094".to_string()),
        "the Delay trashed this card"
    );
}

#[test]
fn bt21_094_delay_ignores_a_top_card_without_the_armor_form_trait() {
    let mut r = builder(&["FILL"; 8], &["BT21-094", "ARMOR-EVO"]).start();
    place_and_wait_a_turn(&mut r);
    let host = r.place_stack(0, &["LOW", "PLAIN"]);
    purge_top(&mut r, host);
    assert!(r.pending_selection().is_none(), "a plain top card does not trigger the Delay");
    assert!(state_of_94(&r).is_some(), "BT21-094 stays parked");
}

#[test]
fn bt21_094_delay_ignores_an_opponents_armor_form_digimon() {
    let mut r = builder(&["FILL"; 8], &["BT21-094", "ARMOR-EVO"]).start();
    place_and_wait_a_turn(&mut r);
    let host = r.place_stack(1, &["LOW", "ARMOR-TOP"]);
    purge_top(&mut r, host);
    assert!(r.pending_selection().is_none(), "only YOUR Digimon count");
}

#[test]
fn bt21_094_delay_does_not_fire_on_the_placing_turn() {
    let mut r = builder(&["FILL"; 8], &["BT21-094", "ARMOR-EVO"]).start();
    play_94(&mut r);
    r.auto_resolve().unwrap();
    let host = r.place_stack(0, &["LOW", "ARMOR-TOP"]);
    purge_top(&mut r, host);
    assert!(r.pending_selection().is_none(), "never on the placing turn");
}

#[test]
fn bt21_094_delay_fires_when_a_two_card_de_digivolve_trashes_an_armor_form_top_card() {
    // One <De-Digivolve 2> instance trashes both cards at the same time (CR 4.3
    // 16-12-5); DCGO fires when ANY trashed card is an [Armor Form] card.
    let mut r = builder(&["FILL"; 8], &["BT21-094", "ARMOR-EVO"]).start();
    place_and_wait_a_turn(&mut r);
    let host = r.place_stack(0, &["LOW", "PLAIN", "ARMOR-TOP"]);
    let top = r.game.players[0].battle_area[host.index as usize].top_card().handle();
    {
        let mut ctx = EffectContext::new(&mut r.game, top, Some(host), 0);
        ctx.de_digivolve(host, Some(3), Some(2));
    }
    r.game.drain_effect_queue();
    assert!(
        r.pending_selection().is_some(),
        "an Armor Form card was among the 2 trashed top cards"
    );
}
