//! EX13-001 Gigimon — Digi-Egg, Lv.2, Red. Traits: Lesser.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/EX13-001.md`)
//!
//! Inherited Effect: [Your Turn] [Once Per Turn] When any of your red Tamers
//! are played, this Digimon may digivolve into a Digimon card with [Growlmon]
//! or [Gallantmon] in its name in the hand with the cost reduced by 2.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Red/EX13_001.cs
//! - `OnEnterFieldAnyone`, inherited, OPT (`SetUpActivateClass(..., 1, true, ...)`,
//!   hash `EX13_001_Inherited`).
//! - CanUse: owner's turn && `CanTriggerOnPermanentPlay` with an own Tamer
//!   whose top card is red.
//! - CanActivate: a hand card `ContainsCardName("Growlmon"|"Gallantmon")`.
//! - Body: `DigivolveIntoHandOrTrashCard(payCost: true, reduceCost: 2, isHand: true)`
//!   — the selection is declinable (optional "may").
//!
//! # Patterns this test covers
//! - G4-adjacent: inherited trigger on a DigiEgg source (EX12-002 idiom).
//! - B3: trigger on another permanent's play (red Tamer gate).
//! - E2: OPT + optional decline.
//! - D2: effect-initiated digivolve with a cost reduction.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-001";

fn digimon(id: &str, name: &str, level: u8, play_cost: u16) -> CardData {
    let mut c = make_test_card_with_level(id, name, level);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.play_cost = play_cost;
    c.dp = Some(3000 + 1000 * level as i32);
    c
}

fn evo(id: &str, name: &str) -> CardData {
    let mut c = digimon(id, name, 4, 5);
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 3,
    }];
    c
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.colors = vec![color];
    c.play_cost = 2;
    c
}

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-001 YAML loads")
        .add_card(digimon("CARRIER", "Carrier", 3, 3))
        .add_card(evo("GROWL", "Growlmon"))
        .add_card(evo("GALLANT", "Gallantmon"))
        .add_card(evo("OTHER-EVO", "Greymon"))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(tamer("RED-TAMER-2", CardColor::Red))
        .add_card(tamer("BLUE-TAMER", CardColor::Blue))
        .add_card(digimon("RED-DIGI", "Red Digi", 3, 3))
        .add_card(digimon("FILL", "Fill", 3, 3))
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn top_card_id(runner: &DebugRunner, player: u8, field_index: usize) -> String {
    runner.game.players[player as usize].battle_area[field_index]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn hand_action(runner: &DebugRunner, card_id: &str) -> u16 {
    let view = runner.pending_selection_view().expect("a prompt is pending");
    let slot = hand_index(runner, 0, card_id) as u16;
    [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{card_id} not selectable: {view:?}"))
}

fn play_from_hand(runner: &mut DebugRunner, card_id: &str) {
    let slot = hand_index(runner, 0, card_id);
    assert!(runner.play(0, slot).is_some(), "{card_id} should be playable");
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn ex13_001_has_one_inherited_your_turn_opt_clause() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.name, "Gigimon");
    assert_eq!(card.level, Some(2));
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    assert_eq!(card.effects.len(), 1, "only the inherited clause");
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::Inherited);
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(t.trigger_if.is_some(), "gated on a red Tamer of yours being played");
}

// ─── Section 2/3 — trigger + behavior ────────────────────────────────────────

#[test]
fn ex13_001_red_tamer_play_lets_source_digivolve_into_growlmon_with_cost_reduced_by_2() {
    let mut runner = builder()
        .hand(0, &["RED-TAMER", "GROWL"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let mem0 = runner.memory();

    play_from_hand(&mut runner, "RED-TAMER");
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));
    assert!(runner.pending_is_optional(), "'may digivolve'");
    let a = hand_action(&runner, "GROWL");
    runner.execute_action(0, a).expect("pick Growlmon");
    let _ = runner.auto_resolve();

    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "GROWL");
    assert_eq!(
        mem0 - runner.memory(),
        2 + 1,
        "tamer cost 2 + digivolve cost 3 reduced by 2 = 1"
    );
}

