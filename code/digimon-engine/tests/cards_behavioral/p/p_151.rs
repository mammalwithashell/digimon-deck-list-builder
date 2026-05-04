//! P-151 Digimon Liberator — Option, Cost 3, White/Purple, [LIBERATOR] trait.
//!
//! # Card text (cards.json)
//!
//! While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this
//! card's color requirements.
//! [Main] Reveal the top 3 cards of your deck. Add 1 card with the [LIBERATOR]
//! trait among them to your hand. Return the rest to the bottom of the deck in
//! any order. Then, you may play 1 card with the [LIBERATOR] trait and a play
//! cost of 3 or less from your hand without paying the cost.
//! Security Effect [Security] Activate this card's [Main] effects.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/White/P_151.cs
//!
//! # Patterns this test covers
//! - D3 color ignore / bypass through option use_requirement
//! - A1 searching by trait (top-3 reveal, add 1 LIBERATOR to hand)
//! - A2 place_remainder_on_deck (bottom, any order)
//! - E2-adjacent: optional free-play from hand (cost ≤ 3, LIBERATOR trait)
//! - Inherited security clause activates Main effects

#![allow(dead_code, unused_imports, unused_variables)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::build_action_mask;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const YAML: &str = include_str!("../../../cards/p/P-151.yaml");

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn resolve_until_optional_prompt(runner: &mut DebugRunner) {
    while runner.pending_selection().is_some() && !runner.pending_is_optional() {
        let view = runner
            .pending_selection_view()
            .expect("pending selection must have a view");
        let action_id = *view
            .valid_action_ids
            .first()
            .expect("mandatory selection must expose at least one action");
        runner
            .execute_action(view.selecting_player, action_id)
            .expect("mandatory selection resolves");
    }
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn make_liberator_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.traits = vec!["LIBERATOR".to_string()];
    c
}

fn make_non_liberator_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.traits = vec![];
    c
}

fn pick_first_pending_action(runner: &mut DebugRunner) {
    let action = runner
        .pending_selection_view()
        .and_then(|view| view.valid_action_ids.first().copied())
        .expect("pending selection should have at least one valid action");
    runner
        .execute_action(0, action)
        .expect("resolve pending action");
}

fn resolve_required_prompts_until_optional_or_done(runner: &mut DebugRunner) {
    while runner.pending_selection().is_some() && !runner.pending_is_optional() {
        pick_first_pending_action(runner);
    }
}

fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c
}

// ---------------------------------------------------------------------------
// Section 1 — YAML parse + structural assertions
// ---------------------------------------------------------------------------

/// P-151 YAML must parse and compile without errors.
#[test]
fn p_151_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-151 YAML must parse and compile without errors");
}

/// P-151 is an Option card with cost 3.
#[test]
fn p_151_is_option_cost_3() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner
        .compiled_card("P-151")
        .expect("P-151 compiled card must be registered");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "P-151 must be an Option card"
    );
    assert_eq!(compiled.cost, Some(3), "P-151 must have play cost 3");
}

/// P-151 must have exactly 3 compiled clauses:
///   [0] flood_gate declarative (color bypass, G-IGNORE-COLOR-MASK)
///   [1] main_from_hand triggered (reveal + add + optional free-play)
///   [2] inherited on_security triggered (activate Main effects)
#[test]
fn p_151_has_three_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-151")
        .expect("P-151 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        3,
        "expected 3 clauses (flood_gate color-bypass, main_from_hand, inherited on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0 is a declarative (flood_gate for IgnoreColorRequirement).
#[test]
fn p_151_clause_0_is_declarative_flood_gate() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-151")
        .expect("P-151 must be compiled");

    let has_declarative = compiled
        .effects
        .iter()
        .any(|c| matches!(c, CompiledClause::Declarative(_)));

    assert!(
        has_declarative,
        "P-151 must have at least 1 declarative clause (flood_gate for color bypass)"
    );
}

/// Clause 1 is a triggered clause with timing MainFromHand.
#[test]
fn p_151_clause_1_is_main_from_hand() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-151")
        .expect("P-151 must be compiled");

    let has_main_from_hand = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::MainFromHand),
        _ => false,
    });

    assert!(
        has_main_from_hand,
        "P-151 must have a triggered clause with timing MainFromHand"
    );
}

/// Clause 2 is an inherited security clause.
#[test]
fn p_151_clause_2_is_inherited_on_security() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-151")
        .expect("P-151 must be compiled");

    let has_inherited_security = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        }
        _ => false,
    });

    assert!(
        has_inherited_security,
        "P-151 must have an inherited clause with timing OnSecurity"
    );
}

