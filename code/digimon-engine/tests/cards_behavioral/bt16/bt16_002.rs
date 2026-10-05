//! BT16-002 DemiVeemon — Digi-Egg, Lv.2, Blue.
//!
//! # Card text (official Bandai DB, data/card_bundles/BT16-002.md)
//!
//! Inherited Effect [All Turns] While this Digimon has 2 or more colors, it
//! gets +1000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/Blue/BT16_002.cs — inherited
//! `ChangeSelfDPStaticEffect(1000)` gated on `TopCard.CardColors.Count >= 2`.
//!
//! # Patterns (docs/RUST_DSL_TEST_API.md §4.3)
//! - D4 inherited declarative DP aura
//! - D-adjacent conditional gate: `self_color_count_gte` (top-card colors)

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT16-002";

fn carrier(id: &str, colors: Vec<CardColor>) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.colors = colors;
    card.dp = Some(3000);
    card
}

/// DemiVeemon in the breeding-style base slot with `top` digivolved on it.
fn stack_with_top(top: CardData) -> (DebugRunner, PermanentHandle) {
    let top_id = top.card_id.clone();
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-002 YAML parses and compiles")
        .add_card(top)
        .memory(10)
        .start();
    let handle = runner.place_on_field(0, CARD_ID, Some(0));
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == top_id)
        .expect("top card registered");
    let next_idx = runner.game.next_card_index();
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[handle.index as usize]
        .digivolve(CardSource::new(data_idx, 0, next_idx), turn);
    runner.game.tick_declarative_effects();
    (runner, handle)
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn bt16_002_has_single_inherited_dp_aura() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT16-002 compiles")
        .start();
    let compiled = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(compiled.name, "DemiVeemon");
    assert_eq!(compiled.level, Some(2));
    assert_eq!(compiled.effects.len(), 1);
    let (scope, dp) = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope,
                dp_modifier,
                ..
            }) => Some((*scope, *dp_modifier)),
            _ => None,
        })
        .expect("BT16-002 must compile an aura");
    assert_eq!(scope, CompiledScope::Inherited);
    assert_eq!(dp, Some(1000));
}

// ─── Section 2/3 — Condition gating, integrated ──────────────────────────────

#[test]
fn bt16_002_two_color_carrier_gets_plus_1000_on_your_turn() {
    let (runner, handle) = stack_with_top(carrier("DUAL", vec![CardColor::Blue, CardColor::Green]));
    assert_eq!(runner.turn_player(), 0);
    assert_eq!(runner.effective_dp(handle), Some(4000));
}

#[test]
fn bt16_002_two_color_carrier_gets_plus_1000_on_opponents_turn() {
    let (mut runner, handle) =
        stack_with_top(carrier("DUAL", vec![CardColor::Blue, CardColor::Green]));
    runner.end_turn();
    assert_eq!(runner.turn_player(), 1, "[All Turns] — still active");
    runner.game.tick_declarative_effects();
    assert_eq!(runner.effective_dp(handle), Some(4000));
}

#[test]
fn bt16_002_mono_color_carrier_gets_no_bonus() {
    let (runner, handle) = stack_with_top(carrier("MONO", vec![CardColor::Blue]));
    assert_eq!(
        runner.effective_dp(handle),
        Some(3000),
        "1-color top card does not meet 'has 2 or more colors'"
    );
}
