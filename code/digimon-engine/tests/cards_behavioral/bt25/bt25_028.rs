//! BT25-028 Dianamon — Digimon, Lv.6, Blue/White, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (card image BT25-028 — authoritative for printed text)
//! When this card would be played, if your opponent has a level 6 or higher
//! Digimon, reduce the cost by 5.
//! [On Play] [When Digivolving] None of your opponent's Digimon with 1 or fewer
//! digivolution cards can suspend until their turn ends. Then, delete 1 of your
//! opponent's unsuspended Digimon.
//! [All Turns] [Once Per Turn] When any Digimon are played or digivolve, you may
//! trash any 4 digivolution cards from your opponent's Digimon. Then, 2 of your
//! Digimon may DNA digivolve into [GraceNovamon] in the hand.
//! Inherited: [When Attacking] [Once Per Turn] 1 of your opponent's Digimon or
//! Tamers can't suspend until their turn ends.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_028.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - D2 conditional cost reduction; F7 cannot-suspend; mandatory delete
//! - G2 DNA digivolve into a hand card; inherited WA cannot-suspend (E2 OPT)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, ModifierType, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-028";

fn make_digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-028 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OPP-NOSRC", 5, 6000, &["Beast"]))
        .add_card(make_digimon("OPP-SRC", 4, 4000, &["Beast"]))
        .add_card(make_digimon("OPP-L6", 6, 11000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_028_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Dianamon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_028_has_cost_reduction() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(
        card.effects.iter().any(|c| matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )),
        "cost-reduction clause present"
    );
}

#[test]
fn bt25_028_op_wd_clause_deletes_and_is_mandatory() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving)
                    && t.process
                        .iter()
                        .any(|s| matches!(s, CompiledStep::DeletePermanent { .. })) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD delete clause present");
    assert!(!clause.optional, "the delete is mandatory (no 'may')");
}

#[test]
fn bt25_028_has_inherited_wa_cannot_suspend() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::WhenAttacking) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited WA clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(
        clause
            .process
            .iter()
            .any(|s| matches!(s, CompiledStep::AddModifier { .. })),
        "inherited WA applies a CannotSuspend modifier"
    );
}

#[test]
fn bt25_028_has_all_turns_dna_observer() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if (t.when.contains(&CompiledTiming::OnEnterFieldAnyone)
                    || t.when.contains(&CompiledTiming::OnDigivolve))
                    && t.optional
                    && t.once_per_turn =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("[All Turns] DNA observer present");
    assert!(clause.optional && clause.once_per_turn);
}

// NOTE: cost-reduction is verified structurally by `bt25_028_has_cost_reduction`.
// The debug runner's `play()` path does not route the printed play cost through
// the `BeforePayCost` cost-reduction scan (cf. BT25-018, whose test also only
// asserts the cost_reduction clause structurally), so a memory-delta behavioral
// check is not reliable here.

#[test]
fn bt25_028_locks_low_source_opp_and_offers_delete() {
    let mut runner = base().memory(12).start();
    let nosrc = runner.place_on_field(1, "OPP-NOSRC", Some(0));
    let diana = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, diana.index as usize);

    assert!(
        runner.modifiers().has(nosrc, ModifierType::CannotSuspend),
        "opp Digimon with <=1 sources gains CannotSuspend"
    );
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "delete-1-unsuspended-opp prompt installs"
    );
}

#[test]
fn bt25_028_op_wd_no_delete_prompt_without_opp() {
    let mut runner = base().memory(12).start();
    let diana = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, diana.index as usize);
    assert!(
        runner.pending_selection().is_none(),
        "no opp Digimon -> OP/WD clause is a no-op"
    );
}
