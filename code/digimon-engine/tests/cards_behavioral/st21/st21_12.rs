//! ST21-12 Joe Kido & Mimi Tachikawa — Tamer, Blue+Green, Cost 3,
//! traits: [ADVENTURE].
//!
//! # Card text (cards.json / official Bandai DB bundle)
//!
//! **[Start of Your Main Phase]** If you have a Digimon with the
//! [ADVENTURE] trait, gain 1 memory.
//!
//! **[Your Turn]** When any Digimon cards with the [ADVENTURE] trait would
//! be played from your hand, by suspending this Tamer, reduce the play cost
//! by 1.
//!
//! **Inherited (Security):** [Security] Play this card without paying the
//! cost.
//!
//! # DCGO C# reference
//! `DCGO/Assets/Scripts/CardEffect/ST21/Blue/ST21_12.cs`
//!
//! - `EffectTiming.OnStartMainPhase`, mandatory (`isOptional: false`), gated
//!   on `IsOwnerTurn` + owner has a battle-area Digimon with
//!   `HasAdventureTraits`.
//! - `EffectTiming.BeforePayCost`, optional (`isOptional: true`); suspend this
//!   Tamer, then install a -1 `ChangeCostClass` for the owner's play.
//! - `EffectTiming.SecuritySkill` → `PlaySelfTamerSecurityEffect`.
//!
//! # Patterns this test covers
//!
//! - **Start-of-main memory gain with a board-state gate** (B-group, AD1-019
//!   sibling): positive, plus negatives for no Digimon, Tamer-only,
//!   non-ADVENTURE Digimon, opponent-only ADVENTURE Digimon, and opponent's
//!   turn. Memory movement is asserted through the event log
//!   (`MemoryChange.source_card_id`).
//! - **[Your Turn] cost reduction with `pay_cost: [suspend self]`** (ST21-13
//!   sibling): positive (memory + `Play` event cost + Tamer suspended),
//!   negatives for non-ADVENTURE Digimon / ADVENTURE Tamer / opponent's turn /
//!   already-suspended Tamer, optional decline, and the suspend-cost natural
//!   lockout (second ADVENTURE play in the same turn pays full price).
//! - **[Security] play self free** (structural + end-to-end through a security
//!   check).

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::events::GameEvent;

use crate::dsl_card_data::compiled;

const CARD_ID: &str = "ST21-12";

// ─── Card-data factories ─────────────────────────────────────────────────────

fn make_filler(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

/// A Lv-N Digimon with the ADVENTURE trait.
fn make_adventure_digimon(card_id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

/// A Lv-N Digimon WITHOUT the ADVENTURE trait.
fn make_non_adventure_digimon(card_id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(5000);
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.traits = vec![];
    c
}

/// A Tamer with the ADVENTURE trait (must satisfy neither the "Digimon with
/// the ADVENTURE trait" gate nor the Digimon-only cost-reduction filter).
fn make_adventure_tamer(card_id: &str, name: &str, cost: u16) -> CardData {
    let mut c = make_test_card(card_id, name);
    c.card_kind = CardKind::Tamer;
    c.play_cost = cost;
    c.colors = vec![CardColor::Blue];
    c.traits = vec!["ADVENTURE".to_string()];
    c
}

/// Accept the first non-PASS legal action at every pending selection until
/// the engine settles.
fn drive_accept(runner: &mut DebugRunner, player: u8) {
    while let Some(view) = runner.pending_selection_view() {
        let accept = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&id| id != PASS)
            .unwrap_or(view.valid_action_ids[0]);
        runner
            .execute_action(player, accept)
            .expect("execute pending selection");
    }
}

/// Count `MemoryChange` events attributed to ST21-12 since `checkpoint`.
fn tamer_memory_events(runner: &DebugRunner, checkpoint: usize) -> Vec<i16> {
    runner
        .events_since(checkpoint)
        .iter()
        .filter_map(|e| match e {
            GameEvent::MemoryChange {
                delta,
                source_card_id: Some(id),
                ..
            } if id.as_str() == CARD_ID => Some(*delta),
            _ => None,
        })
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// ST21-12 compiles as a Blue+Green Tamer, cost 3, ADVENTURE trait.
#[test]
fn st21_12_compiles_as_tamer_cost_3_with_adventure_trait() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.name, "Joe Kido & Mimi Tachikawa");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(3), "ST21-12 prints Cost 3");
    assert_eq!(
        card.color.len(),
        2,
        "ST21-12 is Blue+Green (two printed colours); got {:?}",
        card.color
    );
    assert!(
        card.traits
            .iter()
            .any(|t| t.eq_ignore_ascii_case("ADVENTURE")),
        "ST21-12 must carry the ADVENTURE trait"
    );
}

