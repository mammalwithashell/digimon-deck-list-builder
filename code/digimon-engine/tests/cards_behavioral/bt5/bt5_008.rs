//! BT5-008 Gaossmon — Digimon, Lv.3, DP 2000, Cost 3, Red.
//! Traits: Reptile.
//!
//! # Card text (cards.json)
//!
//! ```text
//! [Your Turn] Your other [Gaossmon] all get +3000 DP.
//! [Opponent's Turn] Your opponent can't reduce digivolution costs.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT5/Red/BT5_008.cs
//!
//! # Patterns this test covers
//! - D4  declarative aura (own-turn, named-card filter, +3000 DP, self-excluded)
//! - D6  flood gate / opponent restriction (opponent-turn, cannot-reduce-digivolve-cost)
//!
//! # Clause summary
//!
//! | # | Clause                       | Timing/Kind | Scope  | DSL shape                                                                                     |
//! |---|------------------------------|-------------|--------|-----------------------------------------------------------------------------------------------|
//! | 1 | [Your Turn] +3000 DP         | aura        | FaceUp | kind: aura, active_when: { your_turn: true }, target: { name_contains: "Gaossmon", other: true }, dp_modifier: 3000 |
//! | 2 | [Opp Turn] no cost reduction | flood_gate  | FaceUp | kind: flood_gate, active_when: { opponents_turn: true }, target_player: opponent, modifier: CannotReduceDigivolveCost |
//!
//! # Implementation status (2026-05-21)
//!
//! All three blocking gaps are resolved, so BT5-008 is now pure DSL with full
//! behavioral coverage:
//!
//! - G-DECLARATIVE-KEYWORD — RESOLVED 2026-05-02. `Game::tick_declarative_effects`
//!   dispatches process-backed declarative effects from face-up field sources, so
//!   the filtered aura and the player-scoped flood gate both materialize at runtime.
//! - G-OTHER-PREDICATE-UNEVALUATED — RESOLVED (Group 6). `eval_permanent_fields`
//!   now rejects the source `PermanentHandle` when `other: true`, so BT5-008 no
//!   longer buffs itself.
//! - G-PLAYER-FLOOD-GATE-DSL — RESOLVED (Group 6). `kind: flood_gate` with
//!   `target_player: opponent` installs `CannotReduceDigivolveCost` on the
//!   opponent; the prior `raw_rust` no-op is removed.
//!
//! `tick_declarative_effects()` is the natural state-change refresh hook — the
//! behavioral tests call it explicitly after seeding the field, exactly as the
//! Group 6 aura/flood-gate fixtures and the sibling BT5-033 Cutemon test do.

use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledPlayerRef, CompiledScope,
};
use digimon_engine::action::{build_action_mask, encode_digivolve};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::{CardColor, CardKind, ModifierType, PlaySource};

// ─── Helper builders ─────────────────────────────────────────────────────────

/// Build a Gaossmon-named Digimon suitable as an "other Gaossmon" target.
fn make_gaossmon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_name = "Gaossmon".to_string();
    c.traits = vec!["Reptile".to_string()];
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// Build a non-Gaossmon Digimon (should NOT be buffed by clause 1).
fn make_non_gaossmon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_name = "SomeOtherDigimon".to_string();
    c.traits = vec!["Reptile".to_string()];
    c.level = Some(3);
    c.dp = Some(2000);
    c
}

/// Standard runner: BT5-008 Gaossmon + one extra Gaossmon + one non-Gaossmon + filler deck.
fn gaossmon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT5-008")
        .expect("BT5-008 found in embedded DSL pack")
        .add_card(make_gaossmon("GAOSSMON-2"))
        .add_card(make_non_gaossmon("NON-GAOSSMON"))
        .add_card(make_test_card("FILLER", "FILLER"))
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start()
}

// ─── Clause 2 fixtures — opponent digivolution-cost reduction ────────────────

/// A field permanent whose effect reduces any digivolution cost by 3.
struct FieldCostReducer;

