//! BT12-031 Imperialdramon: Fighter Mode — Digimon, Lv.6, Blue/Green, DP 13000, Cost 13.
//! Traits: Ancient Dragonkin.
//! Evo: Lv.5 / Cost 5.
//!
//! # Card text (cards.json — verbatim)
//!
//! ```text
//! [When Digivolving] Suspend all of your opponent's Digimon with no
//! digivolution cards. Then, return 1 of your opponent's suspended Digimon
//! to its owner's hand. By returning 1 [Imperialdramon: Dragon Mode] from
//! this Digimon's digivolution cards to its owner's hand, return all of your
//! opponent's suspended Digimon at the bottom of their owners' decks instead.
//!
//! [All Turns] This Digimon gets +1000 DP for each color in its digivolution
//! cards. While there are 2 or more colors in its digivolution cards, it gains
//! ＜Security A. +1＞ and ＜Blocker＞.
//! ```
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT12/Blue/BT12_031.cs
//!
//! # Implementation status
//! - Clause 0a: suspend-all opp Digimon with no digi-cards — IMPLEMENTED
//! - Clause 0b: return 1 opp suspended Digimon to hand — IMPLEMENTED
//! - Clause 0c: alt-cost return Dragon Mode → return-all-to-bottom — BLOCKED
//!   G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME (qa/dsl-vocab-gaps.md)
//! - Clause 1a: +1000 DP per color in digi-cards — IMPLEMENTED
//! - Clause 1b: while 2+ colors → Security A. +1 + Blocker — BLOCKED
//!   G-DSL-SELF-COLOR-COUNT-GTE (qa/dsl-vocab-gaps.md)
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - D1/D4: declarative self-aura with dp_modifier_fn (DigivolutionColorCount formula)
//! - F7-adjacent: for_each suspend over filter (no-digi-card opponents)
//! - A-adjacent: select_opponent_permanent (suspended filter) + return_to_hand
//! - E2-adjacent: optional alt-cost upgrade (BLOCKED)

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
    CompiledTriggeredClause,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helper factories ─────────────────────────────────────────────────────────

/// A minimal level-5 Digimon to act as an opponent stack target.
fn make_opp_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(5000);
    c
}

// ─── Runner factory ──────────────────────────────────────────────────────────

fn fighter_mode() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-031")
        .expect("BT12-031 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-A"))
        .add_card(make_opp_digimon("OPP-B"))
        .add_card(make_opp_digimon("OPP-C"))
        .memory(13)
        .start()
}

/// Fire the `when_digivolving` batch for a permanent `fm` on behalf of player
/// `player_id`. Uses `enqueue_triggered` + `drain_effect_queue` to bypass the
/// play/cost resolution path, matching the pattern in bt16_025.rs.
fn fire_when_digivolving(runner: &mut DebugRunner, fm: digimon_engine::permanent::PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(fm));
    runner.game.drain_effect_queue();
}

// ─── §1 Structural assertions ─────────────────────────────────────────────────

/// BT12-031 must compile to exactly 2 effects:
///   [0] the WhenDigivolving triggered clause
///   [1] the All-Turns self-aura (dp_modifier_fn = DigivolutionColorCount * 1000)
///
/// Clause 0c (alt-cost) and clause 1b (keyword aura) are BLOCKED and omitted.
#[test]
fn bt12_031_compiles_to_two_clauses() {
    let runner = fighter_mode();
    let compiled = runner.compiled_card("BT12-031").expect("BT12-031 compiled");

    assert_eq!(
        compiled.effects.len(),
        2,
        "BT12-031 must have exactly 2 compiled effects: WhenDigivolving triggered + Aura declarative"
    );
}

#[test]
fn bt12_031_clause_0_is_when_digivolving() {
    let runner = fighter_mode();
    let compiled = runner.compiled_card("BT12-031").expect("BT12-031 compiled");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(triggered.len(), 1, "Exactly one triggered clause");
    let clause = triggered[0];
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "WhenDigivolving clause must be FaceUp (own) scope"
    );
    assert!(
        clause.when.contains(&CompiledTiming::WhenDigivolving),
        "Clause must fire on WhenDigivolving"
    );
    assert!(
        !clause.optional,
        "WhenDigivolving clause is not optional — it fires mandatorily"
    );
    assert!(
        !clause.once_per_turn,
        "WhenDigivolving clause has no OPT restriction"
    );
}

