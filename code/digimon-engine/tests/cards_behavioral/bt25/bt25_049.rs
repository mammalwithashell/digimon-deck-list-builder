//! BT25-049 Armalizamon — Digimon, Lv.4, Green, DP 4000, Cost 4.
//! Traits: Reptile, Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json, confirmed vs DCGO)
//! [On Play] [When Digivolving] You may suspend 1 of your opponent's Digimon.
//! [Your Turn] [Once Per Turn] When you would use an Option card with the
//!   [Glowing Dawn] trait, by trashing the bottom face-down card under any of
//!   your Tamers, reduce the cost by 3.                  <-- BLOCKED (omitted)
//! Inherited: <Piercing>.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_049.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - OnPlay/WhenDigivolving optional suspend opponent Digimon
//! - alt-digivolve from Glowing Dawn Lv.3
//! - H3 inherited Piercing
//!
//! # Verdict — PARTIAL
//! Clause 2 (Glowing Dawn Option-USE cost reduction by trashing a face-down card
//! under a Tamer, -3) is BLOCKED on engine gap
//! G-COST-REDUCTION-INTERACTIVE-PAY-COST (docs/RUST_ENGINE_GAPS.md). Omitted from
//! the YAML. Clauses 1, inherited Piercing, and the alt-digivolve are IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledCardKind, CompiledClause, CompiledDeclarativeClause,
    CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-049";

fn armalizamon() -> CardData {
    card_data_from_compiled(CARD_ID)
}

fn make_opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Red];
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_049_compiles_as_digimon() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Digimon);
    assert_eq!(card.cost, Some(4));
    assert_eq!(card.dp, Some(4000));
}

#[test]
fn bt25_049_has_onplay_whendigivolving_clause_and_inherited_piercing() {
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
            .any(|t| t.when == vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]),
        "OnPlay/WhenDigivolving suspend clause present"
    );

    let has_inherited_keyword = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { scope, .. })
                if *scope == CompiledScope::Inherited
        )
    });
    assert!(has_inherited_keyword, "inherited grant_keyword (Piercing) present");
}

#[test]
fn bt25_049_has_glowing_dawn_alt_digivolve() {
    let card = compiled(CARD_ID);
    let has_alt = card
        .alt_paths
        .iter()
        .any(|p| matches!(p.kind, CompiledAltPathKind::Digivolve));
    assert!(has_alt, "alt-digivolve path (Lv.3 Glowing Dawn) present");
}

#[test]
fn bt25_049_blocked_cost_reduction_clause_is_omitted() {
    let card = compiled(CARD_ID);
    let has_cr = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        !has_cr,
        "the interactive-pay_cost cost-reduction clause must stay omitted (BLOCKED)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — [On Play][When Digivolving] you may suspend 1 opponent Digimon
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_049_on_play_suspends_chosen_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(armalizamon())
        .add_card(make_opp_digimon("OPP"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP", Some(0));
    let arm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, arm.index as usize);

    let view = runner
        .pending_selection_view()
        .expect("optional suspend prompt installs (opponent Digimon present)");
    assert!(view.is_optional, "the suspend is 'you may' → optional");
    // Pick the opponent Digimon (first non-PASS action).
    let target = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|&id| id != digimon_engine::action::space::PASS)
        .expect("an opponent Digimon target exists");
    runner
        .execute_action(view.selecting_player, target)
        .expect("suspend the opponent Digimon");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "the chosen opponent Digimon is suspended"
    );
}

#[test]
fn bt25_049_on_play_can_decline_suspend() {
    let mut runner = DebugRunner::builder()
        .add_card(armalizamon())
        .add_card(make_opp_digimon("OPP"))
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let opp = runner.place_on_field(1, "OPP", Some(0));
    let arm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, arm.index as usize);

    assert!(runner.pending_is_optional(), "the suspend is optional");
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional suspend");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[1].battle_area[opp.index as usize].is_suspended,
        "declining leaves the opponent Digimon unsuspended"
    );
}

#[test]
fn bt25_049_on_play_no_prompt_when_no_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .add_card(armalizamon())
        .add_card(make_filler("FILLER"))
        .deck(0, &["FILLER"; 3])
        .deck(1, &["FILLER"; 3])
        .memory(10)
        .start();

    let arm = runner.place_on_field(0, CARD_ID, None);
    runner.fire_on_play(0, arm.index as usize);

    assert!(
        runner.pending_selection().is_none(),
        "no opponent Digimon → no suspend prompt"
    );
}
