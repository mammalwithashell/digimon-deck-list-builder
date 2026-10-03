//! EX13-006 Dorimon — Digi-Egg, Lv.2, Black.
//! Traits: Lesser / X Antibody / Chronicle. Form: In-Training.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-006.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-006.md` agrees)
//!
//! ```text
//! Inherited Effect:
//! [End of Your Turn] [Once Per Turn] By paying 1 cost, 1 of your Digimon with
//! the [X Antibody] or [Chronicle] trait may unsuspend.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX13/Black/EX13_006.cs
//! - `OnEndTurn`, `SetIsInheritedEffect(true)`, OPT (`SetUpActivateClass(..., 1,
//!   true, ...)` → optional activation, once per turn), `IsOwnerTurn`.
//! - Activate: `AddMemory(-1)` (the paid cost), then `SelectPermanentEffect`
//!   (`canNoSelect: true`, `Mode.UnTap`) over own SUSPENDED permanents whose top
//!   card has the [X Antibody] or [Chronicle] trait.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - G-row inherited [End of Your Turn] + [Once Per Turn].
//! - Optional memory-cost activation (outer accept/decline confirm,
//!   BT12-092 / BT22-041 idiom) → optional own-permanent pick → unsuspend.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause};
use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::chronicle_support::*;

const CARD_ID: &str = "EX13-006";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-006 YAML parses, compiles and is in the embedded pack")
        .add_card(digimon("CARRIER", CardColor::Black, 3, 3000, &["Beast"]))
        .add_card(digimon("XA-DIGI", CardColor::Black, 4, 5000, &["X Antibody"]))
        .add_card(digimon("CHRON-DIGI", CardColor::Yellow, 4, 5000, &["Chronicle"]))
        .add_card(digimon("PLAIN-DIGI", CardColor::Black, 4, 5000, &["Beast"]))
        .add_card(digimon("FILL", CardColor::Black, 3, 1000, &[]))
}

fn inherited(runner: &DebugRunner) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t.clone()),
            _ => None,
        })
        .collect()
}

/// Dorimon under CARRIER; `others` placed suspended on player 0's field.
fn setup(others: &[&str]) -> (DebugRunner, PermanentHandle, Vec<PermanentHandle>) {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let mut hs = Vec::new();
    for id in others {
        let h = runner.place_on_field(0, id, Some(0));
        set_suspended(&mut runner, h, true);
        hs.push(h);
    }
    (runner, carrier, hs)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_006_has_one_inherited_optional_opt_end_of_your_turn_clause() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.traits, vec!["Lesser", "X Antibody", "Chronicle"]);
    let inh = inherited(&runner);
    assert_eq!(inh.len(), 1, "exactly one inherited triggered clause");
    let c = &inh[0];
    assert_eq!(c.when, vec![CompiledTiming::EndOfYourTurn]);
    assert!(c.optional, "\"By paying 1 cost ...\" is a may-activate");
    assert!(c.once_per_turn, "[Once Per Turn]");
    let face_up = card
        .effects
        .iter()
        .filter(|c| matches!(c, CompiledClause::Triggered(t) if t.scope == CompiledScope::FaceUp))
        .count();
    assert_eq!(face_up, 0, "Dorimon prints no face-up effect");
}

// ─── Section 2/3 — Behavioral (integrated through end_turn) ──────────────────

#[test]
fn ex13_006_end_of_turn_pays_one_and_unsuspends_x_antibody_digimon() {
    let (mut runner, _carrier, hs) = setup(&["XA-DIGI"]);
    let xa = hs[0];
    runner.game.memory = -3; // the opponent would start with 3
    runner.end_turn();

    // Outer may-activate confirm (cost not yet paid).
    let view = runner.pending_selection_view().expect("optional activation prompt");
    assert!(view.is_optional);
    let memory_before = runner.memory();
    accept(&mut runner);
    assert_eq!(runner.memory(), memory_before - 1, "1 cost paid on activation");

    let view = runner.pending_selection_view().expect("unsuspend target prompt");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(view.is_optional, "\"may unsuspend\" — the pick is declinable");
    pick_field(&mut runner, xa);
    assert!(!suspended(&runner, xa), "the X Antibody Digimon unsuspended");
}