#[test]
fn bt12_031_clause_1a_is_declarative_aura() {
    let runner = fighter_mode();
    let compiled = runner.compiled_card("BT12-031").expect("BT12-031 compiled");

    let declaratives: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Declarative(_) => Some(c),
            _ => None,
        })
        .collect();

    assert_eq!(
        declaratives.len(),
        1,
        "Exactly one declarative clause (dp_modifier_fn aura)"
    );
    // We don't assert on the formula internals per RUST_DSL_TEST_API.md §6
    // anti-patterns (do not test CompiledStep contents).
}

// ─── §2 Behavioral — Clause 0a: suspend-all opp with no digi-cards ───────────

/// When BT12-031's WhenDigivolving fires, ALL opponent Digimon with no
/// digivolution cards (stack_size == 1) must become suspended.
#[test]
fn bt12_031_when_digivolving_suspends_all_opp_digimon_with_no_digi_cards() {
    let mut runner = fighter_mode();

    // Two opponent Digimon with no digivolution cards (stack_size == 1).
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    let opp_b = runner.place_on_field(1, "OPP-B", None);

    // Place BT12-031 on player 0's side and fire WhenDigivolving.
    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_when_digivolving(&mut runner, fm);

    // Both opponent Digimon (stack_size == 1 = no digi-cards) must be suspended.
    assert!(
        runner.game.players[1].battle_area[opp_a.index as usize].is_suspended,
        "Opponent Digimon A (no digi-cards) must be suspended after WhenDigivolving"
    );
    assert!(
        runner.game.players[1].battle_area[opp_b.index as usize].is_suspended,
        "Opponent Digimon B (no digi-cards) must be suspended after WhenDigivolving"
    );

    // Auto-resolve the mandatory "return 1 suspended to hand" step.
    runner.auto_resolve().ok();
}

/// Opponent Digimon WITH digivolution cards (stack_size > 1) must NOT be
/// suspended by Clause 0a.
#[test]
fn bt12_031_when_digivolving_does_not_suspend_opp_digimon_with_digi_cards() {
    let mut runner = fighter_mode();

    // Place one opponent Digimon and add a digi-card under it via push_source.
    let opp_with_stack = runner.place_on_field(1, "OPP-A", None);
    runner.push_source(opp_with_stack, "OPP-B"); // stack_size becomes 2

    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_when_digivolving(&mut runner, fm);

    // Opponent's Digimon with digi-cards (stack_size == 2) must NOT be suspended.
    assert!(
        !runner.game.players[1].battle_area[opp_with_stack.index as usize].is_suspended,
        "Opponent Digimon with digi-cards (stack_size 2) must NOT be suspended"
    );
}

/// Mixed scenario: Digimon with no digi-cards are suspended; stacked Digimon
/// are left unsuspended.
#[test]
fn bt12_031_when_digivolving_only_suspends_opponents_with_no_digi_cards() {
    let mut runner = fighter_mode();

    // opp_a: no digi-cards (stack_size == 1) → should be suspended.
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    // opp_b: has 2 digi-cards (stack_size == 3) → should NOT be suspended.
    let opp_b = runner.place_on_field(1, "OPP-B", None);
    runner.push_source(opp_b, "OPP-C");
    runner.push_source(opp_b, "OPP-C");

    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_when_digivolving(&mut runner, fm);

    assert!(
        runner.game.players[1].battle_area[opp_a.index as usize].is_suspended,
        "opp_a (no digi-cards) must be suspended"
    );
    assert!(
        !runner.game.players[1].battle_area[opp_b.index as usize].is_suspended,
        "opp_b (has digi-cards) must NOT be suspended"
    );

    // Auto-resolve the mandatory return-to-hand step for opp_a.
    runner.auto_resolve().ok();
}

// ─── §3 Behavioral — Clause 0b: return 1 opp suspended Digimon to hand ───────

/// After the for_each suspend fires, a pending selection for "return 1 opp
/// suspended Digimon to hand" must be installed.
#[test]
fn bt12_031_when_digivolving_installs_return_selection_after_suspend() {
    let mut runner = fighter_mode();

    let _opp = runner.place_on_field(1, "OPP-A", None); // no digi-cards → will be suspended

    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_when_digivolving(&mut runner, fm);

    // After for_each suspend runs, a selection for returning 1 suspended Digimon
    // to hand must be pending.
    let kind = runner
        .pending_kind()
        .expect("Return selection must be pending");
    assert_eq!(
        kind,
        SelectionKind::OppField,
        "Selection must be OppField (select 1 suspended opponent Digimon)"
    );
}

