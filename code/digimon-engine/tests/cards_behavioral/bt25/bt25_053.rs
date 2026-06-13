//! BT25-053 Aegiochusmon: Green — Digimon, Lv.5, Green/Black, DP 8000, Cost 8.
//! Traits: Shaman, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! ＜Vortex＞
//! ＜Decode ([Aegiomon])＞
//! [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon or Tamers.
//! It can't unsuspend until their turn ends. Then, if you have 3 or fewer
//! security cards, this Digimon gains ＜Piercing＞ and +5000 DP for the turn.
//!
//! Inherited: [All Turns] [Once Per Turn] When your security stack is removed
//! from, you may suspend 1 of your opponent's Digimon or Tamers.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_053.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - H16 Vortex / H3 Piercing grant (turn-scoped)
//! - special <Decode> replacement
//! - mandatory suspend + CannotUnsuspend (end_of_opponents_turn)
//! - security-gated self buff
//! - inherited security-removed OPT suspend

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardKind, Keyword};

const CARD_ID: &str = "BT25-053";

fn opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 4;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-053 YAML parses and compiles")
        .add_card(make_test_card("DECK-PAD", "Filler"))
        .add_card(opp_digimon("OPP-DIGI"))
        .deck(0, &["DECK-PAD"; 12])
        .deck(1, &["DECK-PAD"; 12])
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_053_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    assert_eq!(card.name, "Aegiochusmon: Green");
    assert_eq!(card.level, Some(5));
    assert_eq!(card.dp, Some(8000));
}

#[test]
fn bt25_053_grants_vortex_and_has_decode_replacement() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let vortex = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { keyword, scope, .. })
            if keyword.eq_ignore_ascii_case("Vortex") && *scope == CompiledScope::FaceUp
    ));
    let decode = card.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
    ));
    assert!(vortex, "own <Vortex>");
    assert!(decode, "Decode modeled as Replacement");
}

#[test]
fn bt25_053_has_inherited_security_removed_optional_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("present");
    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t)
                if t.scope == CompiledScope::Inherited
                    && t.when.contains(&CompiledTiming::OnOwnSecurityRemoved) =>
            {
                Some(t)
            }
            _ => None,
        })
        .expect("inherited clause present");
    assert!(clause.once_per_turn && clause.optional);
}

// ─── Section 2 — Condition gating (no opp target) ────────────────────────────

/// Negative: with no opponent Digimon/Tamer the OP/WD clause's `any_permanent`
/// guard blocks; no suspend prompt installs.
#[test]
fn bt25_053_no_opponent_target_blocks_clause() {
    let mut runner = base().hand(0, &[CARD_ID]).security(0, &["DECK-PAD"]).memory(10).start();
    let _perm = runner.play(0, 0).expect("play");
    assert!(
        runner.pending_selection().is_none(),
        "no opp Digimon/Tamer → suspend clause does not install"
    );
}

// ─── Section 3 — Behavior: suspend + freeze + security-gated self buff ───────

/// Positive: opp Digimon present → it is suspended and gains CannotUnsuspend.
#[test]
fn bt25_053_suspends_and_freezes_opponent() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"]) // 2 ≤ 3 → self buff fires too
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP-DIGI", Some(0));

    let _perm = runner.play(0, 0).expect("play");
    runner.auto_resolve().ok();

    let opp_perm = &runner.game.players[1].battle_area[opp.index as usize];
    assert!(opp_perm.is_suspended, "the chosen opp Digimon is suspended");
    assert!(
        runner
            .modifiers()
            .has(opp, digimon_engine::enums::ModifierType::CannotUnsuspend),
        "the opp Digimon gains CannotUnsuspend"
    );
}

/// Security gate positive: with ≤3 security this Digimon gains Piercing +5000 DP.
#[test]
fn bt25_053_self_gains_piercing_and_dp_with_low_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD"]) // 2 ≤ 3
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let perm_idx = runner.play(0, 0).expect("play");
    let perm = runner.perm_handle(0, perm_idx);
    let base_dp = 8000;
    runner.auto_resolve().ok();

    assert_eq!(
        runner.effective_dp(perm).expect("self dp"),
        base_dp + 5000,
        "with ≤3 security this Digimon gets +5000 DP"
    );
    assert!(
        runner.game.has_keyword(perm, Keyword::Piercing),
        "with ≤3 security this Digimon gains <Piercing>"
    );
}

/// Security gate negative: with >3 security the self buff does NOT apply.
#[test]
fn bt25_053_self_buff_blocked_with_high_security() {
    let mut runner = base()
        .hand(0, &[CARD_ID])
        .security(0, &["DECK-PAD", "DECK-PAD", "DECK-PAD", "DECK-PAD"]) // 4 > 3
        .memory(10)
        .start();
    runner.place_on_field(1, "OPP-DIGI", Some(0));

    let perm_idx = runner.play(0, 0).expect("play");
    let perm = runner.perm_handle(0, perm_idx);
    runner.auto_resolve().ok();

    assert_eq!(
        runner.effective_dp(perm).expect("self dp"),
        8000,
        "with >3 security no +5000 DP"
    );
    assert!(
        !runner.game.has_keyword(perm, Keyword::Piercing),
        "with >3 security no <Piercing>"
    );
}
