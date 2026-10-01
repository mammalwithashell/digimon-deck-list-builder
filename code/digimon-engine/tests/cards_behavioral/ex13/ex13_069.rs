//! EX13-069 Rina Shinomiya — Tamer, Blue, Cost 3. Traits: CS.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-069.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-069.md` agrees. The corpus files the
//! [Security] line under `inherited_effect_description_eng`.)
//!
//! ```text
//! [Start of Your Main Phase] If you have a Digimon with [Veemon] or [Veedramon]
//! in its name, gain 1 memory.
//! [Your Turn] When any of your Digimon unsuspend, by suspending this Tamer,
//! <Draw 1> (Draw 1 card from your deck.). After, 1 of your Digimon may
//! digivolve into a Digimon card with [Veedramon] in its name in the hand with
//! the cost reduced by 2.
//!
//! [Security] Play this card without paying the cost.
//! ```
//!
//! Official Q&A: if you don't suspend this card, you can't process the part
//! after "After".
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Blue/EX13_069.cs
//! - `StartOfYourMainPhaseClass` (mandatory), gated on an own Digimon whose top
//!   card `ContainsCardName("Veemon"|"Veedramon")` → `AddMemory(1)`.
//! - `OnUnTappedAnyone` ActivateClass (no per-turn cap, optional): owner's turn,
//!   own Digimon unsuspended, `CanActivateSuspendCostEffect(this)` → suspend this
//!   Tamer; on success `DrawClass(1)`, then (own Digimon AND a [Veedramon]-name
//!   Digimon card in hand) `SelectPermanentEffect(own Digimon, canNoSelect: true)`
//!   → `DigivolveIntoHandOrTrashCard(payCost, reduceCost 2, isHand)`.
//! - `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - Start-of-main memory gain gated on board state (BT24-084 shape).
//! - F own-Digimon-unsuspended observer with a suspend-self activation cost →
//!   Draw 1 → optional own-Digimon digivolve from hand, cost −2 (BT24-084 /
//!   AD1-021 idiom).
//! - Tamer [Security] play self.

#![allow(dead_code)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause};
use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const CARD_ID: &str = "EX13-069";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.level = Some(level);
    c.colors = vec![CardColor::Blue];
    c
}

/// Lv.4 blue that digivolves from a blue Lv.3 for 4.
fn evo(id: &str, name: &str) -> CardData {
    let mut c = digimon(id, name, 4);
    c.dp = Some(6000);
    c.play_cost = 6;
    c.evo_costs = vec![EvoCost {
        card_color: 1,
        level: 3,
        memory_cost: 4,
    }];
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-069 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("BASE-L3", "Base Three", 3))
        .add_card(digimon("VEEMON-X", "Veemon X", 3))
        .add_card(digimon("VEEDRAMON-Y", "Veedramon Y", 4))
        .add_card(evo("VEEDRAMON-EVO", "Veedramon Evo"))
        .add_card(evo("PLAIN-EVO", "Plain Evo"))
        .add_card(digimon("DRAW", "Drawn", 3))
        .add_card(digimon("FILL", "Fill", 3))
}

fn hand_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    runner.game.players[player as usize]
        .hand
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

