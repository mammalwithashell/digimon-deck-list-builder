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
//! - Clause 0c: alt-cost return Dragon Mode → return-all-to-bottom — IMPLEMENTED
//!   (G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME closed 2026-05-21 — the
//!   `return_selected_sources_to_hand` verb / `EffectContext::return_card_source_to_hand`
//!   returns a selected own digivolution-stack source to its owner's hand).
//! - Clause 1a: +1000 DP per color in digi-cards — IMPLEMENTED
//! - Clause 1b: while 2+ colors → Security A. +1 + Blocker — IMPLEMENTED via a
//!   self-aura with `while_condition: { self_color_count_gte: 2 }`. BT12-031's
//!   top card is permanently blue+green, so the condition holds for every
//!   reachable state of the card — equivalent to DCGO's full-stack count.
//!
//! # Patterns this test covers (RUST_DSL_TEST_API.md §4.3)
//! - D1/D4: declarative self-aura with dp_modifier_fn (DigivolutionColorCount formula)
//! - F7-adjacent: for_each suspend over filter (no-digi-card opponents)
//! - A-adjacent: select_opponent_permanent (suspended filter) + return_to_hand
//! - E2: optional alt-cost upgrade — select_own_sources (name-filtered) +
//!   return_selected_sources_to_hand, branched by binding_count_eq

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

/// BT12-031 must compile to exactly 3 effects:
///   [0] the WhenDigivolving triggered clause
///   [1] the All-Turns self-aura (dp_modifier_fn = DigivolutionColorCount * 1000)
///   [2] the conditional Security A. +1 / Blocker self-aura (while_condition)
///
/// Clause 0c (alt-cost) is implemented via `return_selected_sources_to_hand`.
#[test]
fn bt12_031_compiles_to_three_clauses() {
    let runner = fighter_mode();
    let compiled = runner.compiled_card("BT12-031").expect("BT12-031 compiled");

    assert_eq!(
        compiled.effects.len(),
        3,
        "BT12-031 must have 3 compiled effects: WhenDigivolving triggered + 2 declarative auras"
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
fn bt12_031_has_two_declarative_auras() {
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
        2,
        "Two declarative auras: dp_modifier_fn aura + conditional Security A./Blocker aura"
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

// ─── §4 Behavioral — Clause 0c: alt-cost Dragon Mode ─────────────────────────
//
// G-DSL-COST-RETURN-SELF-DIGI-CARD-BY-NAME (closed 2026-05-21): the
// `return_selected_sources_to_hand` verb (engine method
// `EffectContext::return_card_source_to_hand`) returns a selected own
// digivolution-stack source card to its owner's hand. BT12-031's Step C —
// "By returning 1 [Imperialdramon: Dragon Mode] from this Digimon's
// digivolution cards to its owner's hand, return all of your opponent's
// suspended Digimon at the bottom of their owners' decks instead." — is
// authored as a `select_own_sources` (min 0, max 1) name-filtered to
// "Imperialdramon: Dragon Mode", branching on `binding_count_eq`.

use digimon_engine::action::space::{encode_source_select, PASS};

/// A card named "Imperialdramon: Dragon Mode" — pushed as a digivolution
/// source under BT12-031 so the `name_contains` filter can pick it.
fn make_dragon_mode() -> CardData {
    let mut c = make_test_card("DRAGON-MODE", "Imperialdramon: Dragon Mode");
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(11000);
    c
}

/// ACCEPT path: with an "Imperialdramon: Dragon Mode" card in BT12-031's
/// digivolution stack and 2 suspended opponent Digimon, accepting the optional
/// `select_own_sources` pick returns the Dragon Mode source to its owner's
/// hand AND bottom-decks every suspended opponent Digimon (the "instead"
/// outcome — none of them go to hand).
#[test]
fn bt12_031_alt_cost_dragon_mode_offers_selection_and_returns_all_suspended_to_bottom() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-031")
        .expect("BT12-031 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-A"))
        .add_card(make_opp_digimon("OPP-B"))
        .add_card(make_dragon_mode())
        .memory(13)
        .start();

    // Two opponent Digimon with no digivolution cards → both get suspended.
    let opp_a = runner.place_on_field(1, "OPP-A", None);
    let opp_b = runner.place_on_field(1, "OPP-B", None);

    // BT12-031 with an Imperialdramon: Dragon Mode card in its stack.
    let fm = runner.place_on_field(0, "BT12-031", None);
    let dragon_mode = runner.push_source(fm, "DRAGON-MODE");

    let opp_deck_before = runner.deck_size(1);
    let p0_hand_before = runner.hand_size(0);

    fire_when_digivolving(&mut runner, fm);

    // Both opponents suspended by Step A.
    assert!(runner.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_b.index as usize].is_suspended);

    // Step B: the optional Dragon Mode source pick is pending.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 0,
            max: 1,
            picked: 0,
        }),
        "the optional Dragon Mode `select_own_sources` pick must be pending"
    );

    // Accept: pick the Dragon Mode source (source_index 0, below BT12-031).
    let pick = encode_source_select(fm.index as u16, 0).expect("Dragon Mode source action");
    runner
        .execute_action(0, pick)
        .expect("pick the Dragon Mode source");
    runner.auto_resolve().ok();

    // The Dragon Mode source returned to its owner's (P0's) hand.
    assert_eq!(
        runner.hand_size(0),
        p0_hand_before + 1,
        "the Dragon Mode source returns to its owner's hand"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.handle() == dragon_mode),
        "P0's hand holds the returned Dragon Mode card"
    );

    // ALL suspended opponent Digimon went to the BOTTOM of the deck — the
    // "instead" outcome. The opponent battle area is now empty.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "every suspended opponent Digimon is bottom-decked — opp battle area empty"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before + 2,
        "both bottom-decked Digimon land in their owner's deck"
    );
}

