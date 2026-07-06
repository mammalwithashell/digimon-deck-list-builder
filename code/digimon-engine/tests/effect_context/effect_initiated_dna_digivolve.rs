//! Phase 2f1 Task 3c — `EffectContext::effect_initiated_dna_digivolve`
//! merges TWO existing battle-area permanents into a single permanent topped
//! with a card from hand. Card-text precedent: BT5-085 Omnimon-style
//! "DNA digivolve from-effect".
//!
//! Reference implementation: `EffectContext::effect_initiated_digivolve`
//! covers single-target effect-initiated digivolves. The DNA variant
//! consumes both source permanents and stacks their card_sources
//! underneath the new top from hand. Stacking order:
//!   target_a.card_sources ++ target_b.card_sources ++ [from_hand]
//! (target_a's stack first, target_b's stack second, then the new top).
//!
//! Both this effect path and the user-action `initiate_dna_digivolve`
//! path delegate to the shared `Game::dna_digivolve_inner` core
//! (added in Phase 2f1 Task 4). The user-action two-stage selection
//! chain is covered by `tests/dna_digivolve_user_action.rs`.

use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use std::sync::Arc;

// ─── Helpers for the DNA-workstream gaps (printed cost + recipe enforcement) ──

/// A leveled/colored Digimon material with no DNA cost of its own.
fn material(id: &str, level: u8, color: CardColor) -> CardData {
    let mut d = make_test_card(id, id);
    d.card_kind = CardKind::Digimon;
    d.level = Some(level);
    d.dp = Some(4000);
    d.colors = vec![color];
    d
}

/// A named DNA requirement half.
fn req(level: u8, color: CardColor) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

/// A DNA-digivolve result card with a single printed DNA recipe:
/// `requirement1` = (req1_level, req1_color), `requirement2` = (req2_level,
/// req2_color), paying `memory_cost` printed DNA cost.
fn dna_result(
    id: &str,
    name: &str,
    req1: DnaRequirement,
    req2: DnaRequirement,
    memory_cost: i16,
) -> CardData {
    let mut d = make_test_card(id, name);
    d.card_kind = CardKind::Digimon;
    d.level = Some(7);
    d.dp = Some(14000);
    d.dna_costs = vec![DnaCost {
        memory_cost,
        requirement1: req1,
        requirement2: req2,
    }];
    d
}

#[test]
fn effect_initiated_dna_digivolve_merges_two_permanents_with_hand_top() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    // Place two source permanents on P0's battle area.
    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);

    // Snapshot pre-call invariants.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        2,
        "precondition: P0 has 2 source permanents"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "precondition: P0 has 1 hand card (the DNA result)"
    );
    assert_eq!(runner.game.memory, 5, "precondition: memory == 5");

    // Capture handles before mutation.
    let source_a_handle = runner.game.players[0].battle_area[handle_a.index as usize]
        .top_card()
        .handle();
    let source_b_handle = runner.game.players[0].battle_area[handle_b.index as usize]
        .top_card()
        .handle();
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    // Drive the new primitive. cost=0 with ignore_requirements=true: memory
    // unchanged, no color/level/material-affinity validation.
    let src_handle = hand_card_handle;
    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, src_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)
    };

    assert!(
        result.is_some(),
        "effect_initiated_dna_digivolve should succeed for valid handles"
    );

    // Both source permanents are consumed and merged into ONE permanent.
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "two source permanents should be replaced by one merged permanent"
    );

    // Result card was consumed from hand.
    assert_eq!(
        runner.game.players[0].hand.len(),
        0,
        "the DNA result card should be removed from hand"
    );

    // Memory unchanged when cost=0 (irrespective of ignore_requirements).
    assert_eq!(
        runner.game.memory, 5,
        "memory must be unchanged when cost=0"
    );

    // Inspect the merged permanent.
    let merged = &runner.game.players[0].battle_area[0];

    // Top of stack is the DNA result card from hand.
    assert_eq!(
        merged.top_card().handle(),
        hand_card_handle,
        "top of merged stack must be the result card from hand"
    );

    // Stack must contain BOTH source handles plus the new top — at least
    // 3 sources total (each source perm contributed exactly its own top card,
    // since they were placed via place_on_field with stack_size=1).
    assert_eq!(
        merged.card_sources.len(),
        3,
        "merged stack must have 3 sources: source_a, source_b, result"
    );

    let source_handles: Vec<_> = merged.card_sources.iter().map(|c| c.handle()).collect();
    assert!(
        source_handles.contains(&source_a_handle),
        "merged stack must contain source A as material"
    );
    assert!(
        source_handles.contains(&source_b_handle),
        "merged stack must contain source B as material"
    );
    assert!(
        source_handles.contains(&hand_card_handle),
        "merged stack must contain the result card on top"
    );

    // Returned handle points to the merged permanent.
    let new_handle = result.unwrap();
    assert_eq!(
        new_handle.player, 0,
        "merged permanent is on P0's battle area"
    );
    assert_eq!(
        new_handle.index as usize, 0,
        "merged permanent occupies the first slot"
    );
}

