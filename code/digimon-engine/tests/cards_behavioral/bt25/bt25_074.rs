//! BT25-074 Tankdramon — Digimon, Lv.5, Black+Purple, DP 7000, Cost 7
//! (Machine, D-Brigade, ACCEL).
//!
//! Printed (official Bandai DB bundle + DCGO BT25_074.cs):
//! - Evo box: Black Lv.4 / cost 4, Purple Lv.4 / cost 4; "[Digivolve] Lv.4
//!   w/[D-Brigade]/[ACCEL] trait: Cost 3".
//! - [When Digivolving][When Attacking][Once Per Turn] Reveal the top 3 cards of
//!   your deck. You may play 1 play cost 12 or lower [D-Brigade] or [ACCEL] trait
//!   Digimon card among them with the cost reduced by 5. Trash the rest.
//! - [All Turns][Once Per Turn] When any of your [D-Brigade] or [ACCEL] trait
//!   Digimon are played, 1 of your opponent's Digimon can't digivolve until their
//!   turn ends.
//! - Inherited: [Opponent's Turn] This Digimon with [Chaosmon] in its name or the
//!   [D-Brigade] or [ACCEL] trait gains <Reboot> and <Blocker>.
//!
//! Regression (hunt2 cluster alt-digivolve-cost-cards): no YAML spec. DCGO's cost
//! choice (3 vs 4) met no prompt for a Black/Purple Lv.4 [D-Brigade]. Synthetic
//! boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, PASS, SEL_REVEAL_START}};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, ModifierType};
use digimon_engine::selection::SelectionKind;
use digimon_engine::TriggerSource;

const ID: &str = "BT25-074";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn bt25_074_black_or_purple_lv4_circle_costs_4_without_prompt() {
    assert_single_route(ID, base("B4", "Black Champion", 4, &[BLACK], &["Machine"]), 4);
    assert_single_route(ID, base("P4", "Purple Champion", 4, &[PURPLE], &["Machine"]), 4);
}

#[test]
fn bt25_074_lv4_d_brigade_or_accel_of_any_colour_costs_3() {
    assert_single_route(ID, base("DB4", "Red Brigade", 4, &[RED], &["D-Brigade"]), 3);
    assert_single_route(ID, base("AC4", "Red Accel", 4, &[RED], &["ACCEL"]), 3);
}

#[test]
fn bt25_074_black_or_purple_lv4_d_brigade_prompts_cost_3_vs_4() {
    assert_cost_choice(ID, base("BDB4", "Black Brigade", 4, &[BLACK], &["D-Brigade"]), 3, 4);
    assert_cost_choice(ID, base("PAC4", "Purple Accel", 4, &[PURPLE], &["ACCEL"]), 3, 4);
}

#[test]
fn bt25_074_other_bases_have_no_route() {
    assert_not_offered(ID, base("DB3", "Lv3 Brigade", 3, &[BLACK], &["D-Brigade"]));
    assert_not_offered(ID, base("DB5", "Lv5 Brigade", 5, &[BLACK], &["D-Brigade"]));
    assert_not_offered(ID, base("R4", "Red Champion", 4, &[RED], &["Machine"]));
}

// ─── [When Digivolving][When Attacking][OPT] reveal 3, play with cost -5 ─────

fn digimon(id: &str, name: &str, traits: &[&str], play_cost: u16) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Black];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// BT25-074 in hand, a Black Lv.4 [D-Brigade] base on the field (cost-3 route
/// plus the cost-4 circle: pick via the helper), the given deck (top 3 are
/// revealed), memory `mem`.
fn runner_with(deck: &[&str], mem: i16) -> (DebugRunner, usize) {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT25-074 production YAML loads")
        .add_card(base("RED-DB", "Red Brigade", 4, &[CardColor::Red], &["D-Brigade"]))
        .add_card(digimon("CHEAP", "Cheap Brigade", &["D-Brigade"], 8))
        .add_card(digimon("BIG", "Big Accel", &["ACCEL"], 12))
        .add_card(digimon("TOO-BIG", "Too Big Brigade", &["D-Brigade"], 13))
        .add_card(digimon("OFF", "Off Trait", &["Machine"], 5))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &[ID])
        .deck(0, deck)
        .deck(1, &["FILL"; 12])
        .memory(mem)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "RED-DB", Some(0)).index as usize;
    (r, slot)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0, "digivolve offered");
    r.game.decode_action(a, 0);
}

fn revealed_action(r: &DebugRunner, id: &str) -> Option<u16> {
    r.game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(i, c)| (c.card_id(&r.game.card_data) == id).then_some(SEL_REVEAL_START + i as u16))
}

fn field_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

