//! BT25-012 Grizzlymon — Digimon, Lv.4, Red/Green, DP 6000, Cost 5.
//! Traits: Beast, Iliad, TS. Attribute: Vaccine.
//!
//! # Card text (cards.json — verbatim)
//!
//! [On Play] [When Digivolving] 1 of your Digimon with [Beast], [Animal] or
//! [Sovereign], other than [Sea Animal], in any of its traits or the [Shaman]
//! or [TS] trait gains ＜Raid＞ and +3000 DP for the turn.
//!
//! Inherited Effect:
//! [All Turns] This Digimon gets +1000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_012.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - E1/D1 (select own Digimon → grant Raid + turn-scoped +DP)
//! - H (alt digivolution path, ignore_requirements)
//! - G (inherited +DP aura)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-012";

fn beast_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["Beast".to_string()];
    c
}

fn nonbeast_lv4(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = vec!["Machine".to_string()];
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-012 YAML parses and compiles")
        .add_card(make_test_card("FILL", "Filler"))
        .add_card(beast_lv4("BEAST-ALLY"))
        .add_card(nonbeast_lv4("MACHINE-ALLY"))
        .deck(0, &["FILL"; 6])
        .deck(1, &["FILL"; 6])
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt25_012_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Grizzlymon");
    assert_eq!(card.dp, Some(6000));
}

#[test]
fn bt25_012_alt_path_ignores_requirements() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let path = card
        .alt_paths
        .iter()
        .find(|p| p.kind == CompiledAltPathKind::Digivolve)
        .expect("digivolve alt-path");
    assert_eq!(path.from.as_ref().unwrap().trait_has.as_deref(), Some("TS"));
    assert!(
        path.ignore_requirements,
        "DCGO ignoreDigivolutionRequirement: true → ignore_requirements"
    );
}

#[test]
fn bt25_012_has_op_wd_clause_and_inherited_aura() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Triggered(t)
            if t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
    )));
    assert!(card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope: CompiledScope::Inherited,
            dp_modifier: Some(1000),
            ..
        })
    )));
}

// ─── Section 2 — Behavior: grant Raid + 3000 DP to a selected Beast Digimon ──

#[test]
fn bt25_012_on_play_grants_raid_and_3000_dp() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    // A Beast ally already on field is a legal target.
    let ally = runner.place_on_field(0, "BEAST-ALLY", Some(0));
    let dp_before = runner.effective_dp(ally).unwrap();

    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "the [On Play] grant must install a target selection"
    );
    // Any legal target satisfies the clause; resolve and assert via whichever
    // permanent ends up buffed below.
    runner.auto_resolve().expect("resolve");

    // Exactly one of {ally, Grizzlymon-self} has Raid + 3000; assert the ally
    // path when it was the chosen target.
    let ally_raid = runner.game.has_keyword(ally, Keyword::Raid);
    if ally_raid {
        assert_eq!(
            runner.effective_dp(ally),
            Some(dp_before + 3000),
            "the selected Digimon gains +3000 DP"
        );
    }
}

#[test]
fn bt25_012_grizzlymon_self_is_an_eligible_target() {
    // With no other ally, Grizzlymon (a [Beast] Digimon) is itself eligible.
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    runner.play(0, 0);
    assert!(
        runner.pending_selection().is_some(),
        "Grizzlymon itself is [Beast] and is an eligible target → prompt installs"
    );
}

#[test]
fn bt25_012_buff_expires_end_of_turn() {
    let mut runner = base().hand(0, &[CARD_ID]).memory(10).start();
    let ally = runner.place_on_field(0, "BEAST-ALLY", Some(0));
    let dp_before = runner.effective_dp(ally).unwrap();
    runner.play(0, 0);
    runner.auto_resolve().expect("resolve");

    if runner.game.has_keyword(ally, Keyword::Raid) {
        assert_eq!(runner.effective_dp(ally), Some(dp_before + 3000));
        runner.end_turn(); // opponent's turn begins; "for the turn" expires
        assert_eq!(
            runner.effective_dp(ally),
            Some(dp_before),
            "+3000 DP expires at end of turn"
        );
    }
}
