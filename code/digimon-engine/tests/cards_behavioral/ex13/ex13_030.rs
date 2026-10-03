//! EX13-030 Reppamon — Digimon, Lv.4, Yellow, DP 5000, Cost 5.
//! Traits: Holy Beast / DATA SQUAD. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-030.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-030.md` agrees)
//!
//! ```text
//! Digivolve: Yellow Lv.3 / cost 2;  [Digivolve] Lv.3 w/[DATA SQUAD] trait: Cost 2
//!
//! <Barrier> (When this Digimon would be deleted in battle, by trashing your
//! top security card, it isn't deleted.)
//! [On Play] [When Digivolving] [When Attacking] [Once Per Turn] By trashing
//! your top security card, you may play 1 [Richard Sampson] from your hand or
//! trash without paying the cost.
//!
//! Inherited Effect:
//! <Barrier>
//! ```
//! Official Q&A: "Yes, you can." (the optional processing condition may be
//! paid even when no [Richard Sampson] can be played — rule 15-7-4).
//!
//! # DCGO C# reference
//! None — DCGO has no `EX13_030.cs` at `b9a0638cd`. BT23-035 Dynasmon
//! (`optional` + `outer_prompt` security-trash cost) and BT25-096 /
//! BT19-099 (`select_union_zone` hand-or-trash → `play_union_bound_free`) are
//! the YAML idioms reused.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - H Barrier keyword (own + inherited).
//! - E2 optional processing condition (trash top security) + [Once Per Turn]
//!   across three timings.
//! - A union-zone (hand OR trash) named play, free.
//! - Event: trashing security fires OnLoseSecurity.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX13-030";
const RICHARD: &str = "EX13-071";

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 3;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-030 YAML parses, compiles and is in the embedded pack")
        .dsl_card(RICHARD)
        .expect("EX13-071 Richard Sampson is in the embedded pack")
        .add_card(digimon("FILL", 3, 1000))
        .add_card(digimon("SEC", 3, 1000))
        .add_card(digimon("BASE-L3", 3, 3000))
        .add_card(digimon("CARRIER", 5, 5000))
        .add_card(digimon("BIG", 6, 12000))
}

fn runner_with(hand: &[&str]) -> DebugRunner {
    builder()
        .hand(0, hand)
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["SEC", "SEC", "SEC"])
        .security(1, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start()
}

fn hand_index(r: &DebugRunner, id: &str) -> usize {
    r.game.players[0]
        .hand
        .iter()
        .position(|c| c.card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} in hand"))
}

fn on_field(r: &DebugRunner, player: u8, id: &str) -> bool {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == id)
}

fn find_perm(r: &DebugRunner, id: &str) -> PermanentHandle {
    let i = r.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&r.game.card_data) == id)
        .unwrap_or_else(|| panic!("{id} on field"));
    PermanentHandle {
        player: 0,
        index: i as u8,
    }
}

fn inject_trash(r: &mut DebugRunner, id: &str) {
    r.inject_trash(0, id);
}

/// Resolve Richard Sampson's own [On Play] prompt (decline the optional
/// face-down placement) plus anything else pending, after it lands.
fn drain(r: &mut DebugRunner) {
    for _ in 0..12 {
        let Some(view) = r.pending_selection_view() else {
            return;
        };
        if view.is_optional {
            r.execute_action(view.selecting_player, PASS).expect("pass");
        } else {
            r.execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("first");
        }
    }
}

/// Play Reppamon from hand and accept its optional security-trash cost.
fn play_and_accept(r: &mut DebugRunner) {
    let idx = hand_index(r, CARD_ID);
    r.play(0, idx).expect("Reppamon played");
    let outer = r.pending_selection_view().expect("outer accept/decline");
    assert!(outer.is_optional, "the processing condition is optional");
    r.accept_optional_trigger().expect("accept");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_030_is_yellow_lv4_holy_beast_data_squad() {
    let r = runner_with(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.level, Some(4));
    assert_eq!(card.dp, Some(5000));
    assert!(card.traits.iter().any(|t| t == "Holy Beast"));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
    let digi = card
        .alt_paths
        .iter()
        .filter(|p| p.kind == CompiledAltPathKind::Digivolve)
        .count();
    assert_eq!(digi, 2, "yellow Lv.3 circle + Lv.3 w/[DATA SQUAD] condition");
}

#[test]
fn ex13_030_has_own_and_inherited_barrier() {
    let r = runner_with(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let barriers: Vec<CompiledScope> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
                keyword,
                scope,
                ..
            }) if keyword == "Barrier" => Some(*scope),
            _ => None,
        })
        .collect();
    assert!(barriers.contains(&CompiledScope::FaceUp), "own <Barrier>");
    assert!(barriers.contains(&CompiledScope::Inherited), "inherited <Barrier>");
}