#[test]
fn bt25_074_plays_the_pick_paying_cost_minus_5_and_trashes_the_rest() {
    let (mut r, slot) = runner_with(&["FILL", "CHEAP", "OFF", "FILL", "FILL"], 10);
    digivolve(&mut r, slot);
    assert_eq!(r.memory(), 7, "the cost-3 route");
    let view = r.pending_selection_view().expect("bucket pick");
    assert!(matches!(view.kind, SelectionKind::RevealBucket { .. }), "{:?}", view.kind);
    let offered: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(
        offered,
        vec![revealed_action(&r, "CHEAP").expect("CHEAP revealed")],
        "only the [D-Brigade] Digimon with play cost <= 12 is offered"
    );
    r.execute_action(0, offered[0]).unwrap();
    let _ = r.auto_resolve();
    assert_eq!(r.memory(), 7 - 3, "printed cost 8, reduced by 5 -> pays 3");
    assert!(field_ids(&r).contains(&"CHEAP".to_string()), "played: {:?}", field_ids(&r));
    assert_eq!(r.trash_size(0), 2, "the other two revealed cards are trashed");
}

#[test]
fn bt25_074_a_play_cost_13_card_and_off_trait_cards_are_never_offered() {
    let (mut r, slot) = runner_with(&["FILL", "TOO-BIG", "OFF", "FILL", "FILL"], 10);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_none(), "nothing eligible: no pick");
    assert_eq!(r.trash_size(0), 3, "all three revealed cards are trashed");
}

#[test]
fn bt25_074_declining_the_pick_trashes_every_revealed_card() {
    let (mut r, slot) = runner_with(&["FILL", "CHEAP", "BIG", "FILL", "FILL"], 10);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("bucket pick");
    let _ = view;
    r.execute_action(0, PASS).expect("\"you may\": decline");
    let _ = r.auto_resolve();
    assert_eq!(r.trash_size(0), 3);
    assert_eq!(r.memory(), 7);
}

#[test]
fn bt25_074_when_attacking_shares_the_once_per_turn_with_when_digivolving() {
    let (mut r, slot) = runner_with(&["FILL", "CHEAP", "FILL", "FILL", "FILL", "FILL", "FILL"], 10);
    digivolve(&mut r, slot);
    r.execute_action(0, PASS).ok();
    let _ = r.auto_resolve();
    let trash = r.trash_size(0);
    let host = r.perm_handle(0, slot);
    r.game
        .enqueue_triggered(EffectTiming::WhenAttacking, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
    assert!(r.pending_selection().is_none());
    assert_eq!(r.trash_size(0), trash, "the shared once-per-turn counter is spent");
}

// ─── [All Turns][OPT] a [D-Brigade]/[ACCEL] Digimon is played ────────────────

#[test]
fn bt25_074_playing_a_d_brigade_digimon_denies_digivolution_to_an_opponent_digimon() {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT25-074 production YAML loads")
        .add_card(digimon("OFF", "Off Trait", &["Machine"], 1))
        .add_card(digimon("BRIGADE", "Brigade Rookie", &["D-Brigade"], 1))
        .add_card(digimon("OPP", "Opp Rookie", &["Beast"], 3))
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["OFF", "BRIGADE"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    r.place_on_field(0, ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));

    // An off-trait Digimon does nothing.
    r.play(0, 0).expect("play OFF");
    assert!(r.pending_selection().is_none());
    assert!(!r.game.modifiers.has(opp, ModifierType::CannotDigivolve));

    // The [D-Brigade] Digimon triggers the mandatory pick.
    r.play(0, 0).expect("play BRIGADE");
    let view = r.pending_selection_view().expect("mandatory opponent pick");
    assert!(!view.valid_action_ids.contains(&PASS));
    r.execute_action(0, view.valid_action_ids[0]).unwrap();
    let _ = r.auto_resolve();
    assert!(
        r.game.modifiers.has(opp, ModifierType::CannotDigivolve),
        "the chosen Digimon can't digivolve"
    );
}

// ─── Inherited [Opponent's Turn] Reboot + Blocker ────────────────────────────

#[test]
fn bt25_074_inherited_reboot_and_blocker_on_the_opponents_turn_for_a_brigade_host() {
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT25-074 production YAML loads")
        .add_card(digimon("HOST-DB", "Brigade Host", &["D-Brigade"], 4))
        .add_card(digimon("HOST-PLAIN", "Plain Host", &["Machine"], 4))
        .add_card(digimon("HOST-CHAOS", "Chaosmon Host", &["Machine"], 4))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    let brigade = r.place_stack(0, &[ID, "HOST-DB"]);
    let plain = r.place_stack(0, &[ID, "HOST-PLAIN"]);
    let chaos = r.place_stack(0, &[ID, "HOST-CHAOS"]);

    // The owner's own turn: nothing.
    r.game.turn_player_idx = 0;
    for h in [brigade, plain, chaos] {
        assert!(!r.game.has_keyword(h, Keyword::Blocker), "own turn: no Blocker");
    }
    // The opponent's turn: matching hosts only.
    r.game.turn_player_idx = 1;
    assert!(r.game.has_keyword(brigade, Keyword::Blocker));
    assert!(r.game.has_keyword(brigade, Keyword::Reboot));
    assert!(!r.game.has_keyword(plain, Keyword::Blocker));
    assert!(!r.game.has_keyword(plain, Keyword::Reboot));
    assert!(r.game.has_keyword(chaos, Keyword::Blocker), "[Chaosmon] in the name");
    assert!(r.game.has_keyword(chaos, Keyword::Reboot));
}