/// DECLINE path: with an "Imperialdramon: Dragon Mode" card available but the
/// optional `select_own_sources` pick PASSed, the alt-cost does not fire — the
/// base outcome runs: exactly 1 opponent suspended Digimon returns to hand,
/// the Dragon Mode source stays untouched in the stack.
#[test]
fn bt12_031_declining_alt_cost_falls_through_to_return_one_to_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT12-031")
        .expect("BT12-031 in embedded DSL pack")
        .add_card(make_opp_digimon("OPP-A"))
        .add_card(make_opp_digimon("OPP-B"))
        .add_card(make_dragon_mode())
        .memory(13)
        .start();

    let opp_a = runner.place_on_field(1, "OPP-A", None);
    let opp_b = runner.place_on_field(1, "OPP-B", None);

    let fm = runner.place_on_field(0, "BT12-031", None);
    let dragon_mode = runner.push_source(fm, "DRAGON-MODE");

    let opp_hand_before = runner.hand_size(1);
    let opp_deck_before = runner.deck_size(1);

    fire_when_digivolving(&mut runner, fm);

    // Both opponents suspended by Step A.
    assert!(runner.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_b.index as usize].is_suspended);

    // Step B pending — decline it via PASS.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::SourceMulti {
            min: 0,
            max: 1,
            picked: 0,
        })
    );
    runner
        .execute_action(0, PASS)
        .expect("decline the optional Dragon Mode pick");

    // Step C base outcome: the mandatory "return 1 opp suspended Digimon to
    // hand" selection is now pending.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "declining the alt-cost falls through to the base return-1-to-hand selection"
    );
    let view = runner
        .pending_selection_view()
        .expect("base return selection pending");
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("return 1 suspended opponent Digimon");
    runner.auto_resolve().ok();

    // Exactly 1 opponent Digimon returned to hand; the other stays suspended
    // on the field. Nothing was bottom-decked.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "only 1 opponent Digimon returned to hand — the rest stay on the field"
    );
    assert_eq!(
        runner.hand_size(1),
        opp_hand_before + 1,
        "exactly 1 suspended opponent Digimon returns to its owner's hand"
    );
    assert_eq!(
        runner.deck_size(1),
        opp_deck_before,
        "the decline path bottom-decks nothing"
    );

    // The Dragon Mode source is untouched — still in BT12-031's stack.
    assert!(
        runner.game.players[0].battle_area[fm.index as usize]
            .card_sources
            .iter()
            .any(|c| c.handle() == dragon_mode),
        "declining the alt-cost leaves the Dragon Mode source in the stack"
    );
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