#[test]
fn effect_initiated_dna_digivolve_preserves_stacked_materials_under_top() {
    // Verify that when the source permanents already have multi-card stacks,
    // both stacks are preserved underneath the new top in deterministic order.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-A-BASE", "DnaSourceABase"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    // Build P0 perm A with two sources (base + top).
    let handle_a = runner.place_on_field(0, "TST-A-BASE", None);
    let data_idx_top_a = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "TST-A")
        .expect("TST-A registered");
    let card_index = runner.game.next_card_index();
    let top_src_a = CardSource::new(data_idx_top_a, 0, card_index);
    let top_a_handle = top_src_a.handle();
    runner.game.players[0].battle_area[0]
        .card_sources
        .push(top_src_a);
    let base_a_handle = runner.game.players[0].battle_area[0].card_sources[0].handle();

    // Build P0 perm B with one source.
    let handle_b = runner.place_on_field(0, "TST-B", None);
    let source_b_handle = runner.game.players[0].battle_area[handle_b.index as usize]
        .top_card()
        .handle();

    // Verify pre-conditions on stacks.
    assert_eq!(runner.game.players[0].battle_area[0].stack_size(), 2);
    assert_eq!(runner.game.players[0].battle_area[1].stack_size(), 1);

    let hand_card_handle = runner.game.players[0].hand[0].handle();
    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)
    };
    assert!(result.is_some(), "primitive succeeded");

    // After the merge: 1 permanent on P0's battle area, with stack of size 4
    // (base_a, top_a, source_b, result) in that order.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    let merged = &runner.game.players[0].battle_area[0];
    assert_eq!(
        merged.card_sources.len(),
        4,
        "stack must concatenate target_a's stack, target_b's stack, then top"
    );

    let handles: Vec<_> = merged.card_sources.iter().map(|c| c.handle()).collect();
    // Canonical stack order (locked in Game::dna_digivolve_inner): target_a's
    // stack first, target_b's second, then hand top. Matches the user-action
    // path (Game::initiate_dna_digivolve -> two-stage selection).
    assert_eq!(handles[0], base_a_handle, "card_sources[0] = target_a base");
    assert_eq!(handles[1], top_a_handle, "card_sources[1] = target_a top");
    assert_eq!(
        handles[2], source_b_handle,
        "card_sources[2] = target_b's only source"
    );
    assert_eq!(
        handles[3], hand_card_handle,
        "card_sources[3] = new top from hand"
    );
}

#[test]
fn effect_initiated_dna_digivolve_returns_none_when_targets_equal() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        // Same handle for both targets — defensive: must reject.
        ctx.effect_initiated_dna_digivolve(handle_a, handle_a, hand_card_handle, 0, true)
    };
    assert!(
        result.is_none(),
        "must reject when target_a == target_b (would consume same permanent twice)"
    );
    // No mutations.
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(runner.game.memory, 5);
}

#[test]
fn effect_initiated_dna_digivolve_returns_none_for_invalid_target() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    // Bogus second target: out-of-range index.
    let bogus_b = digimon_engine::permanent::PermanentHandle {
        player: 0,
        index: 99,
    };

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, bogus_b, hand_card_handle, 0, true)
    };
    assert!(result.is_none(), "must reject when target_b is invalid");
    assert_eq!(runner.game.players[0].battle_area.len(), 1);
    assert_eq!(runner.game.players[0].hand.len(), 1);
    assert_eq!(runner.game.memory, 5);
}

#[test]
fn effect_initiated_dna_digivolve_returns_none_when_hand_card_missing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .memory(5)
        .start();

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);

    // Hand is empty — fabricate a handle that doesn't exist anywhere.
    let bogus_card = digimon_engine::card_source::CardHandle(9999);

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, bogus_card, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, bogus_card, 0, true)
    };
    assert!(
        result.is_none(),
        "must reject when from_hand card is not in any player's hand"
    );
    // Battle area unchanged.
    assert_eq!(runner.game.players[0].battle_area.len(), 2);
    assert_eq!(runner.game.memory, 5);
}

/// A `CardEffect` that grants +1 memory whenever an `OnDnaDigivolve` trigger
/// fires for the permanent it's attached to. The test below uses memory delta
/// as the observable side effect — same pattern as `DigivolveObsMem` in
/// `timing_dispatch.rs`.
struct DnaDigivolveObsMem;
impl CardEffect for DnaDigivolveObsMem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_dna_digivolve(card)
            .name("+1 on dna digivolve")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