impl CardEffect for FieldCostReducer {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::before_pay_cost(card).cost_reduction(3).build()]
    }
}

fn make_level_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(3000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Red];
    card
}

fn make_evo_digimon(id: &str, level: u8, from_level: u8, cost: u16) -> CardData {
    let mut card = make_level_digimon(id, level);
    card.evo_costs = vec![EvoCost {
        level: from_level,
        memory_cost: cost,
        card_color: CardColor::Red as u8,
    }];
    card
}

/// Runner with a digivolve target + a P1-side cost reducer for clause 2 tests.
fn cost_reduction_runner() -> DebugRunner {
    let mut registry = digimon_engine::cards::build_registry();
    registry.insert("REDUCER", Arc::new(FieldCostReducer));

    DebugRunner::builder()
        .with_registry(registry)
        .dsl_card("BT5-008")
        .expect("BT5-008 found in embedded DSL pack")
        .add_card(make_level_digimon("BASE", 3))
        .add_card(make_evo_digimon("EVO", 4, 3, 4))
        .add_card(make_level_digimon("REDUCER", 4))
        .add_card(make_test_card("FILLER", "FILLER"))
        .hand(1, &["EVO"])
        .deck(0, &["FILLER"])
        .deck(1, &["FILLER"])
        .memory(10)
        .start()
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

/// BT5-008 must compile to at least 2 declarative clauses (aura + player-targeted flood_gate).
#[test]
fn bt5_008_compiles_to_at_least_two_declarative_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT5-008")
        .expect("BT5-008 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT5-008")
        .expect("BT5-008 compiled card present");

    assert!(
        compiled.effects.len() >= 2,
        "BT5-008 must have at least 2 clauses (aura + player-targeted flood_gate); got {}",
        compiled.effects.len()
    );

    // All clauses must be Declarative (no Triggered clauses — both effects are static/passive).
    for clause in &compiled.effects {
        assert!(
            matches!(clause, CompiledClause::Declarative(_)),
            "BT5-008 must have only declarative clauses; found triggered clause: {:?}",
            clause
        );
    }

    let flood_gate_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
            modifier,
            target_player,
            ..
        }) => Some((modifier.as_str(), *target_player)),
        _ => None,
    });

    assert_eq!(
        flood_gate_clause,
        Some((
            "CannotReduceDigivolveCost",
            Some(CompiledPlayerRef::Opponent),
        )),
        "BT5-008 clause 2 must be a player-targeted CannotReduceDigivolveCost flood_gate"
    );
}

/// Clause 0 must be a Declarative::Aura with FaceUp scope and dp_modifier = +3000.
#[test]
fn bt5_008_has_your_turn_aura_clause_shape() {
    let runner = DebugRunner::builder()
        .dsl_card("BT5-008")
        .expect("BT5-008 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT5-008")
        .expect("BT5-008 compiled card present");

    let aura_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });

    let (scope, dp) = aura_clause.expect(
        "BT5-008 must have a Declarative::Aura clause ([Your Turn] other Gaossmon +3000 DP)",
    );

    assert_eq!(
        scope,
        CompiledScope::FaceUp,
        "aura clause must be FaceUp scope (active while BT5-008 is on the field)"
    );
    assert_eq!(
        dp,
        Some(3000),
        "aura dp_modifier must be +3000 (printed text: +3000 DP)"
    );
}

// ─── Section 2: Clause 1 behavioral — [Your Turn] Gaossmon aura ──────────────

/// Positive: on your turn, another Gaossmon on the field gets +3000 DP
/// while BT5-008 is in play.
#[test]
fn bt5_008_aura_buffs_other_gaossmon_on_your_turn() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    let gaossmon_1 = runner.place_on_field(0, "BT5-008", None);
    // Place a second Gaossmon on P0's field.
    let gaossmon_2 = runner.place_on_field(0, "GAOSSMON-2", None);

    // Refresh declarative field auras now that both permanents are seeded.
    runner.game.tick_declarative_effects();

    // On P0's turn, the aura should give GAOSSMON-2 +3000 DP.
    let dp_gaossmon_2 = runner.dp_of(gaossmon_2).unwrap_or(0);
    assert_eq!(
        dp_gaossmon_2,
        2000 + 3000,
        "other Gaossmon must get +3000 DP on your turn; got {}",
        dp_gaossmon_2
    );

    let _ = gaossmon_1;
}

