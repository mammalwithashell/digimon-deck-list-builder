//! BT23-005 Elizamon — Digimon, Lv.3, Red, DP 1000, Cost 0.
//! Traits: Reptile, LIBERATOR.
//! Evo costs: Lv.2 Red / cost 0.
//!
//! # Card text (cards.json)
//!
//! **Own effect:**
//! [Your Turn] When this Digimon would digivolve into a Digimon card with the
//! [Reptile] or [Dragonkin] trait, reduce the digivolution cost by 1.
//!
//! **Inherited effect:**
//! [Your Turn] This Digimon gets +2000 DP.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT23/Red/BT23_005.cs
//!
//! # Patterns this test covers
//! - D2 — BeforePayCost cost reduction gated on digivolve-target traits
//!   (Clause 1; closed by Phase 2 Track H — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
//! - D4 — declarative aura inherited DP modifier (Clause 2).
//! - §6 Aura "inherited self-DP" pattern: scope: inherited, active_when: your_turn, target: {}
//!
//! # Clause 1 — cost reduction IMPLEMENTED [Phase 2 Track H]
//!
//! The DSL gap (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET) was RESOLVED 2026-05-17.
//! The `cost_target: { ... }` nested predicate evaluates against the digivolve
//! target card during BeforePayCost cost-calc dispatch, and
//! `source_is_cost_target_permanent: true` gates on "THIS permanent (Elizamon)
//! is the one being digivolved into". `trait_has` is a single string, so the
//! Reptile-OR-Dragonkin requirement uses `any_of`. DCGO BT23_005.cs gates on
//! `PermanentCondition` (the digivolving permanent IS Elizamon's) plus
//! `HasProperTrait` (target is a Digimon with Reptile or Dragonkin). Not OPT —
//! DCGO `SetUpActivateClass(..., -1, ...)` (max-count -1 = unlimited).
//!
//! # Clause 2 — inherited [Your Turn] +2000 DP IMPLEMENTED
//!
//! The DSL self-aura (empty target `{}`) with `scope: inherited` sets the
//! static `dp_modifier` field on the Effect. `Game::source_dp_contribution`
//! reads this field for tensor-layer DP computation, gated by the condition
//! (which wraps `active_when: { your_turn: true }`).

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledScope};
use digimon_engine::card_data::EvoCost;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlaySource};
use digimon_engine::permanent::PermanentHandle;

use crate::dsl_card_data::card_data_from_compiled;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a runner with BT23-005 (Elizamon) registered and stacked under a Lv4
/// dummy Digimon on player 0's battle area. Used by the inherited-DP tests.
///
/// Stack layout after `runner_with_elizamon_as_source()`:
///   P0 battle_area[0]: [elizamon (source 0), carrier_lv4 (source 1/top)]
fn runner_with_elizamon_as_source() -> (DebugRunner, PermanentHandle) {
    let elizamon_data = card_data_from_compiled("BT23-005");
    let mut carrier = make_test_card("CARRIER-LV4", "CarrierLv4");
    carrier.level = Some(4);
    carrier.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .add_card(elizamon_data)
        .add_card(carrier)
        .memory(10)
        .start();

    // Place Elizamon on field as the base (source 0).
    let elizamon_handle = runner.place_on_field(0, "BT23-005", Some(0));

    // Simulate digivolving the carrier on top of Elizamon by directly pushing
    // the carrier CardSource into the Elizamon permanent's stack.
    // This mimics `digivolve_from_hand` without the memory/evo-cost machinery —
    // the inherited-DP tests are about post-digivolve state, not the cost path.
    let carrier_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "CARRIER-LV4")
        .expect("CARRIER-LV4 in card_data");
    let next_idx = runner.game.next_card_index();
    let carrier_source = CardSource::new(carrier_data_idx, 0, next_idx);
    let turn = runner.game.turn_count;
    runner.game.players[0].battle_area[elizamon_handle.index as usize]
        .digivolve(carrier_source, turn);

    // After digivolving, the permanent's handle is unchanged (same field index).
    (runner, elizamon_handle)
}

/// A Lv.4 Reptile-trait Digimon that can digivolve from a Lv.3 (evo cost 1).
fn make_reptile_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("REPTILE-LV4", "ReptileDragon");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Reptile".to_string()];
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 0, // Red
        memory_cost: 1,
    }];
    c
}