fn top_id(runner: &DebugRunner, h: PermanentHandle) -> String {
    runner.game.players[h.player as usize].battle_area[h.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

fn suspended(runner: &DebugRunner, h: PermanentHandle) -> bool {
    runner.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

fn hand_index(runner: &DebugRunner, id: &str) -> usize {
    runner.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} must be in hand"))
}

fn triggered(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn fire_start_of_main(runner: &mut DebugRunner, rina: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(rina));
    runner.game.drain_effect_queue();
    let _ = runner.auto_resolve();
}

/// Accept the optional pre-cost confirm.
fn accept_confirm(runner: &mut DebugRunner) {
    let view = runner.pending_selection_view().expect("pre-cost confirm pending");
    assert!(view.is_optional, "\"by suspending this Tamer\" is optional: {view:?}");
    let accept = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("accept");
    runner.execute_action(view.selecting_player, accept).expect("accept");
}

fn pick_own(runner: &mut DebugRunner, h: PermanentHandle) {
    let view = runner.pending_selection_view().expect("own-Digimon prompt");
    assert_eq!(view.kind, SelectionKind::OwnField, "{view:?}");
    assert!(view.is_optional, "\"may digivolve\"");
    let action = digimon_engine::action::space::ATTACK_START + h.index as u16;
    runner.execute_action(view.selecting_player, action).expect("pick own");
}

fn hand_offered(runner: &DebugRunner, id: &str) -> bool {
    let Some(view) = runner.pending_selection_view() else {
        return false;
    };
    if view.kind != SelectionKind::Hand {
        return false;
    }
    let slot = hand_index(runner, id) as u16;
    view.valid_action_ids.contains(&(PLAY_HAND_START + slot))
        || view.valid_action_ids.contains(&(HAND_EFFECT_START + slot))
}

fn pick_hand(runner: &mut DebugRunner, id: &str) {
    let view = runner.pending_selection_view().expect("hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand, "{view:?}");
    let slot = hand_index(runner, id) as u16;
    let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
        .into_iter()
        .find(|a| view.valid_action_ids.contains(a))
        .unwrap_or_else(|| panic!("{id} must be selectable: {view:?}"));
    runner.execute_action(view.selecting_player, action).expect("pick hand");
}

/// Rina + a suspended own Digimon on the field, `hand` in hand. Deck top = DRAW.
fn unsuspend_setup(hand: &[&str]) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "DRAW"])
        .deck(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    let rina = runner.place_on_field(0, CARD_ID, Some(0));
    let digi = runner.place_on_field(0, "BASE-L3", Some(0));
    runner.game.players[0].battle_area[digi.index as usize].is_suspended = true;
    (runner, rina, digi)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_069_clause_shape_matches_printed_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.traits, vec!["CS".to_string()]);
    let t = triggered(&runner);
    assert_eq!(t.len(), 3, "start-of-main + unsuspend observer + [Security]");
    let som = t
        .iter()
        .find(|c| c.when == vec![CompiledTiming::StartOfYourMainPhase])
        .expect("[Start of Your Main Phase]");
    assert!(!som.optional);
    let obs = t
        .iter()
        .find(|c| c.when == vec![CompiledTiming::OnUnsuspend])
        .expect("unsuspend observer");
    assert!(obs.optional, "suspend-cost activation is optional");
    assert!(!obs.once_per_turn, "no [Once Per Turn] printed");
    assert!(t.iter().any(|c| c.when == vec![CompiledTiming::OnSecurity]));
}

// ─── Section 2/3 — [Start of Your Main Phase] ────────────────────────────────

#[test]
fn ex13_069_start_of_main_gains_1_with_a_veemon_name_digimon() {
    let mut runner = builder().deck(0, &["FILL"; 3]).memory(2).start();
    runner.skip_mulligan();
    let rina = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "VEEMON-X", Some(0));
    let before = runner.memory();
    fire_start_of_main(&mut runner, rina);
    assert_eq!(runner.memory(), before + 1);
}

#[test]
fn ex13_069_start_of_main_gains_1_with_a_veedramon_name_digimon() {
    let mut runner = builder().deck(0, &["FILL"; 3]).memory(2).start();
    runner.skip_mulligan();
    let rina = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "VEEDRAMON-Y", Some(0));
    let before = runner.memory();
    fire_start_of_main(&mut runner, rina);
    assert_eq!(runner.memory(), before + 1);
}

#[test]
fn ex13_069_start_of_main_no_gain_without_a_veemon_or_veedramon() {
    let mut runner = builder().deck(0, &["FILL"; 3]).memory(2).start();
    runner.skip_mulligan();
    let rina = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "BASE-L3", Some(0));
    let before = runner.memory();
    fire_start_of_main(&mut runner, rina);
    assert_eq!(runner.memory(), before);
}

// ─── Section 2/3 — [Your Turn] unsuspend observer ────────────────────────────

#[test]
fn ex13_069_unsuspend_draws_then_digivolves_into_veedramon_for_2_less() {
    let (mut runner, rina, digi) = unsuspend_setup(&["VEEDRAMON-EVO"]);
    runner.game.unsuspend(digi);
    accept_confirm(&mut runner);
    assert!(suspended(&runner, rina), "cost: this Tamer suspended");
    assert!(hand_ids(&runner, 0).contains(&"DRAW".to_string()), "<Draw 1>");
    let before = runner.memory();
    pick_own(&mut runner, digi);
    pick_hand(&mut runner, "VEEDRAMON-EVO");
    let _ = runner.auto_resolve();
    assert_eq!(top_id(&runner, digi), "VEEDRAMON-EVO");
    assert_eq!(before - runner.memory(), 2, "cost 4 reduced by 2");
}