// ─── §6 Behavioral — Clause 1b: 2+ colors → Security A. +1 + Blocker ─────────
//
// The "While there are 2 or more colors in its digivolution cards, it gains
// Security A. +1 and Blocker" clause is authored as a self-aura with
// `while_condition: { self_color_count_gte: 2 }`. The condition is evaluated
// against the carrier permanent (PredicateSubject::Permanent), reading the
// synthesized top-card colors. BT12-031's printed colors are blue+green, so
// the condition holds for every reachable state of the card.

use digimon_engine::enums::{Keyword, ModifierType};
use digimon_engine::permanent::PermanentHandle;

/// Fire the field-entry (OnDigivolve) install path for the `while_condition`
/// aura on permanent `fm`.
fn fire_aura_install(runner: &mut DebugRunner, fm: PermanentHandle) {
    runner
        .game
        .enqueue_triggered(EffectTiming::OnDigivolve, TriggerSource::Permanent(fm));
    runner.game.drain_effect_queue();
}

/// Overwrite the registered BT12-031 card data colors so a placed permanent
/// synthesizes a mono-color identity. Used only by the negative gate test.
fn force_bt12_031_mono_color(runner: &mut DebugRunner) {
    use digimon_engine::enums::CardColor;
    let idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "BT12-031")
        .expect("BT12-031 registered");
    runner.game.card_data[idx].colors = vec![CardColor::Blue];
}

/// POSITIVE: BT12-031 (blue+green = 2 colors) gains Security A. +1 from the
/// `while_condition` aura.
#[test]
fn bt12_031_gains_security_attack_plus_1_when_2_or_more_colors_in_digi_cards() {
    let mut runner = fighter_mode();
    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_aura_install(&mut runner, fm);

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(fm, ModifierType::SecurityAttackChange),
        1,
        "BT12-031 (2 colors) must gain Security A. +1 from the while_condition aura"
    );
}

/// POSITIVE: BT12-031 (blue+green = 2 colors) gains Blocker from the
/// `while_condition` aura.
#[test]
fn bt12_031_gains_blocker_when_2_or_more_colors_in_digi_cards() {
    let mut runner = fighter_mode();
    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_aura_install(&mut runner, fm);

    assert!(
        runner.game.has_keyword(fm, Keyword::Blocker),
        "BT12-031 (2 colors) must gain <Blocker> from the while_condition aura"
    );
}

/// NEGATIVE (color gate): with a mono-color BT12-031 identity, the
/// `while_condition: { self_color_count_gte: 2 }` gate fails, so neither
/// Security A. +1 nor Blocker is installed.
#[test]
fn bt12_031_does_not_gain_keywords_with_fewer_than_2_colors() {
    let mut runner = fighter_mode();
    force_bt12_031_mono_color(&mut runner);

    let fm = runner.place_on_field(0, "BT12-031", None);
    fire_aura_install(&mut runner, fm);

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(fm, ModifierType::SecurityAttackChange),
        0,
        "mono-color BT12-031 must NOT gain Security A. +1 (self_color_count_gte gate fails)"
    );
    assert!(
        !runner.game.has_keyword(fm, Keyword::Blocker),
        "mono-color BT12-031 must NOT gain <Blocker> (self_color_count_gte gate fails)"
    );
}