/// A Lv.4 Dragonkin-trait Digimon that can digivolve from a Lv.3 (evo cost 1).
fn make_dragonkin_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("DRAGONKIN-LV4", "DragonkinDrake");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Dragonkin".to_string()];
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 0, // Red
        memory_cost: 1,
    }];
    c
}

/// A Lv.4 Digimon WITHOUT the Reptile / Dragonkin traits (evo cost 1).
fn make_non_trait_lv4() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PLAIN-LV4", "PlainBeast");
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 5;
    c.traits = vec!["Beast".to_string()];
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        level: 3,
        card_color: 0, // Red
        memory_cost: 1,
    }];
    c
}

/// A non-Elizamon Lv.3 Red Digimon that can serve as a different digivolution
/// source (no cost-reduction effect of its own).
fn make_plain_lv3() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("PLAIN-LV3", "PlainLv3");
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(1000);
    c.play_cost = 0;
    c.colors = vec![CardColor::Red];
    c
}

/// Push a card (by id) into player `p`'s hand and return its hand index.
fn push_to_hand(runner: &mut DebugRunner, p: usize, card_id: &str) -> usize {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} in card_data"));
    let card_index = runner.game.next_card_index();
    runner.game.players[p]
        .hand
        .push(CardSource::new(data_idx, p as u8, card_index));
    runner.game.players[p].hand.len() - 1
}

// ─── Section 1: Structural assertions ────────────────────────────────────────

#[test]
fn bt23_005_compiles_with_cost_reduction_and_inherited_aura_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("BT23-005 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT23-005")
        .expect("BT23-005 in compiled_cards");

    // Two clauses: clause 1 cost_reduction + clause 2 inherited aura.
    assert_eq!(
        compiled.effects.len(),
        2,
        "BT23-005 should have exactly 2 compiled clauses \
         (cost_reduction + inherited aura); got {}",
        compiled.effects.len()
    );
}

#[test]
fn bt23_005_has_a_cost_reduction_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("BT23-005 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT23-005")
        .expect("BT23-005 in compiled_cards");

    let has_cost_reduction = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction { .. })
        )
    });
    assert!(
        has_cost_reduction,
        "BT23-005 must have a Declarative::CostReduction clause"
    );
}

#[test]
fn bt23_005_inherited_aura_clause_has_correct_scope() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("BT23-005 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT23-005")
        .expect("BT23-005 in compiled_cards");

    let aura = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Aura {
            scope, dp_modifier, ..
        }) => Some((*scope, *dp_modifier)),
        _ => None,
    });

    let (scope, dp) = aura.expect("BT23-005 should have a Declarative::Aura clause");
    assert_eq!(
        scope,
        CompiledScope::Inherited,
        "the aura clause must have Inherited scope (active as a digivolution source)"
    );
    assert_eq!(dp, Some(2000), "the aura dp_modifier must be +2000 DP");
}

#[test]
fn bt23_005_cost_reduction_is_not_once_per_turn() {
    let runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("BT23-005 found in embedded DSL pack")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT23-005")
        .expect("BT23-005 in compiled_cards");

    let once_per_turn = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
            once_per_turn,
            ..
        }) => Some(*once_per_turn),
        _ => None,
    });
    assert_eq!(
        once_per_turn,
        Some(false),
        "the cost-reduction clause must NOT be once_per_turn \
         (DCGO SetUpActivateClass max-count -1; printed text has no OPT annotation)"
    );
}

// ─── Section 2: Behavioral — inherited DP positive / negative gate ───────────

/// POSITIVE: When BT23-005 is a digivolution source under a carrier, on the
/// controller's own turn, `source_dp_contribution` must return +2000.
#[test]
fn bt23_005_inherited_dp_active_on_your_turn() {
    let (runner, carrier_handle) = runner_with_elizamon_as_source();

    // Source index 0 = Elizamon (the base), index 1 = carrier (top card).
    let elizamon_src_idx = 0;

    // It is currently player 0's turn (turn 1 starts with player 0).
    assert_eq!(
        runner.turn_player(),
        0,
        "precondition: it is player 0's turn"
    );

    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, elizamon_src_idx);

    assert_eq!(
        contribution, 2000,
        "Elizamon's inherited +2000 DP must contribute on the controller's own turn; \
         got {contribution} instead"
    );
}

