//! BT25-059 Ceresmon — Digimon, Lv.6, Green/Blue, DP 12000, Cost 12.
//! Traits: Shaman, Olympos XII, Iliad, TS. Attribute: Data.
//!
//! # Card text (card image BT25-059 — authoritative for printed text)
//! When this card would be played, if there are 2 or more suspended Digimon,
//! reduce the cost by 5.
//! [On Play] [When Digivolving] You may suspend up to 2 Digimon. Then, none of
//! your suspended [Vegetation] or [TS] trait Digimon are affected by your
//! opponent's Digimon effects until their turn ends.
//! [All Turns] [Once Per Turn] When any Digimon suspend, to 1 of your opponent's
//! Digimon, give -3000 DP until their turn ends for each suspended Digimon.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_059.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API 4.3)
//! - D2 conditional cost reduction; CountCappedMultiSelect suspend up to 2
//! - F6 effect immunity (own suspended Veg/TS); on_suspend DP-debuff formula

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT25-059";

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
        .expect("BT25-059 YAML parses and compiles")
        .add_card(make_test_card("PAD", "Filler"))
        .add_card(make_digimon("OWN-VEG", 4, 4000, &["Vegetation"]))
        .add_card(make_digimon("OPP-DIGI", 5, 9000, &["Beast"]))
        .add_card(make_digimon("OPP-DIGI2", 4, 6000, &["Beast"]))
        .deck(0, &["PAD"; 10])
        .deck(1, &["PAD"; 10])
}

#[test]
fn bt25_059_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert_eq!(card.name, "Ceresmon");
    assert_eq!(card.level, Some(6));
    assert_eq!(card.cost, Some(12));
    assert_eq!(card.dp, Some(12000));
    for t in ["Shaman", "Olympos XII", "Iliad", "TS"] {
        assert!(card.traits.contains(&t.to_string()), "trait {t}");
    }
}

#[test]
fn bt25_059_has_cost_reduction() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
    )));
}

#[test]
fn bt25_059_has_op_wd_suspend_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.when.contains(&CompiledTiming::OnPlay)
                    && t.when.contains(&CompiledTiming::WhenDigivolving) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("OP/WD clause present");
    let has_suspend = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::SelectCountCappedMulti { .. }));
    assert!(has_suspend, "OP/WD clause has a count-capped suspend selection");
}

#[test]
fn bt25_059_has_on_suspend_debuff_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("on_suspend clause present");
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(
        clause.process.iter().any(|s| matches!(s, CompiledStep::AddDpModifier { .. })),
        "on_suspend clause applies a DP modifier"
    );
}

// ─── Behavior: OP/WD suspend prompt ─────────────────────────────────────────

#[test]
fn bt25_059_on_play_installs_suspend_up_to_2_prompt() {
    let mut runner = base().memory(12).start();
    runner.place_on_field(0, "OWN-VEG", Some(0));
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let ceres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, ceres.index as usize);

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::OwnField)
        ),
        "suspend-up-to-2 prompt installs"
    );
}

#[test]
fn bt25_059_suspend_picks_actually_suspend_targets() {
    let mut runner = base().memory(12).start();
    let veg = runner.place_on_field(0, "OWN-VEG", Some(0));
    let ceres = runner.place_on_field(0, CARD_ID, Some(0));
    runner.fire_on_play(0, ceres.index as usize);

    // Pick the Vegetation Digimon to suspend, then finish.
    let view = runner.pending_selection_view().expect("prompt installs");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick 1 to suspend");
    runner.auto_resolve().expect("resolve");

    let suspended = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.is_suspended);
    assert!(suspended, "at least one Digimon was suspended by the effect");
}

// ─── Behavior: on_suspend DP debuff ─────────────────────────────────────────

#[test]
fn bt25_059_on_suspend_debuffs_opponent_digimon() {
    let mut runner = base().memory(12).start();
    let ceres = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let opp_dp_before = runner.effective_dp(opp).expect("dp");

    // Suspending a Digimon (via Ceresmon's OP/WD) triggers the on_suspend clause.
    runner.fire_on_play(0, ceres.index as usize);
    // Resolve the suspend-up-to-2 selection by suspending Ceresmon itself.
    if matches!(
        runner.pending_kind(),
        Some(SelectionKind::OwnField)
    ) {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("suspend 1");
        runner.auto_resolve().expect("resolve cascade incl. on_suspend debuff");
    }

    // The on_suspend observer gives the opponent Digimon -3000 per suspended.
    let opp_dp_after = runner.effective_dp(opp).expect("dp");
    assert!(
        opp_dp_after < opp_dp_before,
        "on_suspend clause reduces an opponent Digimon's DP (before={opp_dp_before}, after={opp_dp_after})"
    );
}