#[test]
fn ex13_006_chronicle_trait_digimon_is_a_legal_target() {
    let (mut runner, _carrier, hs) = setup(&["CHRON-DIGI"]);
    runner.game.memory = -3; // the opponent would start with 3
    runner.end_turn();
    accept(&mut runner);
    assert!(field_offered(&runner, hs[0]), "[Chronicle] trait Digimon is offered");
    pick_field(&mut runner, hs[0]);
    assert!(!suspended(&runner, hs[0]));
}

#[test]
fn ex13_006_trait_less_digimon_is_not_a_legal_target() {
    let (mut runner, _carrier, hs) = setup(&["XA-DIGI", "PLAIN-DIGI"]);
    runner.game.memory = -3; // the opponent would start with 3
    runner.end_turn();
    accept(&mut runner);
    assert!(field_offered(&runner, hs[0]), "X Antibody Digimon offered");
    assert!(!field_offered(&runner, hs[1]), "a Digimon without either trait is not offered");
}

#[test]
fn ex13_006_unsuspended_digimon_is_not_a_target() {
    let (mut runner, _carrier, hs) = setup(&["XA-DIGI", "CHRON-DIGI"]);
    set_suspended(&mut runner, hs[1], false);
    runner.game.memory = -3; // the opponent would start with 3
    runner.end_turn();
    accept(&mut runner);
    assert!(field_offered(&runner, hs[0]));
    assert!(!field_offered(&runner, hs[1]), "only suspended Digimon can unsuspend");
}

#[test]
fn ex13_006_declining_activation_pays_nothing() {
    let (mut runner, _carrier, hs) = setup(&["XA-DIGI"]);
    runner.game.memory = -3; // the opponent would start with 3
    runner.end_turn();
    decline(&mut runner);
    assert_eq!(runner.turn_player(), 1, "turn passed");
    assert_eq!(runner.memory(), 3, "no cost paid: the opponent starts with 3");
    assert!(suspended(&runner, hs[0]), "nothing unsuspended");
}

#[test]
fn ex13_006_paid_cost_with_declined_pick_still_costs_one() {
    let (mut runner, _carrier, hs) = setup(&["XA-DIGI"]);
    runner.game.memory = -3; // the opponent would start with 3
    runner.end_turn();
    accept(&mut runner);
    decline(&mut runner);
    assert_eq!(runner.turn_player(), 1, "turn passed");
    assert_eq!(runner.memory(), 4, "the cost stays paid (DCGO pays first): opponent starts with 4");
    assert!(suspended(&runner, hs[0]));
}

#[test]
fn ex13_006_does_not_fire_at_end_of_opponents_turn() {
    // Dorimon under PLAYER 1's Digimon while it is player 0's turn: the
    // turn-ending player is not Dorimon's owner, so nothing fires.
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    assert_eq!(runner.turn_player(), 0);
    let _carrier = runner.place_stack(1, &[CARD_ID, "CARRIER"]);
    let xa = runner.place_on_field(1, "XA-DIGI", Some(0));
    set_suspended(&mut runner, xa, true);
    runner.game.memory = -3;
    runner.end_turn();
    assert!(
        runner.pending_selection().is_none(),
        "player 1's Dorimon does not fire at the end of player 0's turn"
    );
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(runner.memory(), 3, "no cost was paid");
}

// ─── Section 5 — OPT lockout ─────────────────────────────────────────────────

#[test]
fn ex13_006_once_per_turn_lockout_and_reset() {
    let (mut runner, carrier, hs) = setup(&["XA-DIGI"]);
    fire(&mut runner, EffectTiming::EndOfYourTurn, carrier);
    accept(&mut runner);
    pick_field(&mut runner, hs[0]);
    assert!(!suspended(&runner, hs[0]));

    set_suspended(&mut runner, hs[0], true);
    fire(&mut runner, EffectTiming::EndOfYourTurn, carrier);
    assert!(runner.pending_selection().is_none(), "second activation this turn is locked out");
    assert!(suspended(&runner, hs[0]));

    // Next own turn: the lockout has cleared.
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 1);
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 0, "back on player 0's turn");
    set_suspended(&mut runner, hs[0], true);
    fire(&mut runner, EffectTiming::EndOfYourTurn, carrier);
    let view = runner.pending_selection_view().expect("OPT reset on a new turn");
    assert!(view.is_optional);
}