/// NEGATIVE: On the opponent's turn, the [Your Turn] gate must suppress the buff.
#[test]
fn bt23_005_inherited_dp_inactive_on_opponents_turn() {
    let (mut runner, carrier_handle) = runner_with_elizamon_as_source();

    // End player 0's turn → player 1's turn begins.
    runner.end_turn();
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: it is now player 1's turn"
    );

    let elizamon_src_idx = 0;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, elizamon_src_idx);

    assert_eq!(
        contribution, 0,
        "Elizamon's inherited +2000 DP must NOT contribute on the opponent's turn \
         ([Your Turn] gate); got {contribution} instead"
    );
}

/// Confirm the carrier's TOP card (index 1) does NOT contribute the +2000 DP —
/// only the Elizamon source slot does.
#[test]
fn bt23_005_top_card_does_not_contribute_elizamon_dp() {
    let (runner, carrier_handle) = runner_with_elizamon_as_source();

    // Stack: [elizamon (0), carrier (1/top)]
    // CARRIER-LV4 has no effects, so its own slot contributes 0.
    let top_src_idx = 1;
    let contribution = runner
        .game
        .source_dp_contribution(carrier_handle, top_src_idx);

    assert_eq!(
        contribution, 0,
        "the carrier's own slot (top card) should not contribute Elizamon's buff"
    );
}

// ─── Section 3: Behavioral — cost reduction on digivolve ─────────────────────
//
// Clause 1 closed by Phase 2 Track H (G-BEFORE-PAY-COST-DIGIVOLVE-TARGET).
// The reduction fires when Elizamon (on field, level 3) is the digivolution
// source and the target hand card is a Digimon with [Reptile] or [Dragonkin].
//
// Each test places Elizamon on field, pushes a Lv.4 target into hand
// (printed evo cost 1), and digivolves through the real `digivolve_from_hand`
// cost path — so cost calculation runs the BeforePayCost scan.

/// POSITIVE: digivolving FROM Elizamon INTO a [Reptile] Lv.4 reduces the
/// digivolution cost by 1 (printed evo cost 1 → effective 0; memory unchanged).
#[test]
fn bt23_005_cost_reduction_fires_digivolving_into_reptile() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("parses")
        .add_card(make_reptile_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let elizamon = runner.place_on_field(0, "BT23-005", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "REPTILE-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, elizamon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "Elizamon must digivolve into REPTILE-LV4 (effective cost 1 - 1 = 0)"
    );

    // Cost was 1 (base evo cost) - 1 (Elizamon reduction) = 0. Memory unchanged.
    assert_eq!(
        runner.game.memory, memory_before,
        "memory must be unchanged after digivolving into a [Reptile] Digimon \
         (1 evo cost - 1 Elizamon reduction = 0)"
    );
}

/// POSITIVE (sibling trait): digivolving FROM Elizamon INTO a [Dragonkin] Lv.4
/// also reduces the cost by 1.
#[test]
fn bt23_005_cost_reduction_fires_digivolving_into_dragonkin() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("parses")
        .add_card(make_dragonkin_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let elizamon = runner.place_on_field(0, "BT23-005", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "DRAGONKIN-LV4");

    let memory_before = runner.game.memory;
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, elizamon.index as usize, PlaySource::ByHand);
    assert!(
        digivolved,
        "Elizamon must digivolve into DRAGONKIN-LV4 (effective cost 1 - 1 = 0)"
    );

    assert_eq!(
        runner.game.memory, memory_before,
        "memory must be unchanged after digivolving into a [Dragonkin] Digimon"
    );
}

/// NEGATIVE (trait gate): digivolving FROM Elizamon INTO a non-Reptile /
/// non-Dragonkin Lv.4 must NOT trigger the cost reduction.
#[test]
fn bt23_005_cost_reduction_does_not_fire_for_non_trait_target() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("parses")
        .add_card(make_non_trait_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let elizamon = runner.place_on_field(0, "BT23-005", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "PLAIN-LV4");

    let memory_before = runner.game.memory;
    let _ =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, elizamon.index as usize, PlaySource::ByHand);

    // Target lacks Reptile/Dragonkin → reduction must not apply; memory drops by 1.
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "target has no [Reptile]/[Dragonkin] trait — reduction must not apply, \
         full evo cost 1 is paid"
    );
}

