//! BT25-087 Thomas H. Norstein — Tamer, Blue, Cost 4. Traits: DATA SQUAD.
//!
//! Near-clone of BT25-088 Kyo Sawashiro. Clauses:
//!   1. [Start of Your Turn] memory <= 2 -> set 3.
//!   2. [All Turns] on effect-add to opponent's hand -> may suspend self ->
//!      place top 2 deck cards face down under self.
//!   3. [Your Turn][OPT] when any ally digivolves into a [DATA SQUAD] Digimon,
//!      by trashing a bottom FD card under a Tamer, reduce the cost by 1
//!      (the digivolve interactive cost-reduction path closed in #644;
//!      mechanism proven by ST23-03/11 + tests/cost_hooks/interactive_digivolve_reducer.rs).
//!   4. [Security] play self.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-087";

fn thomas() -> CardData {
    card_data_from_compiled(CARD_ID)
}
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = 3;
    c
}

#[test]
fn bt25_087_compiles_as_data_squad_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "DATA SQUAD"));
}

#[test]
fn bt25_087_has_all_four_clauses() {
    let card = compiled(CARD_ID);
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::StartOfYourTurn]),
        "start-of-turn memory clause"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnAddToHand]),
        "on-add-to-hand place-FD clause"
    );
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "[Security] play-self clause"
    );
    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        has_cr,
        "the digivolve-into-DATA-SQUAD cost-reduction clause must compile"
    );
}

/// Clause 1 — [Start of Your Turn] memory <= 2 -> set 3 (condition-gated).
#[test]
fn bt25_087_start_of_turn_sets_low_memory_to_three() {
    let mut runner = DebugRunner::builder()
        .add_card(thomas())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(1)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 1;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 3, "memory <=2 set to 3");
}

/// Clause 1 negative — memory > 2 is unchanged (the condition gates it).
#[test]
fn bt25_087_start_of_turn_leaves_high_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(thomas())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 5])
        .deck(1, &["FILLER"; 5])
        .memory(5)
        .start();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.memory = 5;
    runner.end_turn();
    runner.end_turn();
    let _ = runner.auto_resolve();
    assert_eq!(runner.memory(), 5, "memory >2 unchanged");
}
