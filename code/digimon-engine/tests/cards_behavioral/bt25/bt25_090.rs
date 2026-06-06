//! BT25-090 Tomoro Tenma — Tamer, Green, Cost 4. Traits: Glowing Dawn, BEATBREAK.
//!
//! # Card text (data/cards.json, confirmed vs DCGO)
//! [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! [All Turns] When any Digimon suspend, by suspending this Tamer, you may place
//!   the top 2 cards of your deck face down under this Tamer.
//! [Your Turn] [Once Per Turn] When you would use [Glowing Dawn] trait Option
//!   cards, by trashing the bottom face-down card under any of your Tamers,
//!   reduce the cost by 1.                              <-- BLOCKED (omitted)
//! Inherited: [Security] Play this card without paying the cost.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Green/BT25_090.cs
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - B1 start-of-turn tamer (memory swing — set to 3)
//! - on-suspend (any Digimon) self-suspend cost → place top 2 face down (stash substrate)
//! - Tamer [Security] play-self
//!
//! # Verdict — PARTIAL
//! Clause 3 (Glowing Dawn Option-USE cost reduction by trashing a face-down card
//! under a Tamer) is BLOCKED on engine gap G-COST-REDUCTION-INTERACTIVE-PAY-COST
//! (docs/RUST_ENGINE_GAPS.md): the interactive Tamer-pick pay_cost parks, so the
//! reduction credit is dropped while still paying the cost. Clause omitted from
//! the YAML rather than shipping an over-charge. Clauses 1, 2, 4 IMPLEMENTED.

#![allow(dead_code)]

use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledDeclarativeClause, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

use crate::dsl_card_data::{card_data_from_compiled, compiled};

const CARD_ID: &str = "BT25-090";

fn tomoro() -> CardData {
    card_data_from_compiled(CARD_ID)
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
fn bt25_090_compiles_as_tamer() {
    let card = compiled(CARD_ID);
    assert_eq!(card.card, CARD_ID);
    assert_eq!(card.kind, CompiledCardKind::Tamer);
    assert_eq!(card.cost, Some(4));
    assert!(card.traits.iter().any(|t| t == "Glowing Dawn"));
}

#[test]
fn bt25_090_has_start_of_turn_suspend_and_security_clauses() {
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
        "start-of-your-turn clause present"
    );
    let on_suspend = triggered
        .iter()
        .find(|t| t.when == vec![CompiledTiming::OnSuspend])
        .expect("on-suspend clause present");
    assert!(on_suspend.optional, "'you may place' → optional clause");
    assert!(
        triggered
            .iter()
            .any(|t| t.when == vec![CompiledTiming::OnSecurity]),
        "security play-self clause present"
    );
}

#[test]
fn bt25_090_blocked_cost_reduction_clause_is_omitted() {
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
// Section 2 — [Start of Your Turn] set memory to 3
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_090_start_of_turn_sets_low_memory_to_three() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
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
    assert_eq!(runner.memory(), 3, "memory <=2 is set to 3");
}

#[test]
fn bt25_090_start_of_turn_does_not_change_high_memory() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
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
    assert_eq!(runner.memory(), 5, "memory >2 is unchanged");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — [All Turns] on ANY Digimon suspend → may suspend self + place 2
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_090_on_any_digimon_suspend_suspends_self_and_places_top_two() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(8)
        .start();

    let tomoro_perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0));

    let deck_before = runner.deck_size(0);
    // Any Digimon suspending fires the trigger — suspend the opponent's Digimon.
    runner.game.suspend(opp_digi);

    assert!(
        runner.game.pending_selection.is_some(),
        "an optional accept/decline prompt installs when a Digimon suspends"
    );
    runner
        .accept_optional_trigger()
        .expect("accept the suspend+place");
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[tomoro_perm.index as usize].is_suspended,
        "Tomoro is suspended (the activation cost was paid)"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 2,
        "top 2 deck cards placed under Tomoro"
    );
    let perm = &runner.game.players[0].battle_area[tomoro_perm.index as usize];
    assert_eq!(perm.card_sources.len(), 3, "own card + 2 face-down stash");
    assert!(
        perm.card_sources[0].face_down && perm.card_sources[1].face_down,
        "the two placed sources are face-down"
    );
}

#[test]
fn bt25_090_on_suspend_does_not_fire_for_tamer_suspend() {
    // A Tamer suspending (not a Digimon) must NOT trigger (event_target_kind: digimon).
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(8)
        .start();

    let tomoro_perm = runner.place_on_field(0, CARD_ID, Some(0));
    // Suspend Tomoro itself (a Tamer). The clause gates on a Digimon suspending.
    runner.game.suspend(tomoro_perm);
    let _ = runner.auto_resolve();

    // No place-2 prompt fired (Tamer suspend is not a Digimon suspend).
    let perm = &runner.game.players[0].battle_area[tomoro_perm.index as usize];
    assert_eq!(
        perm.card_sources.len(),
        1,
        "a Tamer suspend must not trigger the place-2 effect (event_target_kind: digimon gate)"
    );
}

#[test]
fn bt25_090_on_suspend_decline_does_nothing() {
    let mut runner = DebugRunner::builder()
        .add_card(tomoro())
        .add_card(make_filler("OPP-DIGI"))
        .add_card(make_filler("D1"))
        .add_card(make_filler("D2"))
        .deck(0, &["D1", "D2"])
        .deck(1, &["D1"])
        .memory(8)
        .start();

    let tomoro_perm = runner.place_on_field(0, CARD_ID, Some(0));
    let opp_digi = runner.place_on_field(1, "OPP-DIGI", Some(0));
    let deck_before = runner.deck_size(0);

    runner.game.suspend(opp_digi);
    assert!(runner.game.pending_selection.is_some());
    assert!(runner.pending_is_optional());
    runner
        .decline_optional_trigger()
        .expect("decline the optional clause");
    let _ = runner.auto_resolve();

    assert!(
        !runner.game.players[0].battle_area[tomoro_perm.index as usize].is_suspended,
        "declining leaves Tomoro unsuspended"
    );
    assert_eq!(runner.deck_size(0), deck_before, "no cards placed");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — [Security] play-self structural
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn bt25_090_clause_on_security_present() {
    let card = compiled(CARD_ID);
    let on_sec = card.effects.iter().any(|c| {
        matches!(c, CompiledClause::Triggered(t) if t.when == vec![CompiledTiming::OnSecurity])
    });
    assert!(on_sec, "[Security] play-self clause must compile");
}