/// Negative: on your turn, a non-Gaossmon Digimon on P0's field does NOT get
/// buffed — the aura's `name_contains: "Gaossmon"` filter excludes it.
///
/// To prove this is real filtering (not a vacuous "aura does nothing" pass),
/// a sibling Gaossmon is on the same field and IS buffed in the same tick.
#[test]
fn bt5_008_aura_does_not_buff_non_gaossmon() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    runner.place_on_field(0, "BT5-008", None);
    // A real Gaossmon (control — must be buffed) and a non-Gaossmon (must not).
    let gaossmon_2 = runner.place_on_field(0, "GAOSSMON-2", None);
    let non_gaossmon = runner.place_on_field(0, "NON-GAOSSMON", None);

    runner.game.tick_declarative_effects();

    // Control: the named filter matches a real Gaossmon.
    assert_eq!(
        runner.dp_of(gaossmon_2).unwrap_or(0),
        2000 + 3000,
        "control: a real other Gaossmon must be buffed (proves the aura fired)"
    );

    // NON-GAOSSMON should NOT get +3000 DP from BT5-008's aura.
    let dp_non_gaossmon = runner.dp_of(non_gaossmon).unwrap_or(0);
    assert_eq!(
        dp_non_gaossmon,
        2000, // base DP, no buff
        "non-Gaossmon must NOT receive +3000 DP from BT5-008; got {}",
        dp_non_gaossmon
    );
}

/// Negative: on the opponent's turn, the [Your Turn] aura condition fails —
/// the other Gaossmon should NOT get +3000 DP.
#[test]
fn bt5_008_aura_does_not_fire_on_opponents_turn() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    let gaossmon_1 = runner.place_on_field(0, "BT5-008", None);
    // Place a second Gaossmon on P0's field.
    let gaossmon_2 = runner.place_on_field(0, "GAOSSMON-2", None);

    // Sanity: on P0's turn the aura is active.
    runner.game.tick_declarative_effects();
    assert_eq!(
        runner.dp_of(gaossmon_2).unwrap_or(0),
        2000 + 3000,
        "control: aura must be active on the controller's own turn"
    );

    // Advance to P1's turn and re-tick.
    runner.end_turn();
    runner.game.tick_declarative_effects();

    // The aura must NOT fire on P1's turn (active_when: { your_turn: true }).
    let dp_gaossmon_2 = runner.dp_of(gaossmon_2).unwrap_or(0);
    assert_eq!(
        dp_gaossmon_2,
        2000, // base DP, no buff
        "other Gaossmon must NOT get +3000 DP on opponent's turn; got {}",
        dp_gaossmon_2
    );

    let _ = gaossmon_1;
}

/// Negative (self-exclusion): BT5-008's aura must NOT buff itself —
/// the printed text says "Your OTHER [Gaossmon]", expressed via `other: true`.
#[test]
fn bt5_008_aura_does_not_buff_self() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field, alongside another Gaossmon as a control.
    let gaossmon_1 = runner.place_on_field(0, "BT5-008", None);
    let gaossmon_2 = runner.place_on_field(0, "GAOSSMON-2", None);

    runner.game.tick_declarative_effects();

    // Control: the OTHER Gaossmon IS buffed, proving the aura is active.
    assert_eq!(
        runner.dp_of(gaossmon_2).unwrap_or(0),
        2000 + 3000,
        "control: another Gaossmon must be buffed (proves the aura fired)"
    );

    // On P0's turn, BT5-008 must NOT buff itself (other: true exclusion).
    let dp_self = runner.dp_of(gaossmon_1).unwrap_or(0);
    assert_eq!(
        dp_self,
        2000, // base DP, no self-buff
        "BT5-008 must NOT buff itself (other: true); got {}",
        dp_self
    );
}

