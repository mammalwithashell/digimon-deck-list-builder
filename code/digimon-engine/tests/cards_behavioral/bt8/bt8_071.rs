//! BT8-071 Psychemon — Digimon, Lv.3, Purple, DP 3000, Cost 3. Traits: Reptile.
//!
//! # Card text (official Bandai DB bundle `data/card_bundles/BT8-071.md`;
//! card image confirms)
//!
//! [All Turns] Players can't reduce play costs.
//!
//! Official Q&A: "This effect prevents costs from being reduced when play
//! costs are to be paid. This effect affects both players."
//!
//! Digivolve: Purple Lv.2 / cost 0.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT8/Purple/BT8_071.cs — a single
//! `CannotReduceCostClass` at `EffectTiming.None`: `PlayerCondition` = every
//! player, `TargetPermanentsCondition` = "no target permanents" (i.e. PLAY
//! costs, not digivolution costs), `CardCondition` = `HasPlayCost`, gated on
//! `IsExistOnBattleArea(card)`.
//!
//! # Patterns this test covers
//! - D6 flood gate / player restriction (`kind: flood_gate`,
//!   `CannotReducePlayCost`, `target_player: any`) — the ST13-08 Chikurimon
//!   idiom (identical printed text).

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledColor, CompiledCost, CompiledDeclarativeClause,
    CompiledPlayerRef,
};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;
use digimon_engine::replacement::ReplacementCause;

const CARD_ID: &str = "BT8-071";

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT8-071 YAML parses, compiles and is in the embedded pack")
        .memory(5)
        .start()
}

// ─── Section 1 — Structural ───────────────────────────────────────────────────

#[test]
fn bt8_071_metadata_matches_printed_card() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.name, "Psychemon");
    assert_eq!(card.level, Some(3));
    assert_eq!(card.cost, Some(3));
    assert_eq!(card.dp, Some(3000));
    assert_eq!(card.color, vec![CompiledColor::Purple]);
    assert!(card.traits.iter().any(|t| t == "Reptile"));
}

#[test]
fn bt8_071_has_one_standard_lv2_purple_digivolve_circle() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");
    assert_eq!(card.alt_paths.len(), 1, "one printed circle: Purple Lv.2 / cost 0");
    let path = &card.alt_paths[0];
    assert_eq!(path.kind, CompiledAltPathKind::Digivolve);
    assert_eq!(path.cost, Some(CompiledCost::Literal(0)));
    let from = path.from.as_ref().expect("bare level/colour gate");
    assert_eq!(from.level_eq, Some(2));
}

#[test]
fn bt8_071_has_player_scoped_cannot_reduce_play_cost_flood_gate_for_both_players() {
    let runner = runner();
    let card = runner.compiled_card(CARD_ID).expect("compiled card");

    let gates: Vec<(&str, Option<CompiledPlayerRef>)> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                modifier,
                target_player,
                ..
            }) => Some((modifier.as_str(), *target_player)),
            _ => None,
        })
        .collect();

    assert_eq!(gates.len(), 1, "exactly one flood-gate clause");
    assert_eq!(gates[0].0, "CannotReducePlayCost");
    assert_eq!(
        gates[0].1,
        Some(CompiledPlayerRef::Any),
        "printed 'Players' (plural) must target BOTH players"
    );
    assert_eq!(card.effects.len(), 1, "the flood gate is the card's only clause");
}

// ─── Section 2 / 3 — Behavioral ──────────────────────────────────────────────

#[test]
fn bt8_071_no_lock_before_psychemon_is_on_the_field() {
    // NEGATIVE baseline: the lock only exists while the card is on the field.
    let mut runner = runner();
    runner.game.tick_declarative_effects();
    assert!(!runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotReducePlayCost));
    assert!(!runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotReducePlayCost));
}

#[test]
fn bt8_071_installs_play_cost_reduction_lock_on_both_players() {
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);

    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotReducePlayCost),
        "controller can't reduce play costs"
    );
    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotReducePlayCost),
        "opponent can't reduce play costs either (Q&A: affects both players)"
    );
}

#[test]
fn bt8_071_lock_persists_on_opponents_turn() {
    // [All Turns]: the gate is live on the opponent's turn too.
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "it is now the opponent's turn");

    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotReducePlayCost));
    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotReducePlayCost));
}

#[test]
fn bt8_071_lock_does_not_touch_digivolution_cost_reduction() {
    // The printed text is about PLAY costs only (DCGO TargetPermanentsCondition
    // = no target permanents) — the digivolution-cost lock must NOT be set.
    let mut runner = runner();
    runner.place_on_field(0, CARD_ID, None);
    runner.game.tick_declarative_effects();

    assert!(!runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotReduceDigivolveCost));
    assert!(!runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotReduceDigivolveCost));
}

#[test]
fn bt8_071_lock_lifts_once_psychemon_leaves_the_field() {
    let mut runner = runner();
    let perm = runner.place_on_field(0, CARD_ID, None);
    runner.game.tick_declarative_effects();
    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotReducePlayCost));

    runner
        .game
        .delete_permanent_with_cause(perm, ReplacementCause::OpponentEffect);
    let _ = runner.auto_resolve();
    runner.game.tick_declarative_effects();

    assert!(
        !runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotReducePlayCost),
        "once Psychemon is gone, the controller may reduce play costs again"
    );
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotReducePlayCost),
        "once Psychemon is gone, the opponent may reduce play costs again"
    );
}