/// Three clauses total: start-of-main (triggered), cost_reduction
/// (declarative), on_security (triggered).
#[test]
fn st21_12_has_two_triggered_and_one_cost_reduction() {
    let card = compiled(CARD_ID);
    assert_eq!(card.effects.len(), 3, "expected exactly 3 clauses");

    let triggered = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(_)))
        .count();
    assert_eq!(triggered, 2, "start_of_your_main_phase + on_security");

    let cr = card
        .effects
        .iter()
        .filter(|c| {
            matches!(
                c,
                CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
            )
        })
        .count();
    assert_eq!(cr, 1, "exactly one cost_reduction declarative");
}

/// Clause 1: start_of_your_main_phase, mandatory, gain_memory 1, FaceUp.
#[test]
fn st21_12_clause1_start_of_main_mandatory_gain_1_memory() {
    let card = compiled(CARD_ID);
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when == vec![CompiledTiming::StartOfYourMainPhase] =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("ST21-12 must have a start_of_your_main_phase clause");

    assert_eq!(t.scope, CompiledScope::FaceUp);
    assert!(
        !t.optional,
        "[Start of Your Main Phase] gain 1 memory is mandatory (DCGO isOptional: false)"
    );
    assert!(!t.once_per_turn, "no OPT is printed");
    assert_eq!(
        t.process,
        vec![CompiledStep::GainMemory(1)],
        "body must be exactly gain_memory 1"
    );
}

/// Clause 2: cost_reduction — optional, your-turn gated, amount 1, with an
/// ally-played predicate and a non-empty pay_cost (suspend self).
#[test]
fn st21_12_clause2_cost_reduction_optional_your_turn_amount_1() {
    let card = compiled(CARD_ID);
    let (scope, active_when, optional, once_per_turn, amount, when_any_ally, pay_cost) = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                scope,
                active_when,
                optional,
                once_per_turn,
                amount,
                when_any_ally_played,
                pay_cost,
                ..
            }) => Some((
                *scope,
                active_when.clone(),
                *optional,
                *once_per_turn,
                *amount,
                when_any_ally_played.clone(),
                pay_cost.clone(),
            )),
            _ => None,
        })
        .expect("ST21-12 must have a cost_reduction clause");

    assert_eq!(scope, CompiledScope::FaceUp);
    assert!(
        optional,
        "the suspend cost is opt-in (DCGO isOptional: true; printed 'by suspending')"
    );
    assert!(
        !once_per_turn,
        "no [Once Per Turn] printed; the suspend cost self-limits"
    );
    assert_eq!(amount, Some(1), "printed 'reduce the play cost by 1'");
    assert!(
        when_any_ally.is_some(),
        "when_any_ally_played (Digimon + ADVENTURE) must be present"
    );
    assert!(
        active_when.is_some_and(|p| p.your_turn == Some(true)),
        "active_when must gate to [Your Turn]"
    );
    assert!(!pay_cost.is_empty(), "pay_cost must suspend this Tamer");
}

/// Clause 3: on_security, FaceUp, mandatory, play_from_security.
#[test]
fn st21_12_clause3_on_security_plays_self_mandatory() {
    let card = compiled(CARD_ID);
    let t = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnSecurity] => Some(t),
            _ => None,
        })
        .expect("ST21-12 must have an on_security clause");

    assert_eq!(t.scope, CompiledScope::FaceUp);
    assert!(!t.optional, "[Security] play-self is mandatory");
    assert!(
        t.process
            .iter()
            .any(|s| matches!(s, CompiledStep::PlayFromSecurity)),
        "body must be play_from_security; got {:?}",
        t.process
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause 1: [Start of Your Main Phase] gain 1 memory
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: own ADVENTURE Digimon on the battle area → +1 memory at the
/// start of the main phase, attributed to ST21-12 in the event log.
#[test]
fn st21_12_start_of_main_gains_1_memory_with_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-DIGI", "Adv Digimon", 4, 4))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ADV-DIGI", Some(0));
    runner.game.memory = 0;

    let cp = runner.event_checkpoint();
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        1,
        "P0 must gain 1 memory with an ADVENTURE Digimon on the field"
    );
    assert_eq!(
        tamer_memory_events(&runner, cp),
        vec![1],
        "exactly one +1 MemoryChange attributed to ST21-12"
    );
}

/// Negative: no Digimon at all → no memory.
#[test]
fn st21_12_start_of_main_no_gain_without_any_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 0;

    let cp = runner.event_checkpoint();
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 0, "no Digimon → no memory gain");
    assert!(tamer_memory_events(&runner, cp).is_empty());
}

