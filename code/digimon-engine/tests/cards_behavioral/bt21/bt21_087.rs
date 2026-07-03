//! BT21-087 Zenith — Tamer, Black, Cost 4.
//! Traits: LIBERATOR
//!
//! # Printed card text (card image — authoritative)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [On Play] Reveal the top 3 cards of your deck. Among them, play 1
//!   [Vemmon] without paying the cost or add 1 card with [Vemmon] in its
//!   text to the hand. Trash the rest.
//! [Security] Play this card without paying the cost.
//!
//! # Gaps and omissions
//! Clause 2 [On Play] is OMITTED entirely. Faithfully implementing it needs a
//! predicate that reads the NAME of an already-bound reveal-pick `Card`
//! binding inside a downstream `if:` (to branch the Play-vs-Add-to-hand
//! choice on whether the ONE card selected from the reveal pool happens to
//! be exactly named "Vemmon", vs. merely referencing "[Vemmon]" in its
//! printed text) — e.g. `binding_card_name_is`, analogous to the existing
//! `binding_card_kind` leaf. VERIFIED ABSENT from the DSL (2026-07-02) —
//! see the full crosscheck + rejected-workaround list in
//! `code/digimon-engine/cards/bt21/BT21-087.yaml`.
//! Gap: G-DSL-BINDING-CARD-NAME-EQUALS — see qa/dsl-vocab-gaps.md.
//!
//! # Verdict: PARTIAL
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Black/BT21_087.cs

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledCardKind, CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, EffectTiming, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-087";

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-087 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Structural tests ─────────────────────────────────────────────────────────

#[test]
fn bt21_087_yaml_printed_metadata() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let card = runner.compiled_card(CARD_ID).expect("present in pack");
    assert_eq!(card.name, "Zenith");
    assert_eq!(card.kind, CompiledCardKind::Tamer);
}

#[test]
fn bt21_087_has_exactly_two_triggered_clauses_start_of_turn_and_security() {
    // Clause 2 [On Play] is BLOCKED and intentionally omitted (see file
    // header + the YAML header docstring) — only Clause 1 (Start of Your
    // Turn) and Clause 3 (Security) are shipped.
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner.compiled_card(CARD_ID).expect("present in pack");

    let triggered: Vec<&digimon_dsl::compiled::CompiledTriggeredClause> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "BT21-087 ships exactly two triggered clauses (Start of Turn, Security); \
         [On Play] is BLOCKED on G-DSL-BINDING-CARD-NAME-EQUALS and intentionally omitted"
    );

    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "Start of Your Turn clause must be present"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "Security clause must be present"
    );
    assert!(
        !triggered.iter().any(|t| t.when.contains(&CompiledTiming::OnPlay)),
        "On Play clause must NOT be present — it is BLOCKED, not stubbed"
    );
}

// ─── Clause 1: memory ramp ────────────────────────────────────────────────────

#[test]
fn bt21_087_start_of_turn_sets_memory_to_3_when_at_0() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(0).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 0 (<=2) → must be set to 3");
}

#[test]
fn bt21_087_start_of_turn_sets_memory_to_3_when_at_2() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(2).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 2 (<=2) → must be set to 3");
}

#[test]
fn bt21_087_start_of_turn_does_not_change_memory_when_at_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(3).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(r.game.memory, 3, "memory was 3 (>2) → must remain 3");
}

#[test]
fn bt21_087_start_of_turn_does_not_change_memory_when_above_3() {
    let mut r = base().deck(0, &["DECK-PAD"; 12]).memory(5).start();
    let zenith = r.place_on_field(0, CARD_ID, Some(0));
    let handle = r.perm_handle(0, zenith.index as usize);
    r.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(handle),
    );
    r.game.drain_effect_queue();
    assert_eq!(
        r.game.memory, 5,
        "memory was 5 (>2) → must remain unchanged"
    );
}

// ─── Clause 3: security play ──────────────────────────────────────────────────

/// Structural test: the `on_security` clause compiles with a `PlayFromSecurity`
/// step. The behavioral runtime for play-self-free security is well-covered by
/// BT18-087, BT21-084, BT22-084, BT21-015; here we only assert the clause is
/// present so security checks can route to it correctly.
#[test]
fn bt21_087_security_clause_has_play_from_security_step() {
    let runner = base().deck(0, &["DECK-PAD"; 12]).start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT21-087 in compiled_cards");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity))
        .expect("on_security clause must exist on BT21-087");

    let has_play_from_security = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::PlayFromSecurity));
    assert!(
        has_play_from_security,
        "on_security clause must lower to a PlayFromSecurity step"
    );
}