#[test]
fn ex13_001_gallantmon_named_card_is_also_eligible_but_other_names_are_not() {
    let mut runner = builder()
        .hand(0, &["RED-TAMER", "GALLANT", "OTHER-EVO"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    play_from_hand(&mut runner, "RED-TAMER");
    let view = runner.pending_selection_view().expect("hand prompt");
    let other_slot = hand_index(&runner, 0, "OTHER-EVO") as u16;
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + other_slot))
            && !view.valid_action_ids.contains(&(HAND_EFFECT_START + other_slot)),
        "a Greymon is not [Growlmon]/[Gallantmon]-named"
    );
    let a = hand_action(&runner, "GALLANT");
    runner.execute_action(0, a).expect("pick Gallantmon");
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "GALLANT");
}

#[test]
fn ex13_001_declining_leaves_the_source_undigivolved() {
    let mut runner = builder()
        .hand(0, &["RED-TAMER", "GROWL"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    play_from_hand(&mut runner, "RED-TAMER");
    assert!(runner.pending_is_optional());
    runner.execute_action(0, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "CARRIER");
    assert_eq!(runner.hand_size(0), 1, "Growlmon stays in hand");
}

#[test]
fn ex13_001_non_red_tamer_play_does_not_trigger() {
    let mut runner = builder()
        .hand(0, &["BLUE-TAMER", "GROWL"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    play_from_hand(&mut runner, "BLUE-TAMER");
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none());
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "CARRIER");
}

#[test]
fn ex13_001_red_digimon_play_does_not_trigger() {
    let mut runner = builder()
        .hand(0, &["RED-DIGI", "GROWL"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    play_from_hand(&mut runner, "RED-DIGI");
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "CARRIER");
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn ex13_001_does_not_trigger_from_the_breeding_area_or_without_a_source_egg() {
    // The effect is inherited: Gigimon in the breeding area (not a source of
    // a battle-area Digimon) must not fire it for the unrelated CARRIER.
    let mut runner = builder()
        .hand(0, &["RED-TAMER", "GROWL"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    runner.place_in_breeding(0, CARD_ID);
    play_from_hand(&mut runner, "RED-TAMER");
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "CARRIER");
}

// ─── Section 5 — OPT lockout ─────────────────────────────────────────────────

#[test]
fn ex13_001_once_per_turn_second_red_tamer_does_not_retrigger() {
    let mut runner = builder()
        .hand(0, &["RED-TAMER", "RED-TAMER-2", "GROWL", "GALLANT"])
        .deck(0, &["FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    play_from_hand(&mut runner, "RED-TAMER");
    let a = hand_action(&runner, "GROWL");
    runner.execute_action(0, a).expect("pick Growlmon");
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "GROWL");

    play_from_hand(&mut runner, "RED-TAMER-2");
    let _ = runner.auto_resolve();
    assert!(runner.pending_selection().is_none(), "OPT already used");
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "GROWL");
}

#[test]
fn ex13_001_once_per_turn_lockout_clears_on_your_next_turn() {
    let mut runner = builder()
        .hand(0, &["RED-TAMER", "RED-TAMER-2", "GROWL", "GALLANT"])
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    play_from_hand(&mut runner, "RED-TAMER");
    runner.execute_action(0, PASS).expect("decline this time");
    let _ = runner.auto_resolve();

    runner.game.set_memory(-3);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    runner.game.set_memory(-3);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 0);
    runner.game.set_memory(10);

    play_from_hand(&mut runner, "RED-TAMER-2");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Hand),
        "the OPT resets on your next turn"
    );
    let a = hand_action(&runner, "GROWL");
    runner.execute_action(0, a).expect("digivolve");
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "GROWL");
}

#[test]
fn ex13_001_opponent_turn_red_tamer_does_not_trigger() {
    // [Your Turn] gate: an opponent's turn play of YOUR red tamer is not
    // possible in normal play, so check that an opponent's red Tamer on
    // their own turn doesn't fire your inherited effect.
    let mut runner = builder()
        .hand(1, &["RED-TAMER"])
        .hand(0, &["GROWL"])
        .deck(0, &["FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    runner.game.set_memory(-10);
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.turn_player(), 1);
    let slot = hand_index(&runner, 1, "RED-TAMER");
    let _ = runner.play(1, slot);
    let _ = runner.auto_resolve();
    assert_eq!(top_card_id(&runner, 0, carrier.index as usize), "CARRIER");
}

// ─── Candidate legality: only cards the paid digivolve can complete ──────────
//
// DCGO `DigivolveIntoHandOrTrashCard(payCost: true, reduceCost: 2)`
// (EX13_001.cs:55-64) offers a hand card when `!CanNotEvolve(target)` and
// `CanPlayCardTargetFrame(target.PermanentFrame, PayCost: true, ...)`
// (CardEffectCommons.cs:858-882). For the occupied carrier frame that is a
// ROUTE-ONLY check: the `MaxMemoryCost < cost` refusal (CardSource.cs:1150-1162)
// sits inside `if (PayCost && frame.GetFramePermanent() == null)`, i.e. only
// for plays into an empty frame, and `CanEvolve` (CardSource.cs:1269-1290) /
// `EvoCosts` never compare cost to memory. So DCGO offers a [Growlmon] card the
// carrier has a route onto even when the player can't pay for it.
//
// We deliberately diverge (rules over DCGO): CR 4.3 1-3-11-1 forbids declaring
// a digivolve whose cost can't be paid, and our effect digivolve refuses it
// (`Game::can_effect_digivolve_from_source_with_cost`, game_actions/
// digivolve.rs), so an unaffordable card is not offered (it would be an illegal
// choice in the action space) and a hand holding only such cards opens no pick.
// The no-route exclusion matches DCGO. A DCGO recording that offers or picks an
// unaffordable card here is a known `dcgo_quirk`, not an engine bug.

/// A red Lv.4 [Growlmon]-named Digimon digivolving from a red Lv.3 for `cost`.
fn growl_costing(id: &str, cost: u8) -> CardData {
    let mut c = digimon(id, "Growlmon", 4, 5);
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: cost.into(),
    }];
    c
}

