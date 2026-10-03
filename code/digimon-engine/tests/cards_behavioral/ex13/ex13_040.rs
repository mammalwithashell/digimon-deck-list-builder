//! EX13-040 Mikemon — Digimon, Lv.4, Green, DP 5000, Cost 4.
//! Traits: Beast. Form: Champion. Attribute: Data.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-040.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-040.md` agrees)
//!
//! ```text
//! Digivolve: Green Lv.3 / cost 2
//!
//! [On Play] [When Digivolving] 1 of your opponent's Digimon or Tamers can't
//! unsuspend until their turn ends.
//! [All Turns] When this Digimon suspends, suspend 1 of your opponent's
//! Digimon or Tamers.
//!
//! Inherited Effect:
//! [All Turns] All of your suspended Digimon get +1000 DP.
//! ```
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_040.cs` at `b9a0638cd`. Idioms reused:
//! EX12-063 (select 1 opponent Digimon/Tamer + CannotUnsuspend until end of
//! their turn), BT23-058 (self-scoped `on_suspend`), BT25-050 (inherited DP
//! aura over own Digimon).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - [On Play]/[When Digivolving] targeted opponent lock (CannotUnsuspend,
//!   expiry end of opponent's turn).
//! - Self-scoped [All Turns] on_suspend trigger → suspend 1 opponent
//!   Digimon/Tamer.
//! - Inherited static aura gated on the target's suspended state.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::{encode_digivolve, ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-040";

fn digimon(id: &str, level: u8, dp: i32, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![color];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 2;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-040 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("OPP-A", 4, 5000, CardColor::Red))
        .add_card(digimon("OPP-B", 4, 4000, CardColor::Red))
        .add_card(tamer("OPP-TAMER"))
        .add_card(tamer("OWN-TAMER"))
        .add_card(digimon("GREEN-L3", 3, 3000, CardColor::Green))
        .add_card(digimon("ALLY", 4, 4000, CardColor::Green))
        .add_card(digimon("CARRIER", 5, 6000, CardColor::Green))
        .add_card(digimon("FILL", 3, 1000, CardColor::Red))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["FILL"; 3])
        .security(1, &["FILL"; 3])
}

fn pick_field(r: &mut DebugRunner, h: PermanentHandle) {
    let view = r.pending_selection_view().expect("field prompt pending");
    let id = ATTACK_START + h.index as u16;
    assert!(view.valid_action_ids.contains(&id), "{h:?} not offered: {view:?}");
    r.execute_action(view.selecting_player, id).expect("pick field");
}

fn offered_count(r: &DebugRunner) -> usize {
    let view = r.pending_selection_view().expect("prompt pending");
    view.valid_action_ids.iter().filter(|&&a| a != PASS).count()
}

fn suspended(r: &DebugRunner, h: PermanentHandle) -> bool {
    r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended
}

/// Suspend through the engine (fires on-suspend timing + re-ticks the
/// declarative auras); unsuspend is a raw flip followed by a re-tick.
fn set_suspended(r: &mut DebugRunner, h: PermanentHandle, v: bool) {
    if v {
        r.game.suspend(h);
    } else {
        r.game.players[h.player as usize].battle_area[h.index as usize].is_suspended = false;
    }
    r.game.tick_declarative_effects();
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_040_printed_metadata_and_digivolve_path() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!((c.level, c.dp, c.cost), (Some(4), Some(5000), Some(4)));
    assert_eq!(c.color, vec![CompiledColor::Green]);
    assert_eq!(c.traits, vec!["Beast".to_string()]);
    let digi: Vec<_> = c
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .collect();
    assert_eq!(digi.len(), 1);
    assert_eq!(digi[0].cost, Some(CompiledCost::Literal(2)));
}

#[test]
fn ex13_040_clause_shape() {
    let r = builder().start();
    let c = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 2);
    let lock = triggered
        .iter()
        .find(|t| t.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play][When Digivolving] clause");
    assert!(lock.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!lock.optional);
    let sus = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSuspend])
        .expect("[All Turns] on_suspend clause");
    assert!(!sus.optional);
    assert!(!sus.once_per_turn, "no [Once Per Turn]");
    assert_eq!(sus.scope, CompiledScope::FaceUp);

    let auras: Vec<_> = c
        .effects
        .iter()
        .filter_map(|e| match e {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura { scope, .. }) => {
                Some(*scope)
            }
            _ => None,
        })
        .collect();
    assert_eq!(auras, vec![CompiledScope::Inherited]);
}

// ─── Section 2/3 — [On Play][When Digivolving] can't-unsuspend lock ──────────

#[test]
fn ex13_040_on_play_offers_opponent_digimon_and_tamers_only() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.skip_mulligan();
    r.place_on_field(1, "OPP-A", Some(0));
    r.place_on_field(1, "OPP-TAMER", Some(0));
    r.place_on_field(0, "OWN-TAMER", Some(0));
    r.play(0, 0).expect("Mikemon played");
    let view = r.pending_selection_view().expect("target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(offered_count(&r), 2, "opponent's Digimon and Tamer");
    assert!(!view.valid_action_ids.contains(&PASS), "mandatory");
}

#[test]
fn ex13_040_on_play_lock_survives_opponents_unsuspend_phase_and_expires() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.skip_mulligan();
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    let other = r.place_on_field(1, "OPP-B", Some(0));
    set_suspended(&mut r, opp, true);
    set_suspended(&mut r, other, true);
    r.play(0, 0).expect("Mikemon played");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
    assert!(!r.modifiers().has(other, ModifierType::CannotUnsuspend));

    r.end_turn(); // → opponent's turn: unsuspend phase
    let _ = r.auto_resolve();
    assert!(suspended(&r, opp), "locked Digimon stays suspended");
    assert!(!suspended(&r, other), "the other one unsuspends normally");

    r.end_turn(); // opponent's turn ends → lock expires
    let _ = r.auto_resolve();
    assert!(
        !r.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "\"until their turn ends\""
    );
}

#[test]
fn ex13_040_when_digivolving_applies_lock() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.skip_mulligan();
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    let base = r.place_on_field(0, "GREEN-L3", Some(0));
    r.game.decode_action(encode_digivolve(0, base.index as u16), 0);
    assert_eq!(r.memory(), 8, "Green Lv.3 → cost 2");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert!(r.modifiers().has(opp, ModifierType::CannotUnsuspend));
}

#[test]
fn ex13_040_on_play_with_no_opponent_targets_is_noop() {
    let mut r = builder().hand(0, &[CARD_ID]).memory(10).start();
    r.skip_mulligan();
    r.play(0, 0).expect("Mikemon played");
    assert!(r.pending_selection().is_none());
}

// ─── Section 3 — [All Turns] when this Digimon suspends ──────────────────────

#[test]
fn ex13_040_attacking_suspends_mikemon_and_suspends_an_opponent_permanent() {
    let mut r = builder().memory(5).start();
    r.skip_mulligan();
    let mike = r.place_on_field(0, CARD_ID, Some(0));
    let opp_t = r.place_on_field(1, "OPP-TAMER", Some(0));
    r.place_on_field(1, "OPP-A", Some(0));
    let _ = r.attack_player(mike, 1, false);
    let view = r.pending_selection_view().expect("suspend-target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(offered_count(&r), 2, "Digimon or Tamers");
    pick_field(&mut r, opp_t);
    assert!(suspended(&r, opp_t), "opponent Tamer suspended");
}

#[test]
fn ex13_040_direct_suspend_fires_trigger() {
    let mut r = builder().memory(5).start();
    r.skip_mulligan();
    let mike = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    r.game.suspend(mike);
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert!(suspended(&r, opp));
}

#[test]
fn ex13_040_other_permanent_suspending_does_not_fire() {
    let mut r = builder().memory(5).start();
    r.skip_mulligan();
    r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    r.game.suspend(ally);
    let _ = r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert!(!suspended(&r, opp));
}

#[test]
fn ex13_040_suspend_trigger_fires_on_opponents_turn_too() {
    let mut r = builder().memory(5).start();
    r.skip_mulligan();
    let mike = r.place_on_field(0, CARD_ID, Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    r.end_turn();
    let _ = r.auto_resolve();
    r.game.suspend(mike);
    assert!(r.pending_selection().is_some(), "[All Turns]");
    pick_field(&mut r, opp);
    let _ = r.auto_resolve();
    assert!(suspended(&r, opp));
}

// ─── Section 3 — Inherited [All Turns] suspended Digimon +1000 DP ────────────

#[test]
fn ex13_040_inherited_suspended_digimon_get_plus_1000() {
    let mut r = builder().start();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    assert_eq!(r.effective_dp(carrier), Some(6000));
    assert_eq!(r.effective_dp(ally), Some(4000));
    set_suspended(&mut r, carrier, true);
    set_suspended(&mut r, ally, true);
    assert_eq!(r.effective_dp(carrier), Some(7000));
    assert_eq!(r.effective_dp(ally), Some(5000), "all of your suspended Digimon");
}

#[test]
fn ex13_040_inherited_does_not_buff_unsuspended_or_opponent_digimon() {
    let mut r = builder().start();
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let ally = r.place_on_field(0, "ALLY", Some(0));
    let opp = r.place_on_field(1, "OPP-A", Some(0));
    set_suspended(&mut r, opp, true);
    let _ = carrier;
    assert_eq!(r.effective_dp(ally), Some(4000), "unsuspended: no buff");
    assert_eq!(r.effective_dp(opp), Some(5000), "opponent's Digimon: no buff");
}

#[test]
fn ex13_040_inherited_not_active_face_up() {
    let mut r = builder().start();
    let mike = r.place_on_field(0, CARD_ID, Some(0));
    let ally = r.place_on_field(0, "ALLY", Some(0));
    set_suspended(&mut r, mike, true);
    set_suspended(&mut r, ally, true);
    assert_eq!(r.effective_dp(mike), Some(5000));
    assert_eq!(r.effective_dp(ally), Some(4000));
}