/// After selecting the suspended Digimon, it must move from the opponent's
/// battle area to their hand.
#[test]
fn bt12_031_when_digivolving_returned_digimon_moves_to_opponents_hand() {
    let mut runner = fighter_mode();

    let _opp = runner.place_on_field(1, "OPP-A", None);

    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_when_digivolving(&mut runner, fm);

    let hand_before = runner.hand_size(1);
    let ba_before = runner.battle_area_size(1);

    let view = runner
        .pending_selection_view()
        .expect("Selection must be pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select target");
    runner.auto_resolve().ok();

    assert_eq!(
        runner.battle_area_size(1),
        ba_before - 1,
        "Opponent battle area must shrink by 1 after return"
    );
    assert_eq!(
        runner.hand_size(1),
        hand_before + 1,
        "Opponent hand must grow by 1 (returned Digimon)"
    );
}

/// When NO opponent Digimon have no digi-cards (all have stacks), no suspensions
/// occur. The return-1-to-hand selection must still fire (it selects from already-
/// suspended Digimon — which in this case means only manually pre-suspended ones,
/// or the selection skips if there are none).
///
/// In this test, no Digimon exist on the opponent's field at all, so the return
/// step has no candidates and should be skipped without error.
#[test]
fn bt12_031_when_digivolving_with_no_opp_digimon_does_not_error() {
    let mut runner = fighter_mode();

    // No opponent Digimon on field.
    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_when_digivolving(&mut runner, fm);

    // for_each found no targets (empty opp field), suspend loop fires 0 times.
    // The return-1-to-hand step has no suspended Digimon → skips.
    // No pending selection should remain.
    runner.auto_resolve().ok();
    assert!(
        runner.pending_kind().is_none(),
        "No pending selection when opp field is empty"
    );
}

// ─── §4 Behavioral — Clause 0c: alt-cost Dragon Mode (BLOCKED) ───────────────

/// BLOCKED on G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME.
///
/// The "By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's
/// digivolution cards to its owner's hand, return all of your opponent's
/// suspended Digimon at the bottom of their owners' decks instead" branch
/// requires:
///   (A) `select_own_sources` with a name-based filter
///       (G-DSL-SELECT-OWN-SOURCES-FILTER, filed under EX4-073).
///   (B) `binding_present` predicate for conditional branching on the
///       selection result (G-DSL-BIND-PRESENT, filed under EX9-066).
///   (C) Return-all-suspended-to-bottom loop after the opt-in cost.
///
/// Until G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME closes, there is no
/// player-visible selection for the Dragon Mode opt-in, and ALL suspended
/// Digimon only ever go to hand (base clause) rather than to the bottom of
/// the deck.
#[test]
#[ignore = "pending: G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME from qa/dsl-vocab-gaps.md — select_own_sources lacks filter predicate; binding_present predicate missing"]
fn bt12_031_alt_cost_dragon_mode_offers_selection_and_returns_all_suspended_to_bottom() {
    // Intended test body:
    //
    // 1. Stack a card named "Imperialdramon: Dragon Mode" under BT12-031
    //    (as a source) via push_source or place_stack.
    // 2. Place 2+ opponent Digimon with no digi-cards.
    // 3. fire_when_digivolving → for_each suspends all 2+ opp Digimon.
    // 4. A selection appears: "Return [Imperialdramon: Dragon Mode] from
    //    digi-cards to hand?" (optional).
    // 5. Player accepts: the Dragon Mode source card moves to hand.
    //    Then ALL suspended opp Digimon go to the BOTTOM of the opponent's deck.
    //    Assert: opp battle_area is empty; opp deck grew by N; Dragon Mode in hand.
    todo!("Pending G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME")
}

#[test]
#[ignore = "pending: G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME from qa/dsl-vocab-gaps.md — declining alt-cost must fall through to base return-1-to-hand"]
fn bt12_031_declining_alt_cost_falls_through_to_return_one_to_hand() {
    // Intended test body:
    //
    // 1. Stack "Imperialdramon: Dragon Mode" under BT12-031.
    // 2. Place 2+ opp Digimon with no digi-cards.
    // 3. fire_when_digivolving → all 2+ suspended.
    // 4. Player declines Dragon Mode selection (PASS).
    // 5. Only 1 opp Digimon returns to hand (base clause); rest stay suspended.
    todo!("Pending G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME")
}

// ─── §5 Behavioral — Clause 1a: +1000 DP per color in digi-cards ─────────────