/// Negative: own Digimon WITHOUT the ADVENTURE trait does not satisfy the gate.
#[test]
fn st21_12_start_of_main_no_gain_with_non_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_non_adventure_digimon("PLAIN", "Plain Digimon", 4, 4))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "PLAIN", Some(0));
    runner.game.memory = 0;

    let cp = runner.event_checkpoint();
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 0, "non-ADVENTURE Digimon → no gain");
    assert!(tamer_memory_events(&runner, cp).is_empty());
}

/// Negative: an ADVENTURE *Tamer* is not "a Digimon with the [ADVENTURE]
/// trait" (printed text says Digimon).
#[test]
fn st21_12_start_of_main_no_gain_with_only_adventure_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_tamer("ADV-TAMER", "Adv Tamer", 3))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ADV-TAMER", Some(0));
    runner.game.memory = 0;

    let cp = runner.event_checkpoint();
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(runner.memory(), 0, "an ADVENTURE Tamer is not a Digimon");
    assert!(tamer_memory_events(&runner, cp).is_empty());
}

/// Negative: only the OPPONENT controls an ADVENTURE Digimon ("if YOU have").
#[test]
fn st21_12_start_of_main_no_gain_when_only_opponent_has_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("OPP-ADV", "Opp Adv", 4, 4))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "OPP-ADV", Some(0));
    runner.game.memory = 0;

    let cp = runner.event_checkpoint();
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        0,
        "the opponent's ADVENTURE Digimon must not satisfy 'if you have'"
    );
    assert!(tamer_memory_events(&runner, cp).is_empty());
}

/// Negative: the clause is [Start of YOUR Main Phase] — on the opponent's
/// main phase start, ST21-12 must not fire even with an ADVENTURE Digimon.
#[test]
fn st21_12_start_of_main_does_not_fire_on_opponents_main_phase() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-DIGI", "Adv Digimon", 4, 4))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .memory(0)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(0, "ADV-DIGI", Some(0));

    let cp = runner.event_checkpoint();
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: P1's turn");
    runner.game.enter_main_phase();
    let _ = runner.auto_resolve();

    assert!(
        tamer_memory_events(&runner, cp).is_empty(),
        "ST21-12 must not gain memory at the start of the OPPONENT's main phase"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause 2: [Your Turn] suspend self → -1 cost on ADVENTURE Digimon
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: ADVENTURE Digimon (cost 5) is played from hand → the optional
/// reduction prompt installs; accepting suspends ST21-12 and the Digimon
/// costs 4. Memory and the `Play` event both reflect the discount.
#[test]
fn st21_12_cost_reduction_fires_on_adventure_digimon_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-DIGI", "Adv Digimon", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-DIGI"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "precondition: ST21-12 starts unsuspended"
    );

    let mem_before = runner.memory();
    let cp = runner.event_checkpoint();
    let _ = runner.play(0, 0);

    assert!(
        runner.game.pending_selection.is_some(),
        "an optional cost-reduction prompt must install"
    );
    assert!(
        runner.pending_is_optional(),
        "the reduction prompt must be optional (PASS-able)"
    );

    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(
        mem_before - runner.memory(),
        4,
        "ADV-DIGI costs 5 - 1 = 4 with ST21-12's reduction"
    );

    // Event-log cost assertions: the Play event records the reduced cost.
    let play = runner
        .events_since(cp)
        .iter()
        .find_map(|e| match e {
            GameEvent::Play {
                card_id,
                cost_paid,
                cost_printed,
                ..
            } if card_id == "ADV-DIGI" => Some((*cost_paid, *cost_printed)),
            _ => None,
        })
        .expect("a Play event for ADV-DIGI must be logged");
    assert_eq!(play, (4, 5), "Play event: cost_paid 4 of printed 5");

    let tamer_after = runner.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID)
        .expect("ST21-12 stays on the field (suspended, not deleted)");
    assert!(
        tamer_after.is_suspended,
        "paying the cost must suspend ST21-12"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "ADV-DIGI"),
        "ADV-DIGI must be on the field"
    );
}

/// Negative: a non-ADVENTURE Digimon pays full cost; the Tamer stays untapped.
#[test]
fn st21_12_cost_reduction_does_not_fire_on_non_adventure_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_non_adventure_digimon("NON-ADV", "NonAdv", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["NON-ADV"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "NON-ADV play succeeds with no reduction prompt"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "full printed cost 5");
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Tamer must stay unsuspended"
    );
}