#[test]
fn ex13_030_play_clause_shape() {
    let r = runner_with(&[]);
    let card = r.compiled_card(CARD_ID).expect("compiled");
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1);
    let t = triggered[0];
    assert_eq!(t.scope, CompiledScope::FaceUp);
    for w in [
        CompiledTiming::OnPlay,
        CompiledTiming::WhenDigivolving,
        CompiledTiming::WhenAttacking,
    ] {
        assert!(t.when.contains(&w), "missing {w:?}");
    }
    assert!(t.once_per_turn, "[Once Per Turn]");
    assert!(t.optional, "'By trashing…' optional processing condition");
}

// ════════════════════════════════════════════════════════════════════════════
// Section 2 — Condition gating (security available)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_030_with_security_offers_the_cost() {
    let mut r = runner_with(&[CARD_ID, RICHARD]);
    let idx = hand_index(&r, CARD_ID);
    r.play(0, idx).expect("played");
    assert!(r.pending_is_optional(), "outer accept/decline installs");
}

#[test]
fn ex13_030_without_security_does_not_offer_the_cost() {
    let mut r = builder()
        .hand(0, &[CARD_ID, RICHARD])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .memory(10)
        .start();
    let idx = hand_index(&r, CARD_ID);
    r.play(0, idx).expect("played");
    assert!(
        r.pending_selection().is_none(),
        "no security card to trash → the clause cannot be paid"
    );
    assert!(!on_field(&r, 0, RICHARD));
}

// ════════════════════════════════════════════════════════════════════════════
// Section 3 — Behavioral outcomes
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_030_on_play_trashes_security_and_plays_richard_from_hand_free() {
    let mut r = runner_with(&[CARD_ID, RICHARD]);
    let sec = r.security_count(0);
    let mem_before = r.memory();
    play_and_accept(&mut r);
    assert_eq!(r.security_count(0), sec - 1, "top security card trashed");
    assert!(
        r.pending_selection().is_some(),
        "the hand-or-trash Richard pick installs"
    );
    let view = r.pending_selection_view().unwrap();
    r.execute_action(0, view.valid_action_ids[0]).expect("pick Richard");
    drain(&mut r);
    assert!(on_field(&r, 0, RICHARD), "Richard Sampson played");
    assert_eq!(
        r.memory(),
        mem_before - 5,
        "only Reppamon's 5 paid; Richard (cost 4) played free"
    );
}

#[test]
fn ex13_030_can_play_richard_from_trash() {
    let mut r = runner_with(&[CARD_ID]);
    inject_trash(&mut r, RICHARD);
    play_and_accept(&mut r);
    let view = r.pending_selection_view().expect("trash pick");
    r.execute_action(0, view.valid_action_ids[0]).expect("pick Richard");
    drain(&mut r);
    assert!(on_field(&r, 0, RICHARD), "Richard Sampson played from trash");
    assert!(r.game.players[0]
        .trash
        .iter()
        .all(|c| c.card_id(&r.game.card_data) != RICHARD));
}

#[test]
fn ex13_030_may_decline_the_play_after_paying() {
    let mut r = runner_with(&[CARD_ID, RICHARD]);
    let sec = r.security_count(0);
    play_and_accept(&mut r);
    assert!(r.pending_is_optional(), "'you may play' is declinable");
    r.execute_action(0, PASS).expect("decline play");
    drain(&mut r);
    assert_eq!(r.security_count(0), sec - 1);
    assert!(!on_field(&r, 0, RICHARD));
}

#[test]
fn ex13_030_can_pay_even_without_richard_available() {
    // Official Q&A "Yes, you can." / rule 15-7-4.
    let mut r = runner_with(&[CARD_ID]);
    let sec = r.security_count(0);
    play_and_accept(&mut r);
    drain(&mut r);
    assert_eq!(r.security_count(0), sec - 1, "cost paid");
}

#[test]
fn ex13_030_declining_the_cost_keeps_security() {
    let mut r = runner_with(&[CARD_ID, RICHARD]);
    let sec = r.security_count(0);
    let idx = hand_index(&r, CARD_ID);
    r.play(0, idx).expect("played");
    r.decline_optional_trigger().expect("decline");
    drain(&mut r);
    assert_eq!(r.security_count(0), sec);
    assert!(!on_field(&r, 0, RICHARD));
}

#[test]
fn ex13_030_when_digivolving_fires() {
    let mut r = runner_with(&[CARD_ID, RICHARD]);
    let base = r.place_on_field(0, "BASE-L3", Some(0));
    let idx = hand_index(&r, CARD_ID);
    let mut ok = false;
    {
        let hand_card = r.game.players[0].hand[idx].handle();
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut r.game,
            hand_card,
            None,
            0,
        );
        ok = ctx.effect_initiated_digivolve_from_source(
            0,
            digimon_engine::enums::CardSourceRef::Hand(0, idx),
            base,
            digimon_engine::enums::CostDelta::Fixed(0),
            false,
        );
    }
    assert!(ok, "Reppamon digivolves onto a yellow Lv.3");
    assert!(
        r.pending_is_optional(),
        "[When Digivolving] offers the security-trash cost"
    );
}