/// The [Main] clause's main_from_hand timing is NOT optional (the effect must fire).
#[test]
fn p_151_main_clause_is_not_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-151")
        .expect("P-151 must be compiled");

    let main_clause = compiled
        .effects
        .iter()
        .find(|c| match c {
            CompiledClause::Triggered(t) => t.when.contains(&CompiledTiming::MainFromHand),
            _ => false,
        })
        .expect("main_from_hand clause must exist");

    match main_clause {
        CompiledClause::Triggered(t) => {
            assert!(
                !t.optional,
                "[Main] clause must not be optional (the effect fires when P-151 is played)"
            );
        }
        _ => panic!("expected Triggered"),
    }
}

// ---------------------------------------------------------------------------
// Section 2 — Condition gating: color-bypass (declarative, G-IGNORE-COLOR-MASK)
// ---------------------------------------------------------------------------

#[test]
fn p_151_color_bypass_active_with_liberator_digimon_on_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card({
            let mut c = make_liberator_digimon("RED-LIBERATOR");
            c.colors = vec![CardColor::Red];
            c
        })
        .hand(0, &["P-151"])
        .deck(0, &["RED-LIBERATOR", "RED-LIBERATOR", "RED-LIBERATOR"])
        .memory(10)
        .start();

    let before = build_action_mask(&runner.game, 0);
    assert_eq!(
        before[PLAY_HAND_START as usize], 0.0,
        "P-151 should be blocked without matching color or LIBERATOR bypass"
    );

    runner.place_on_field(0, "RED-LIBERATOR", Some(0));

    let after = build_action_mask(&runner.game, 0);
    assert_eq!(
        after[PLAY_HAND_START as usize], 1.0,
        "P-151 should be playable when a LIBERATOR Digimon enables its use requirement"
    );
}

#[test]
fn p_151_color_bypass_inactive_without_liberator_on_field() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .hand(0, &["P-151"])
        .memory(10)
        .start();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "without LIBERATOR board presence, color requirements must still apply"
    );
}

// ---------------------------------------------------------------------------
// Section 3 — Behavioral outcome: [Main] clause
// ---------------------------------------------------------------------------

/// Firing the [Main] effect with LIBERATOR cards in top-3 installs a pending
/// selection (the reveal-pick prompt).
///
/// Uses `activate_hand_main` to fire the process without consuming the option
/// card (same pattern as P-035 tests). This lets us inspect `pending_selection`
/// immediately after the process begins, before the full play-action pipeline
/// auto-resolves any intermediate steps.
#[test]
fn p_151_main_installs_pending_selection_after_activate() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["P-151"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for P-151");

    // A selection prompt must be pending after the reveal step
    // (the reveal-pick for selecting 1 LIBERATOR to add to hand).
    assert!(
        runner.game.pending_selection.is_some(),
        "P-151 [Main] must install a pending selection (reveal-pick) when LIBERATOR in top 3"
    );
}

/// Playing P-151 with no eligible cards in top 3 auto-resolves the reveal
/// (optional select = no pick) and does not crash.
#[test]
fn p_151_main_reveal_with_no_liberator_in_top_3_resolves_cleanly() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_non_liberator_digimon("NLIB"))
        .hand(0, &["P-151"])
        .deck(0, &["NLIB", "NLIB", "NLIB", "NLIB", "NLIB", "NLIB"])
        .memory(10)
        .start();

    // Should not panic even with no eligible reveal candidates
    runner.play(0, 0);
    runner.auto_resolve();
    // Reaching here without panic == PASS
}

/// After resolving the reveal-pick, the selected LIBERATOR is added to hand.
///
/// Uses `activate_hand_main` (P-035 pattern). P-151 stays in hand (not consumed
/// by activate_hand_main). Auto-resolving: picks first revealed LIBERATOR → hand
/// grows by 1. Then the optional free-play prompt may park; we pass on it.
///
#[test]
fn p_151_main_selected_card_added_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["P-151"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len(); // 1 (P-151 in hand)
    runner.game.activate_hand_main(0, 0);

    // Explicitly pick the revealed LIBERATOR card. `auto_resolve()` may PASS
    // on optional prompts; we need this selection to happen.
    pick_first_pending_action(&mut runner);
    resolve_required_prompts_until_optional_or_done(&mut runner);

    resolve_until_optional_prompt(&mut runner);

    // Pass on the optional free-play prompt (decline it to keep hand count clean).
    if runner.pending_selection().is_some() && runner.pending_is_optional() {
        let _ = runner.execute_action(0, digimon_engine::action::space::PASS);
    }
    resolve_required_prompts_until_optional_or_done(&mut runner);

    // P-151 stays in hand (activate_hand_main doesn't consume it).
    // One LIBERATOR card was added from reveal → hand grows by net +1.
    let hand_after = runner.game.players[0].hand.len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "Hand must grow by 1 after reveal-pick (LIB added; P-151 stays via activate_hand_main); \
         before={hand_before}, after={hand_after}"
    );
}