#[test]
fn ex13_069_only_veedramon_name_digimon_cards_are_offered() {
    let (mut runner, _rina, digi) = unsuspend_setup(&["VEEDRAMON-EVO", "PLAIN-EVO"]);
    runner.game.unsuspend(digi);
    accept_confirm(&mut runner);
    pick_own(&mut runner, digi);
    assert!(hand_offered(&runner, "VEEDRAMON-EVO"));
    assert!(!hand_offered(&runner, "PLAIN-EVO"), "no [Veedramon] in its name");
}

#[test]
fn ex13_069_digivolve_is_optional_but_the_draw_happens() {
    let (mut runner, rina, digi) = unsuspend_setup(&["VEEDRAMON-EVO"]);
    runner.game.unsuspend(digi);
    accept_confirm(&mut runner);
    let view = runner.pending_selection_view().expect("own-Digimon prompt");
    assert!(view.is_optional);
    runner.execute_action(view.selecting_player, PASS).expect("decline digivolve");
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, rina));
    assert!(hand_ids(&runner, 0).contains(&"DRAW".to_string()));
    assert_eq!(top_id(&runner, digi), "BASE-L3");
}

#[test]
fn ex13_069_draw_still_happens_with_no_veedramon_in_hand() {
    let (mut runner, rina, digi) = unsuspend_setup(&["PLAIN-EVO"]);
    runner.game.unsuspend(digi);
    accept_confirm(&mut runner);
    let _ = runner.auto_resolve();
    assert!(suspended(&runner, rina));
    assert!(hand_ids(&runner, 0).contains(&"DRAW".to_string()));
    assert_eq!(top_id(&runner, digi), "BASE-L3");
}

#[test]
fn ex13_069_declining_the_cost_skips_everything() {
    let (mut runner, rina, digi) = unsuspend_setup(&["VEEDRAMON-EVO"]);
    runner.game.unsuspend(digi);
    let view = runner.pending_selection_view().expect("confirm");
    runner.execute_action(view.selecting_player, PASS).expect("decline");
    let _ = runner.auto_resolve();
    assert!(!suspended(&runner, rina), "cost not paid");
    assert_eq!(hand_ids(&runner, 0), vec!["VEEDRAMON-EVO"], "no draw (Q&A)");
}

#[test]
fn ex13_069_suspended_rina_cannot_pay_the_cost() {
    let (mut runner, rina, digi) = unsuspend_setup(&["VEEDRAMON-EVO"]);
    runner.game.players[0].battle_area[rina.index as usize].is_suspended = true;
    runner.game.unsuspend(digi);
    assert!(runner.pending_selection().is_none());
    assert_eq!(hand_ids(&runner, 0), vec!["VEEDRAMON-EVO"]);
}

#[test]
fn ex13_069_opponent_digimon_unsuspending_does_not_trigger() {
    let (mut runner, _rina, _digi) = unsuspend_setup(&["VEEDRAMON-EVO"]);
    let opp = runner.place_on_field(1, "BASE-L3", Some(0));
    runner.game.players[1].battle_area[opp.index as usize].is_suspended = true;
    runner.game.unsuspend(opp);
    assert!(runner.pending_selection().is_none(), "only YOUR Digimon");
}

#[test]
fn ex13_069_does_not_trigger_on_the_opponents_turn() {
    let (mut runner, _rina, digi) = unsuspend_setup(&["VEEDRAMON-EVO"]);
    runner.end_turn();
    let _ = runner.auto_resolve();
    runner.game.players[0].battle_area[digi.index as usize].is_suspended = true;
    runner.game.unsuspend(digi);
    assert!(runner.pending_selection().is_none(), "[Your Turn]");
}

// ─── [Security] ──────────────────────────────────────────────────────────────

#[test]
fn ex13_069_security_plays_itself_for_free() {
    let mut runner = builder()
        .deck(0, &["FILL"; 3])
        .deck(1, &["FILL"; 3])
        .security(1, &[CARD_ID])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let attacker = runner.place_on_field(0, "VEEDRAMON-Y", Some(0));
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();
    let opp_field: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(opp_field, vec![CARD_ID], "Rina played from security");
    assert_eq!(runner.memory(), 3, "without paying the cost");
}