/// Cost firing (§5 Section 4): no `GameEvent` variant records a security
/// trash, so the cost's side effect is asserted through the observer it must
/// fire — EX13-003 Kyaromon's inherited "[Your Turn] When your security stack
/// is removed from" (same DATA SQUAD slice) wakes up when Reppamon pays.
#[test]
fn ex13_030_paying_the_cost_fires_own_security_removed_observers() {
    let mut r = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-030")
        .dsl_card("EX13-003")
        .expect("EX13-003")
        .add_card(digimon("FILL", 3, 1000))
        .add_card(digimon("SEC", 3, 1000))
        .add_card(digimon("CARRIER-L3", 3, 3000))
        .add_card({
            let mut c = digimon("HB-L4", 4, 5000);
            c.traits = vec!["Holy Beast".to_string()];
            c.evo_costs = vec![EvoCost {
                card_color: 2,
                level: 3,
                memory_cost: 3,
            }];
            c
        })
        .hand(0, &[CARD_ID, "HB-L4"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .security(0, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    r.place_stack(0, &["EX13-003", "CARRIER-L3"]);
    play_and_accept(&mut r);
    // Reppamon's own "you may play Richard" pick (none available) may park
    // first; the Kyaromon observer's hand pick must surface either way.
    let mut saw_kyaromon = false;
    for _ in 0..6 {
        let Some(view) = r.pending_selection_view() else { break };
        if view.kind == SelectionKind::Hand {
            saw_kyaromon = true;
            break;
        }
        r.execute_action(view.selecting_player, PASS).expect("pass");
    }
    assert!(
        saw_kyaromon,
        "trashing security as Reppamon's cost fires on_own_security_removed"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Section 5 — OPT across the three timings
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_030_once_per_turn_blocks_when_attacking_after_on_play() {
    let mut r = runner_with(&[CARD_ID]);
    play_and_accept(&mut r);
    drain(&mut r);
    let rep = find_perm(&r, CARD_ID);
    r.game.players[0].battle_area[rep.index as usize].turn_played = 0;
    r.attack_player(rep, 1, false);
    assert!(
        r.pending_selection().is_none(),
        "[Once Per Turn]: [When Attacking] cannot reuse it this turn"
    );
}

#[test]
fn ex13_030_when_attacking_fires_on_a_later_turn() {
    let mut r = runner_with(&[CARD_ID]);
    play_and_accept(&mut r);
    drain(&mut r);
    r.end_turn();
    drain(&mut r);
    r.end_turn();
    drain(&mut r);
    assert_eq!(r.turn_player(), 0);
    let rep = find_perm(&r, CARD_ID);
    r.attack_player(rep, 1, false);
    assert!(
        r.pending_is_optional(),
        "OPT cleared: [When Attacking] offers the cost again"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// Barrier — own and inherited
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ex13_030_own_barrier_prevents_battle_deletion_by_trashing_security() {
    let mut r = runner_with(&[]);
    let rep = r.place_on_field(0, CARD_ID, Some(0));
    let big = r.place_on_field(1, "BIG", Some(0));
    r.game.players[1].battle_area[big.index as usize].is_suspended = true;
    let sec = r.security_count(0);
    r.attack_digimon(rep, big, false);
    // [When Attacking] cost prompt first — decline it.
    r.decline_optional_trigger().expect("decline WA cost");
    // Then <Barrier> would-delete prompt — accept.
    assert_eq!(r.pending_kind(), Some(SelectionKind::Replacement));
    r.accept_optional_trigger().expect("accept Barrier");
    drain(&mut r);
    assert!(on_field(&r, 0, CARD_ID), "Reppamon survives via <Barrier>");
    assert_eq!(r.security_count(0), sec - 1, "Barrier trashed 1 security");
}

#[test]
fn ex13_030_inherited_barrier_protects_the_carrier() {
    let mut r = runner_with(&[]);
    let carrier = r.place_stack(0, &[CARD_ID, "CARRIER"]);
    let big = r.place_on_field(1, "BIG", Some(0));
    r.game.players[1].battle_area[big.index as usize].is_suspended = true;
    let sec = r.security_count(0);
    r.attack_digimon(carrier, big, false);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Replacement),
        "inherited <Barrier> offers the save"
    );
    r.accept_optional_trigger().expect("accept Barrier");
    drain(&mut r);
    assert!(on_field(&r, 0, "CARRIER"), "carrier survives");
    assert_eq!(r.security_count(0), sec - 1);
}