/// A red Lv.6 [Growlmon]-named Digimon that needs a Lv.5: no route onto the Lv.3 carrier.
fn growl_needing_lv5(id: &str) -> CardData {
    let mut c = digimon(id, "WarGrowlmon", 6, 10);
    c.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 5,
        memory_cost: 3,
    }];
    c
}

/// Memory 3, play the red Tamer (cost 2) → 1 memory, paying at most 11 (CR 4.3
/// memory floor −10). Returns the runner with the hand pick (if any) pending.
fn after_red_tamer(hand: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .add_card(growl_costing("GROWL-13", 13))
        .add_card(growl_costing("GROWL-14", 14))
        .add_card(growl_needing_lv5("GROWL-LV6"))
        .hand(0, hand)
        .deck(0, &["FILL", "FILL"])
        .memory(3)
        .start();
    runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    play_from_hand(&mut runner, "RED-TAMER");
    runner
}

fn offered(runner: &DebugRunner) -> Vec<String> {
    let Some(view) = runner.pending_selection_view() else {
        return Vec::new();
    };
    if view.kind != SelectionKind::Hand {
        return Vec::new();
    }
    let hand: Vec<String> = runner.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect();
    let mut out: Vec<String> = view
        .valid_action_ids
        .iter()
        .filter(|&&a| a != PASS)
        .filter_map(|&a| {
            [PLAY_HAND_START, HAND_EFFECT_START]
                .iter()
                .filter_map(|&s| a.checked_sub(s))
                .find(|&i| (i as usize) < hand.len() && i < 30)
        })
        .map(|i| hand[i as usize].clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// GROWL-13 costs 11 after the −2 (payable); GROWL-14 costs 12 (not).
#[test]
fn ex13_001_hand_pick_offers_only_growlmon_affordable_after_the_reduction() {
    let runner = after_red_tamer(&["RED-TAMER", "GROWL-13", "GROWL-14"]);
    assert_eq!(offered(&runner), vec!["GROWL-13".to_string()]);
}

/// Only an unaffordable [Growlmon] card in hand: no pick opens.
#[test]
fn ex13_001_no_pick_when_the_only_growlmon_is_unaffordable() {
    let runner = after_red_tamer(&["RED-TAMER", "GROWL-14"]);
    assert!(
        offered(&runner).is_empty(),
        "GROWL-14 can't be paid for: {:?}",
        runner.pending_selection_view()
    );
}

/// Regression guard: a [Growlmon] card with no route onto the Lv.3 carrier is
/// not offered (the compile pass's `can_digivolve_onto: source`).
#[test]
fn ex13_001_hand_pick_excludes_growlmon_without_a_route_onto_the_carrier() {
    let runner = after_red_tamer(&["RED-TAMER", "GROWL-LV6", "GROWL-13"]);
    assert_eq!(offered(&runner), vec!["GROWL-13".to_string()]);
}