// ─── Section 3: Clause 2 behavioral — [Opponent's Turn] no cost reduction ────

/// Structural/runtime: while BT5-008 is on P0's field during P1's turn, the
/// `CannotReduceDigivolveCost` modifier is installed on P1 (the opponent) and
/// NOT on P0.
#[test]
fn bt5_008_installs_cannot_reduce_digivolve_cost_on_opponent_on_their_turn() {
    let mut runner = gaossmon_runner();

    // Place BT5-008 on P0's field.
    runner.place_on_field(0, "BT5-008", None);

    // On P0's own turn, the [Opponent's Turn] flood gate must be inactive.
    runner.game.tick_declarative_effects();
    assert!(
        !runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotReduceDigivolveCost),
        "flood gate must be inactive on the controller's own turn"
    );

    // Advance to P1's turn — the flood gate becomes active.
    runner.end_turn();
    runner.game.tick_declarative_effects();

    assert!(
        runner
            .game
            .modifiers
            .player_has(1, ModifierType::CannotReduceDigivolveCost),
        "opponent must carry CannotReduceDigivolveCost on their turn while BT5-008 is in play"
    );
    assert!(
        !runner
            .game
            .modifiers
            .player_has(0, ModifierType::CannotReduceDigivolveCost),
        "the controller must never receive their own flood gate"
    );
}

/// Integrated: with BT5-008 on P0's field, P1's digivolution-cost reducer is
/// suppressed during P1's turn — the digivolve stays legal, but pays full cost.
#[test]
fn bt5_008_opponent_cannot_reduce_digivolution_costs_while_in_play() {
    let mut runner = cost_reduction_runner();

    // BT5-008 on P0's field; P1 has a base Digimon + a -3 cost reducer.
    runner.place_on_field(0, "BT5-008", Some(0));
    runner.place_on_field(1, "BASE", Some(0));
    runner.place_on_field(1, "REDUCER", Some(0));
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();

    // BT5-008 must NOT hide the opponent's ordinary digivolve from the mask.
    let action = encode_digivolve(0, 0);
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(
        mask[action as usize], 1.0,
        "BT5-008 must not hide the opponent's ordinary digivolve action from the mask"
    );

    // EVO costs 4 to digivolve from level 3. The -3 reducer would normally
    // make it cost 1; BT5-008 suppresses the reduction, so it costs the full 4.
    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(1, 0, 0, PlaySource::ByDigivolve);

    assert!(
        digivolved,
        "opponent's ordinary digivolve must remain legal"
    );
    assert_eq!(
        memory_before - runner.memory(),
        4,
        "BT5-008 must suppress the opponent's -3 digivolution cost reducer (full cost paid)"
    );
}

/// Baseline (negative): without BT5-008 on the field, the opponent's
/// digivolution-cost reducer applies normally — proving the suppression in the
/// integrated test above is caused by BT5-008, not by anything else.
#[test]
fn bt5_008_baseline_without_gaossmon_allows_digivolution_cost_reductions() {
    let mut runner = cost_reduction_runner();

    // No BT5-008 on the field this time.
    runner.place_on_field(1, "BASE", Some(0));
    runner.place_on_field(1, "REDUCER", Some(0));
    runner.end_turn();
    runner.game.enter_main_phase();
    runner.game.set_memory(10);
    runner.game.tick_declarative_effects();

    let memory_before = runner.memory();
    let digivolved = runner
        .game
        .digivolve_from_hand(1, 0, 0, PlaySource::ByDigivolve);

    assert!(
        digivolved,
        "ordinary digivolve should remain legal when no Gaossmon flood gate is active"
    );
    assert_eq!(
        memory_before - runner.memory(),
        1,
        "digivolution cost reducer should apply normally without BT5-008 (4 - 3 = 1)"
    );
}