/// [Your Turn] gate — structural coverage.
///
/// The printed text carries a `[Your Turn]` qualifier on the cost-reduction
/// clause, mirrored by `active_when: { your_turn: true, ... }` in the YAML and
/// by DCGO's `CanUseCondition` (`IsOwnerTurn`). It cannot be exercised by an
/// off-turn digivolve: `digivolve_from_hand` only succeeds for the turn player,
/// and `source_is_cost_target_permanent` only matches when the digivolving
/// permanent IS Elizamon — i.e. its controller is digivolving, which is always
/// that controller's own turn. The gate is therefore structurally always-true
/// for this code path; we assert its presence on the compiled clause instead
/// of driving an impossible game state. The behavioral [Your Turn] timing-gate
/// mechanism itself is covered by
/// `bt23_005_inherited_dp_inactive_on_opponents_turn`.
#[test]
fn bt23_005_cost_reduction_clause_carries_your_turn_gate() {
    use digimon_dsl::compiled::CompiledPredicate;

    let runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("parses")
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("BT23-005")
        .expect("BT23-005 in compiled_cards");

    let active_when = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Declarative(CompiledDeclarativeClause::CostReduction {
                active_when,
                ..
            }) => Some(active_when.clone()),
            _ => None,
        })
        .expect("BT23-005 must have a CostReduction clause")
        .expect("the CostReduction clause must carry an active_when predicate");

    /// Recursively scan a compiled predicate tree for a `your_turn: true` leaf.
    fn has_your_turn(p: &CompiledPredicate) -> bool {
        p.your_turn == Some(true)
            || p.all_of.iter().any(has_your_turn)
            || p.any_of.iter().any(has_your_turn)
    }

    assert!(
        has_your_turn(&active_when),
        "the cost-reduction clause must carry a `your_turn: true` gate \
         (printed [Your Turn] qualifier)"
    );
}

/// NEGATIVE ("THIS Digimon" gate): with Elizamon present on the field as a
/// BYSTANDER, digivolving a DIFFERENT Lv.3 permanent into a [Reptile] target
/// gets NO cost reduction. `source_is_cost_target_permanent` requires Elizamon
/// itself to be the permanent being digivolved.
#[test]
fn bt23_005_cost_reduction_does_not_fire_for_different_source() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("parses")
        .add_card(make_plain_lv3())
        .add_card(make_reptile_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    // Elizamon is on the field but is NOT the digivolution source.
    let _elizamon = runner.place_on_field(0, "BT23-005", Some(0));
    // PLAIN-LV3 is the permanent that actually digivolves.
    let plain = runner.place_on_field(0, "PLAIN-LV3", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "REPTILE-LV4");

    let memory_before = runner.game.memory;
    let _ = runner
        .game
        .digivolve_from_hand(0, hand_idx, plain.index as usize, PlaySource::ByHand);

    // The digivolving permanent is PLAIN-LV3, not Elizamon → reduction must
    // not apply; full evo cost 1 is paid.
    assert_eq!(
        runner.game.memory,
        memory_before - 1,
        "a non-Elizamon digivolution source must not receive Elizamon's \
         cost reduction even when Elizamon is on the field"
    );
}

/// Integrated event-log check: a `Digivolve` event fires when Elizamon
/// digivolves into a [Reptile] target through the reduced-cost path,
/// confirming the digivolution completed.
#[test]
fn bt23_005_cost_reduction_digivolve_emits_digivolve_event() {
    use digimon_engine::events::GameEvent;

    let mut runner = DebugRunner::builder()
        .dsl_card("BT23-005")
        .expect("parses")
        .add_card(make_reptile_lv4())
        .memory(5)
        .start();
    runner.game.turn_count = 1;

    let elizamon = runner.place_on_field(0, "BT23-005", Some(0));
    let hand_idx = push_to_hand(&mut runner, 0, "REPTILE-LV4");

    let cp = runner.event_checkpoint();
    let digivolved =
        runner
            .game
            .digivolve_from_hand(0, hand_idx, elizamon.index as usize, PlaySource::ByHand);
    assert!(digivolved, "digivolve must succeed");

    let events = runner.events_since(cp);
    let digivolve_count = events
        .iter()
        .filter(|e| matches!(e, GameEvent::Digivolve { .. }))
        .count();
    assert!(
        digivolve_count >= 1,
        "a Digivolve event must fire when Elizamon digivolves into the \
         [Reptile] target through the reduced-cost path"
    );
}