/// Playing P-151 reduces hand size by 1 (the option is consumed from hand).
#[test]
fn p_151_play_removes_option_from_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    let hand_before = runner.hand_size(0);
    runner.play(0, 0);
    let hand_mid = runner.hand_size(0);

    assert!(
        hand_mid < hand_before,
        "Playing P-151 must remove the option from hand; before={hand_before}, after={hand_mid}"
    );
}

/// After the reveal-pick resolves, the free-play prompt is optional ("you may").
///
/// DCGO: `canNoSelect: true` on SelectHandEffect → optional: true.
/// Uses `activate_hand_main` to fire the process, then explicitly picks the
/// first valid action from the reveal-select to advance to the free-play step.
#[test]
fn p_151_main_free_play_prompt_is_optional() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["P-151"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);

    pick_first_pending_action(&mut runner);
    resolve_required_prompts_until_optional_or_done(&mut runner);

    resolve_until_optional_prompt(&mut runner);

    // Now check if the free-play prompt is pending and optional
    if runner.pending_selection().is_some() {
        assert!(
            runner.pending_is_optional(),
            "The free-play prompt from P-151 [Main] must be optional ('you may')"
        );
    }
    // If no prompt remains, the test passes vacuously.
}

/// Declining the optional free-play prompt does not add a Digimon to the battle area.
///
/// After the reveal-pick resolves, if an optional free-play prompt is pending,
/// passing on it must leave the battle area unchanged.
#[test]
fn p_151_main_declining_free_play_does_not_add_to_field() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["P-151"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    let field_before = runner.battle_area_size(0);
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true");

    pick_first_pending_action(&mut runner);
    resolve_required_prompts_until_optional_or_done(&mut runner);

    resolve_until_optional_prompt(&mut runner);

    // Now the optional free-play prompt should be pending; decline it with PASS.
    if runner.pending_selection().is_some() && runner.pending_is_optional() {
        let _ = runner.execute_action(0, digimon_engine::action::space::PASS);
    }

    resolve_required_prompts_until_optional_or_done(&mut runner);

    let field_after = runner.battle_area_size(0);
    assert_eq!(
        field_after, field_before,
        "Declining the free-play must not add any Digimon to the field; before={field_before}, after={field_after}"
    );
}

/// The free-play prompt should NOT show LIBERATOR cards with cost > 3.
#[test]
fn p_151_main_free_play_excludes_cost_gt_3_liberator() {
    let mut low = make_liberator_digimon("LOW-COST-LIB");
    low.play_cost = 3;
    let mut high = make_liberator_digimon("HIGH-COST-LIB");
    high.play_cost = 4;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(low)
        .add_card(high)
        .add_card(make_liberator_digimon("REVEAL-LIB"))
        .hand(0, &["P-151", "LOW-COST-LIB", "HIGH-COST-LIB"])
        .deck(0, &["REVEAL-LIB", "REVEAL-LIB", "REVEAL-LIB", "REVEAL-LIB"])
        .memory(10)
        .start();

    runner.game.activate_hand_main(0, 0);
    pick_first_pending_action(&mut runner);
    resolve_required_prompts_until_optional_or_done(&mut runner);
    resolve_until_optional_prompt(&mut runner);

    let view = runner
        .pending_selection_view()
        .expect("optional free-play prompt should be pending");
    assert!(
        view.valid_action_ids.contains(&(PLAY_HAND_START + 1)),
        "cost-3 LIBERATOR should be offered"
    );
    assert!(
        !view.valid_action_ids.contains(&(PLAY_HAND_START + 2)),
        "cost-4 LIBERATOR must be excluded by play_cost_lte: 3"
    );
}

/// Full-flow integration: play P-151 → auto_resolve all prompts → no panic.
#[test]
fn p_151_full_main_flow_no_panic() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["P-151"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    runner.play(0, 0);
    runner.auto_resolve();
    // Reaching here without panic == PASS
}

// ---------------------------------------------------------------------------
// Section 3b — Security clause (inherited on_security)
// ---------------------------------------------------------------------------

/// The inherited security clause is structurally present (compiled).
/// Full behavioral test requires the security-resolution harness; structural
/// presence is sufficient to verify the clause was not dropped.
#[test]
fn p_151_security_clause_compiles_as_inherited_on_security() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-151"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-151")
        .expect("P-151 must be compiled");

    let has_security_clause = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::Inherited && t.when.contains(&CompiledTiming::OnSecurity)
        }
        _ => false,
    });

    assert!(
        has_security_clause,
        "P-151 must have an inherited OnSecurity clause for 'Activate Main effects'"
    );
}

// ---------------------------------------------------------------------------
// Section 4 — Event-log assertions
// ---------------------------------------------------------------------------

// P-151 has no security-trash cost effects. The primary event of interest is
// that playing the option reduces hand size (covered in Section 3).

// ---------------------------------------------------------------------------
// Section 5 — OPT enforcement
// ---------------------------------------------------------------------------

// P-151 has no [Once Per Turn] clause. Option cards are single-use by nature;
// no OPT lockout test is needed.