/// Negative: an ADVENTURE Tamer is not a "Digimon card" — no reduction.
#[test]
fn st21_12_cost_reduction_does_not_fire_on_adventure_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_tamer("ADV-TAMER", "Adv Tamer", 3))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-TAMER"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "ADV-TAMER play succeeds with no reduction prompt"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 3, "full printed cost 3");
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "Tamer must stay unsuspended"
    );
}

/// Negative: on the opponent's turn the [Your Turn] reduction is gated off.
#[test]
fn st21_12_cost_reduction_does_not_fire_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("OPP-ADV", "Opp Adv", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER", "FILLER", "FILLER"])
        .hand(1, &["OPP-ADV"])
        .memory(0)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "precondition: P1's turn");

    let mem_before = runner.memory();
    let played = runner.play(1, 0);
    assert!(played.is_some(), "P1's play succeeds with no prompt");
    drive_accept(&mut runner, 1);
    let _ = runner.auto_resolve();

    assert_eq!(
        (runner.memory() - mem_before).abs(),
        5,
        "P1 pays the full printed cost 5"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "P0's Tamer must not be suspended by the opponent's play"
    );
}

/// Optional: declining (PASS) pays full cost and leaves the Tamer unsuspended.
#[test]
fn st21_12_cost_reduction_can_be_declined() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-DIGI", "Adv Digimon", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-DIGI"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "reduction prompt must install (declined below)"
    );

    while let Some(view) = runner.pending_selection_view() {
        let action = if view.is_optional {
            PASS
        } else {
            view.valid_action_ids
                .iter()
                .copied()
                .find(|&id| id != PASS)
                .unwrap_or(view.valid_action_ids[0])
        };
        runner
            .execute_action(0, action)
            .expect("execute pending selection (declining where optional)");
    }
    let _ = runner.auto_resolve();

    assert_eq!(
        mem_before - runner.memory(),
        5,
        "declined reduction → full cost 5"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "declining must not suspend the Tamer"
    );
}

/// Cost lockout: an already-suspended ST21-12 cannot pay the suspend cost, so
/// no reduction is offered and the full cost is paid.
#[test]
fn st21_12_cost_reduction_unavailable_while_tamer_is_suspended() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-DIGI", "Adv Digimon", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-DIGI"])
        .memory(10)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.players[0].battle_area[tamer.index as usize].is_suspended = true;

    let mem_before = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "a suspended Tamer cannot pay the cost → no reduction prompt, play resolves"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();

    assert_eq!(mem_before - runner.memory(), 5, "full printed cost 5");
}

/// Natural lockout: after the first ADVENTURE play suspends the Tamer, a
/// second ADVENTURE play in the same turn pays the full cost.
#[test]
fn st21_12_second_adventure_play_same_turn_pays_full_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_adventure_digimon("ADV-A", "Adv A", 4, 5))
        .add_card(make_adventure_digimon("ADV-B", "Adv B", 4, 5))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .deck(1, &["FILLER"])
        .hand(0, &["ADV-A", "ADV-B"])
        .memory(10)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));

    let mem_before = runner.memory();
    let _ = runner.play(0, 0);
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();
    assert_eq!(
        mem_before - runner.memory(),
        4,
        "first play is discounted (5 - 1)"
    );

    let mem_mid = runner.memory();
    let played = runner.play(0, 0);
    assert!(
        played.is_some(),
        "second play resolves with no reduction prompt (Tamer is suspended)"
    );
    drive_accept(&mut runner, 0);
    let _ = runner.auto_resolve();
    assert_eq!(
        mem_mid - runner.memory(),
        5,
        "second ADVENTURE play in the same turn pays the full cost"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause 3: [Security] play this card without paying the cost
// ═══════════════════════════════════════════════════════════════════════════════

/// End-to-end: when P0 attacks and flips ST21-12 as P1's security card, the
/// [Security] effect plays it onto P1's battle area for free.
#[test]
fn st21_12_security_effect_plays_self_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("ST21-12 in embedded DSL pack")
        .add_card(make_filler("ATTACKER"))
        .security(1, &[CARD_ID])
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let p1_field_before = runner.battle_area_size(1);
    let mem_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.battle_area_size(1),
        p1_field_before + 1,
        "ST21-12 must land on P1's battle area via its [Security] effect"
    );
    assert!(
        runner
            .game
            .player(1)
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == CARD_ID),
        "the card placed must be ST21-12"
    );
    assert_eq!(
        runner.memory(),
        mem_before,
        "the [Security] play is free — no memory spent"
    );
}

#[allow(dead_code)]
fn _unused_silencer() {
    let _ = make_filler("X");
    let _ = make_adventure_tamer("X", "X", 0);
}