#[test]
fn effect_initiated_dna_digivolve_fires_on_dna_digivolve_trigger() {
    // Place TST-A and TST-B on P0's battle area, with TST-DNA-RESULT in hand.
    // Register an OnDnaDigivolve effect against TST-DNA-RESULT that grants +1
    // memory when fired. After the merge, TST-DNA-RESULT sits on top of the
    // merged permanent — so its OnDnaDigivolve effect is in scope regardless
    // of whether Task 4 fires the trigger globally (PlayerBattleArea) or
    // locally (Permanent(merged_handle)).
    //
    // This test FAILS today: `EffectContext::effect_initiated_dna_digivolve`
    // does not yet enqueue OnDnaDigivolve. Task 4 will wire it.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST-A", "DnaSourceA"))
        .add_card(make_test_card("TST-B", "DnaSourceB"))
        .add_card(make_test_card("TST-DNA-RESULT", "DnaResult"))
        .hand(0, &["TST-DNA-RESULT"])
        .memory(5)
        .start();
    runner.register_effect("TST-DNA-RESULT", Arc::new(DnaDigivolveObsMem));

    let handle_a = runner.place_on_field(0, "TST-A", None);
    let handle_b = runner.place_on_field(0, "TST-B", None);
    let hand_card_handle = runner.game.players[0].hand[0].handle();

    let before = runner.game.memory;

    let result = {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card_handle, None, 0);
        ctx.effect_initiated_dna_digivolve(handle_a, handle_b, hand_card_handle, 0, true)
    };
    assert!(
        result.is_some(),
        "precondition: effect_initiated_dna_digivolve should succeed for valid handles"
    );

    let after = runner.game.memory;
    assert_eq!(
        after - before,
        1,
        "OnDnaDigivolve must fire exactly once on the merged permanent \
         (memory before={}, after={}, expected delta=+1)",
        before,
        after
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// G-ENGINE-DNA-RECIPE-ENFORCEMENT (gap 2) — the engine primitive validates
// the result's printed DNA recipe against the two materials, and ABORTS an
// illegal pairing at commit as a backstop. DCGO parity:
// `CardSource.CanJogressFromTargetPermanents`.
// ═══════════════════════════════════════════════════════════════════════════

/// A material pair that DOES satisfy the result's printed recipe (Lv4 Red +
/// Lv4 Purple) merges normally when `ignore_requirements == false`.
#[test]
fn dna_recipe_valid_pair_merges() {
    let mut runner = DebugRunner::builder()
        .add_card(material("MAT-RED", 4, CardColor::Red))
        .add_card(material("MAT-PUR", 4, CardColor::Purple))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    let a = runner.place_on_field(0, "MAT-RED", None);
    let b = runner.place_on_field(0, "MAT-PUR", None);
    let res = runner.game.players[0].hand[0].handle();

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, res, None, 0);
        // ignore_requirements=false → the recipe MUST be checked and satisfied.
        ctx.effect_initiated_dna_digivolve(a, b, res, 0, false)
    };
    assert!(
        merged.is_some(),
        "a recipe-satisfying material pair (Lv4 Red + Lv4 Purple) must merge"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        1,
        "the two materials merged into one permanent"
    );
}

/// A material pair that does NOT satisfy the result's printed recipe (both Red)
/// is REJECTED at commit — the primitive returns None and mutates nothing.
#[test]
fn dna_recipe_illegal_pair_is_rejected_at_commit() {
    let mut runner = DebugRunner::builder()
        .add_card(material("MAT-RED-1", 4, CardColor::Red))
        .add_card(material("MAT-RED-2", 4, CardColor::Red))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    let a = runner.place_on_field(0, "MAT-RED-1", None);
    let b = runner.place_on_field(0, "MAT-RED-2", None);
    let res = runner.game.players[0].hand[0].handle();

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, res, None, 0);
        // ignore_requirements=false → the recipe MUST reject an illegal pairing.
        ctx.effect_initiated_dna_digivolve(a, b, res, 0, false)
    };
    assert!(
        merged.is_none(),
        "an illegal material pair (Red + Red, recipe wants Red + Purple) must be REJECTED"
    );
    assert_eq!(
        runner.game.players[0].battle_area.len(),
        2,
        "battle area unchanged — no merge on an illegal pairing"
    );
    assert_eq!(
        runner.game.players[0].hand.len(),
        1,
        "result card must remain in hand when the pairing is illegal"
    );
}