/// BT12-031 is blue + green (2 colors). With no digivolution sources (only the
/// top card itself), DigivolutionColorCount == 2, so the bonus is 2000 DP.
/// Expected: base 13000 + 2000 = 15000.
#[test]
fn bt12_031_all_turns_dp_bonus_equals_color_count_times_1000_base_card_only() {
    let mut runner = fighter_mode();
    let fm = runner.place_on_field(0, "BT12-031", None);

    // BT12-031 is blue + green on its own card (2 distinct colors in stack).
    let base_dp = 13000;
    let expected_bonus = 2 * 1000; // 2 colors × 1000
    let effective = runner
        .effective_dp(fm)
        .expect("BT12-031 must have an effective DP");
    assert_eq!(
        effective,
        base_dp + expected_bonus,
        "DP must be {} (base) + {} (2 colors × 1000) = {}",
        base_dp,
        expected_bonus,
        base_dp + expected_bonus
    );
}

/// Adding a mono-blue source to the blue+green stack keeps distinct color count
/// at 2 (blue is already present). DP must remain 13000 + 2000 = 15000.
#[test]
fn bt12_031_dp_bonus_does_not_increase_for_duplicate_color_source() {
    let mut runner = fighter_mode();
    let fm = runner.place_on_field(0, "BT12-031", None);
    // Push a source card onto the stack. OPP-A is just any Digimon card —
    // its color contribution depends on the card data. Use another BT12-031
    // (blue+green) to verify duplicate handling.
    runner.push_source(fm, "BT12-031"); // now stack_size == 2, still 2 distinct colors

    // Still 2 distinct colors (blue + green duplicated), DP bonus unchanged.
    let base_dp = 13000;
    let expected = base_dp + 2 * 1000;
    let effective = runner
        .effective_dp(fm)
        .expect("BT12-031 must have an effective DP");
    assert_eq!(
        effective, expected,
        "Adding a same-color source must not increase DP bonus beyond 2 colors"
    );
}

// ─── §6 Behavioral — Clause 1b: 2+ colors → Security A. +1 + Blocker (BLOCKED)

/// BLOCKED on G-DSL-SELF-COLOR-COUNT-GTE.
///
/// The "While there are 2 or more colors in its digivolution cards, it gains
/// Security A. +1 and Blocker" clause requires a `self_color_count_gte: 2`
/// BoolPredicate leaf evaluating the distinct color count of the source
/// permanent's full digivolution stack.
///
/// Cross-referenced: G-DSL-SELF-COLOR-COUNT-GTE was filed under P-117 in
/// qa/dsl-vocab-gaps.md on 2026-05-04 (line 204). BT12-031 clause 1b has the
/// same structural gap.
///
/// Once `self_color_count_gte: 2` lands in the DSL, the two keyword-grant
/// aura clauses (SecurityAttackPlus and Blocker) can be authored. Until then
/// they are BLOCKED per the no-approximations policy.
#[test]
#[ignore = "pending: G-DSL-SELF-COLOR-COUNT-GTE from qa/dsl-vocab-gaps.md — no DSL predicate evaluates self digivolution color count >= N"]
fn bt12_031_gains_security_attack_plus_1_when_2_or_more_colors_in_digi_cards() {
    // Intended test body:
    //
    // 1. Place BT12-031 (blue+green = 2 colors).
    // 2. Assert SecurityAttackPlus modifier (+1) is active on the permanent.
    //    runner.modifiers().has(fm, ModifierType::SecurityAttack, 1)
    todo!("Pending G-DSL-SELF-COLOR-COUNT-GTE")
}

#[test]
#[ignore = "pending: G-DSL-SELF-COLOR-COUNT-GTE from qa/dsl-vocab-gaps.md — same gap as SecurityAttackPlus conditional"]
fn bt12_031_gains_blocker_when_2_or_more_colors_in_digi_cards() {
    // Intended test body:
    //
    // 1. Place BT12-031 (blue+green = 2 colors).
    // 2. Assert Blocker keyword is active on the permanent.
    todo!("Pending G-DSL-SELF-COLOR-COUNT-GTE")
}

#[test]
#[ignore = "pending: G-DSL-SELF-COLOR-COUNT-GTE from qa/dsl-vocab-gaps.md — negative condition: mono-color → no Security A. +1 or Blocker"]
fn bt12_031_does_not_gain_keywords_with_fewer_than_2_colors() {
    // Intended test body:
    //
    // With a hypothetical mono-color digivolution stack, neither SecurityAttackPlus
    // nor Blocker should be granted. Without the predicate we cannot gate the aura,
    // so this test stays blocked.
    todo!("Pending G-DSL-SELF-COLOR-COUNT-GTE")
}
