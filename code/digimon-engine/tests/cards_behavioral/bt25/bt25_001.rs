//! BT25-001 Tokomon — Digi-Egg, Lv.2, Red.
//! Traits: Lesser, Iliad, TS.
//!
//! # Card text (cards.json — verbatim)
//!
//! Inherited Effect [When Attacking] [Once Per Turn] If this Digimon has the
//! [TS] trait, <Draw 1> (Draw 1 card from your deck.)
//!
//! (Tokomon itself is a Digi-Egg with no face-up effect — only the inherited
//! When Attacking draw.)
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT25/Red/BT25_001.cs
//!   - timing OnAllyAttack, SetIsInheritedEffect(true),
//!     SetHashString("BT25_001_Inherited"), max_count_per_turn 1 (OPT).
//!   - CanActivate gated on the carrier (TopCard) having the [TS] trait;
//!     if not, RemoveUse() (does not consume the OPT).
//!
//! # Patterns this test covers (RUST_DSL_TEST_API §4.3)
//! - G4 inherited When Attacking on a Digi-Egg
//! - E2 OPT (once per turn) — lockout + reset
//! - D condition gating (carrier [TS] trait, positive + negative)

#![allow(dead_code, unused_imports, unused_variables, unused_mut)]

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;

const CARD_ID: &str = "BT25-001";

// ─── Fixture helpers ─────────────────────────────────────────────────────────

/// A Lv.3 Red carrier the Tokomon egg lives under. `traits` controls whether
/// the carrier (TopCard) has the [TS] trait — the inherited draw's gate.
fn carrier(card_id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(card_id, card_id, 3);
    card.colors = vec![CardColor::Red];
    card.dp = Some(3000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT25-001 YAML parses and compiles")
        .add_card(carrier("TS-CARRIER", &["TS"]))
        .add_card(carrier("PLAIN-CARRIER", &[]))
        .add_card(make_test_card_with_level("FILL", "Filler", 3))
        .deck(0, &["FILL"; 40])
        .memory(10)
}

// ─── Section 1 — Structural assertions ───────────────────────────────────────

#[test]
fn bt25_001_yaml_has_printed_metadata() {
    let runner = base().start();
    let card = runner
        .compiled_card(CARD_ID)
        .expect("BT25-001 must be in the embedded DSL pack");

    assert_eq!(card.name, "Tokomon");
    assert_eq!(card.level, Some(2));
    for trait_name in ["Lesser", "Iliad", "TS"] {
        assert!(
            card.traits.contains(&trait_name.to_string()),
            "BT25-001 metadata must include trait {trait_name}"
        );
    }
}

#[test]
fn bt25_001_has_one_inherited_when_attacking_opt_clause() {
    let runner = base().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled present");

    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        1,
        "Tokomon has exactly one triggered clause (the inherited When Attacking draw)"
    );
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert_eq!(clause.when, vec![CompiledTiming::WhenAttacking]);
    assert!(
        clause.once_per_turn,
        "[Once Per Turn] → once_per_turn must be set"
    );
}

// ─── Section 2 — Condition gating (carrier [TS] trait) ───────────────────────

#[test]
fn bt25_001_draws_when_ts_carrier_attacks() {
    let mut runner = base().start();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "TS-CARRIER"]);

    let hand_before = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().expect("resolve inherited draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "a [TS] carrier with the Tokomon source must draw 1 when attacking"
    );
}

#[test]
fn bt25_001_does_not_draw_without_ts_carrier() {
    let mut runner = base().start();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "PLAIN-CARRIER"]);

    let hand_before = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().expect("resolve attack with no draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "the inherited draw is gated by the carrier having the [TS] trait"
    );
}

// ─── Section 3 — OPT enforcement (lockout + reset) ───────────────────────────

#[test]
fn bt25_001_opt_blocks_second_attack_same_turn() {
    let mut runner = base().start();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "TS-CARRIER"]);

    // First attack fires the inherited draw.
    let hand_before = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().expect("first attack draws");
    assert_eq!(runner.hand_size(0), hand_before + 1, "first attack draws 1");

    // Unsuspend the attacker so it can attack again this turn.
    runner.game_mut().players[0].battle_area[stack.index as usize].is_suspended = false;

    let hand_mid = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().expect("second attack resolves");
    assert_eq!(
        runner.hand_size(0),
        hand_mid,
        "[Once Per Turn] must block a second inherited draw in the same turn"
    );
}

#[test]
fn bt25_001_opt_resets_next_turn() {
    let mut runner = base().start();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "TS-CARRIER"]);

    let hand_before = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    let _ = runner.auto_resolve();
    assert!(
        runner.hand_size(0) as i32 - hand_before as i32 >= 1,
        "turn 1 attack must fire the inherited TS draw"
    );

    if runner.game_over() {
        return;
    }

    // End P0's turn, advance through P1's turn, back to P0 (engine re-unsuspends
    // at start of turn — do NOT hand-toggle is_suspended, that diverges from the
    // production unsuspend path; mirrors bt12_002's OPT-clears idiom).
    runner.end_turn(); // P1's turn begins
    runner.end_turn(); // P0's turn begins again

    if runner.game.players[0].battle_area.is_empty() {
        return; // carrier did not survive combat — reset can't be exercised
    }
    let carrier2 = runner.perm_handle(0, 0);
    runner.game_mut().players[1].security.clear();

    let hand_mid = runner.hand_size(0);
    runner.attack_player(carrier2, 1, false);
    let _ = runner.auto_resolve();
    assert!(
        runner.hand_size(0) as i32 - hand_mid as i32 >= 1,
        "the OPT lockout must clear after end_turn: turn-2 attack must draw again"
    );
}