/// The recipe backstop is ONLY active when `ignore_requirements == false`.
/// `ignore_requirements == true` (DCGO cards that skip the requirement, e.g.
/// the shipped cost-0 Omnimon users) must still merge an off-recipe pair.
#[test]
fn dna_recipe_ignore_requirements_bypasses_recipe() {
    let mut runner = DebugRunner::builder()
        .add_card(material("MAT-RED-1", 4, CardColor::Red))
        .add_card(material("MAT-RED-2", 4, CardColor::Red))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    let a = runner.place_on_field(0, "MAT-RED-1", None);
    let b = runner.place_on_field(0, "MAT-RED-2", None);
    let res = runner.game.players[0].hand[0].handle();

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, res, None, 0);
        // ignore_requirements=true → recipe NOT checked (escape hatch).
        ctx.effect_initiated_dna_digivolve(a, b, res, 0, true)
    };
    assert!(
        merged.is_some(),
        "ignore_requirements=true must bypass the recipe backstop (escape hatch)"
    );
}

/// The recipe accepts EITHER ordering of the two materials (DCGO enumerates
/// both permutations). A Purple + Red pair still satisfies a req1=Red/req2=Purple
/// recipe.
#[test]
fn dna_recipe_accepts_either_material_ordering() {
    let mut runner = DebugRunner::builder()
        .add_card(material("MAT-PUR", 4, CardColor::Purple))
        .add_card(material("MAT-RED", 4, CardColor::Red))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            0,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    // Place Purple first, Red second — the reverse of req1/req2 order.
    let a = runner.place_on_field(0, "MAT-PUR", None);
    let b = runner.place_on_field(0, "MAT-RED", None);
    let res = runner.game.players[0].hand[0].handle();

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, res, None, 0);
        ctx.effect_initiated_dna_digivolve(a, b, res, 0, false)
    };
    assert!(
        merged.is_some(),
        "recipe must accept (Purple, Red) for a req1=Red/req2=Purple recipe (either ordering)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// G-ENGINE-DNA-PRINTED-COST (gap 1) — the effect-initiated DNA verb pays the
// result card's printed `dna_costs[].memory_cost` when the DSL requests
// `cost: printed`. DCGO parity: `PlayCardClass(..., payCost: true)` pays
// `condition.cost`.
// ═══════════════════════════════════════════════════════════════════════════

/// The `pay_printed_dna_cost` primitive helper: with a recipe-satisfying pair,
/// the merge pays the result's printed DNA cost (here 3), so memory drops by 3.
#[test]
fn dna_printed_cost_is_paid_by_primitive() {
    let mut runner = DebugRunner::builder()
        .add_card(material("MAT-RED", 4, CardColor::Red))
        .add_card(material("MAT-PUR", 4, CardColor::Purple))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            3, // printed DNA cost = 3
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    let a = runner.place_on_field(0, "MAT-RED", None);
    let b = runner.place_on_field(0, "MAT-PUR", None);
    let res = runner.game.players[0].hand[0].handle();
    let before = runner.game.memory;

    let merged = {
        let mut ctx = EffectContext::new(&mut runner.game, res, None, 0);
        // Pass the printed cost explicitly (this is what the DSL `cost: printed`
        // lowering will compute for us). ignore_requirements=false so the
        // recipe is validated first.
        let cost = ctx
            .printed_dna_cost_for_pair(a, b, res)
            .expect("recipe-satisfying pair resolves a printed DNA cost");
        ctx.effect_initiated_dna_digivolve(a, b, res, cost, false)
    };
    assert!(merged.is_some(), "recipe-satisfying pair merges");
    assert_eq!(
        before - runner.game.memory,
        3,
        "memory must drop by the result's printed DNA cost (3)"
    );
}

/// `printed_dna_cost_for_pair` returns `None` for an illegal pairing — so a
/// DSL `cost: printed` never charges when the recipe is unsatisfied.
#[test]
fn dna_printed_cost_none_for_illegal_pair() {
    let mut runner = DebugRunner::builder()
        .add_card(material("MAT-RED-1", 4, CardColor::Red))
        .add_card(material("MAT-RED-2", 4, CardColor::Red))
        .add_card(dna_result(
            "RES",
            "Result",
            req(4, CardColor::Red),
            req(4, CardColor::Purple),
            3,
        ))
        .hand(0, &["RES"])
        .memory(5)
        .start();

    let a = runner.place_on_field(0, "MAT-RED-1", None);
    let b = runner.place_on_field(0, "MAT-RED-2", None);
    let res = runner.game.players[0].hand[0].handle();

    let cost = {
        let ctx = EffectContext::new(&mut runner.game, res, None, 0);
        ctx.printed_dna_cost_for_pair(a, b, res)
    };
    assert!(
        cost.is_none(),
        "illegal pair must resolve NO printed DNA cost (recipe unsatisfied)"
    );
}
