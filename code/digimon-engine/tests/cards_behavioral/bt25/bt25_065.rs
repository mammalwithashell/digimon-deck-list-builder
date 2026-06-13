//! BT25-065 Monodramon — Digimon, Lv.3, Black, DP 4000, Cost 3.
//! Traits: Mini Dragon, Iliad, TS. Attribute: Vaccine.
//!
//! Printed effect text (cards.json):
//!   [All Turns] When this Digimon suspends, <Draw 1> (Draw 1 card from your
//!   deck.)
//!   [Your Turn] When this Digimon attacks a player, lose 2 memory.
//!
//! Printed inherited text:
//!   [All Turns] This Digimon gets +1000 DP.
//!
//! DCGO C# reference: DCGO/Assets/Scripts/CardEffect/BT25/Black/BT25_065.cs
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): on_suspend self-trigger draw,
//! on_attack-a-player memory loss (event_target_is_player gate), D1 inherited
//! All-Turns DP aura, alt-digivolve recipe.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardKind;
use digimon_engine::CardData;

fn monodramon() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("BT25-065")
        .expect("BT25-065 YAML in embedded pack")
}

fn make_digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

// ── Section 1: Structural ──────────────────────────────────────────────────

#[test]
fn bt25_065_has_on_suspend_self_draw_clause() {
    let runner = monodramon().build();
    let card = runner.compiled_card("BT25-065").expect("compiled");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSuspend) => Some(t),
            _ => None,
        })
        .expect("BT25-065 must have an on_suspend clause");

    // Self-gated (printed: "When THIS Digimon suspends").
    // The gate is a condition predicate; assert the draw step is present.
    let has_draw = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::Draw { .. }));
    assert!(has_draw, "on_suspend clause must Draw 1");
    assert!(
        clause.condition.is_some(),
        "on_suspend must be self-gated (event_permanent_is_source) — printed 'this Digimon'"
    );
}

#[test]
fn bt25_065_has_on_attack_lose_memory_clause() {
    let runner = monodramon().build();
    let card = runner.compiled_card("BT25-065").expect("compiled");

    let clause = card
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAttack) => Some(t),
            _ => None,
        })
        .expect("BT25-065 must have an on_attack clause");

    let loses_two = clause
        .process
        .iter()
        .any(|s| matches!(s, CompiledStep::LoseMemory(2)));
    assert!(loses_two, "on_attack clause must lose 2 memory");
    assert!(
        clause.condition.is_some(),
        "on_attack must gate on attacking a player (event_target_is_player)"
    );
}

#[test]
fn bt25_065_has_inherited_all_turns_dp_aura() {
    let runner = monodramon().build();
    let card = runner.compiled_card("BT25-065").expect("compiled");
    let aura = card.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
                scope: CompiledScope::Inherited,
                dp_modifier: Some(1000),
                ..
            })
        )
    });
    assert!(aura, "BT25-065 must declare inherited +1000 DP aura");
}

#[test]
fn bt25_065_registers_ts_alt_digivolve() {
    let runner = monodramon().build();
    let card = runner.compiled_card("BT25-065").expect("compiled");
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(digivolve_paths, 1, "BT25-065 must register the TS-trait alt digivolve path");
}

// ── Section 3: Behavioral — on_suspend self-draw ───────────────────────────

#[test]
fn bt25_065_drawing_on_self_suspend() {
    let mut runner = monodramon()
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    let perm = runner.place_on_field(0, "BT25-065", Some(0));

    let hand_before = runner.hand_size(0);
    runner.game.suspend(perm);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "self-suspend draws exactly 1"
    );
}

#[test]
fn bt25_065_ally_suspend_does_not_draw() {
    let mut runner = monodramon()
        .add_card(make_digimon("ALLY-065", 4, 5000))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    let _self_perm = runner.place_on_field(0, "BT25-065", Some(0));
    let ally = runner.place_on_field(0, "ALLY-065", Some(0));

    let hand_before = runner.hand_size(0);
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "an ally suspending must NOT trigger Monodramon's self-only on_suspend draw"
    );
}

// ── Section 3: Behavioral — on_attack lose 2 memory ────────────────────────

#[test]
fn bt25_065_attacking_a_player_loses_two_memory() {
    let mut runner = monodramon()
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .security(1, &["FILL"; 5])
        .memory(5)
        .start();
    // Place on field with turn_played_override so it is not summoning-sick.
    let attacker = runner.place_on_field(0, "BT25-065", Some(0));

    let memory_before = runner.memory();
    runner.attack_player(attacker, 1, false);
    let _ = runner.auto_resolve();

    // The on_attack lose-2 fires once when attacking the player.
    assert_eq!(
        runner.memory(),
        memory_before - 2,
        "attacking a player loses 2 memory"
    );
}

#[test]
fn bt25_065_attacking_a_digimon_does_not_lose_memory() {
    let mut runner = monodramon()
        .add_card(make_digimon("DEF-065", 3, 1000))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 10])
        .memory(5)
        .start();
    let attacker = runner.place_on_field(0, "BT25-065", Some(0));
    // Suspended defender so the attack targets a Digimon, not the player.
    let defender = runner.place_on_field(1, "DEF-065", Some(0));
    runner.game.players[1].battle_area[defender.index as usize].is_suspended = true;

    let memory_before = runner.memory();
    runner.attack_digimon(attacker, defender, false);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.memory(),
        memory_before,
        "attacking a Digimon must NOT lose memory (clause gates on attacking a player)"
    );
}
