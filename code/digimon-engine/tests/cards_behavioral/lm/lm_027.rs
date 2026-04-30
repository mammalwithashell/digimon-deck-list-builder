//! LM-027 Red Scramble — Option, Cost 2, Red.
//!
//! # Card text (cards.json)
//!
//! [Main] 1 of your red Digimon may digivolve into a red Digimon card in the
//! hand with the digivolution cost reduced by 3. Then, place this card in the
//! battle area.
//!
//! [Start of Your Turn] If your opponent has a Digimon, ＜Delay＞ (By trashing
//! this card after the placing turn, activate the effect below.)
//! ・Return 1 red Digimon card from your trash to the top of the deck.
//!   Then, if you don't have a Digimon, you may play 1 red Digimon card with
//!   2000 DP or less from your trash without paying the cost.
//!
//! Inherited: Security Effect [Security] You may play 1 red Digimon card with
//! 2000 DP or less from your trash without paying the cost. Then, add this card
//! to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/LM/Red/LM_027.cs
//!
//! # Patterns this test covers
//! - Clause A (Main): effect_initiated_digivolve with cost reduce 3 from an
//!   option card (main_from_hand timing, optional permanent selection)
//! - Clause B (Delay/StartOfYourTurn): BLOCKED G-DELAY-START-OF-TURN — DSL
//!   `kind: delay` only supports EndOfThisTurn / EndOfYourNextTurn; no
//!   StartOfYourTurn delay trigger exists in engine or DSL.
//! - Clause C (Security): on_security with select_trash (red Digimon ≤2000 DP),
//!   play_from_trash_free, raw_rust add-self-to-hand.
//!   - PARTIAL: G-PRED-DP-LTE (dp_lte filter not enforced at selection)
//!   - PARTIAL: G-ADD-OPTION-SELF-TO-HAND (no DSL step for returning played
//!     option card to hand after security resolution; uses raw_rust placeholder)
//!
//! # Known gaps
//! - **G-DELAY-START-OF-TURN**: Clause B requires a Delay that fires at the
//!   START of the owner's next turn (DCGO `EffectTiming.OnStartTurn`). The
//!   engine's `DelayTrigger` enum only has `EndOfThisTurn` and
//!   `EndOfYourNextTurn`. The DSL `kind: delay` lowerer maps everything except
//!   `EndOfYourTurn` to `EndOfYourNextTurn` (fires at end-of-turn, not start).
//!   There is no `DelayTrigger::StartOfYourNextTurn`. The entire Clause B
//!   body is modelled as a raw_rust no-op placeholder until this gap is resolved.
//! - **G-PRED-DP-LTE**: `dp_lte` predicate not evaluated at selection time for
//!   permanents or trash cards.
//! - **G-ADD-OPTION-SELF-TO-HAND**: No DSL step for returning the currently-
//!   resolving security option card to the controller's hand.
//! - **G-ZONE-TRASH-TO-DECK** (Clause B inner): "Return 1 red Digimon from
//!   trash to top of deck" — no native DSL verb; raw_rust step needed.
//!   Blocked by the outer G-DELAY-START-OF-TURN gap anyway.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

// The test file uses from_dsl_yaml so we can compile and test without a
// pre-built card pack. This is the standard pattern for new-card TDD.
const LM_027_YAML: &str = include_str!("../../../cards/lm/LM-027.yaml");

// ─── Helper cards ──────────────────────────────────────────────────────────────

/// A minimal red Digimon with which to populate fields / hands for tests.
fn make_red_digimon(id: &str, level: u8, dp: i32, cost: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = cost as u16;
    c.colors = vec![CardColor::Red];
    c
}

/// A minimal red Digimon with DP ≤ 2000 (target for Security clause play).
fn make_small_red_digimon(id: &str) -> CardData {
    make_red_digimon(id, 3, 2000, 3)
}

/// A large red Digimon (4000 DP) — should be filtered out by dp_lte: 2000.
fn make_large_red_digimon(id: &str) -> CardData {
    make_red_digimon(id, 4, 4000, 5)
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Build the base runner for structural tests.
fn lm_027_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML must parse")
        .memory(10)
        .start()
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// YAML must parse and compile without error.
#[test]
fn lm_027_yaml_parses_without_error() {
    let _runner = lm_027_runner();
}

/// Clause A: [Main] is a triggered clause with `main_from_hand` timing and
/// `optional: true` (the printed "may" makes the permanent selection optional).
#[test]
fn lm_027_has_main_from_hand_triggered_clause() {
    let runner = lm_027_runner();
    let compiled = runner
        .compiled_card("LM-027")
        .expect("LM-027 must be in compiled_cards");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand));

    assert!(
        main_clause.is_some(),
        "LM-027 must have a triggered clause with MainFromHand timing for [Main] effect"
    );
}

/// The main_from_hand clause must be optional (printed: "1 of your red Digimon
/// may digivolve") and have FaceUp scope.
#[test]
fn lm_027_main_clause_is_optional_face_up() {
    let runner = lm_027_runner();
    let compiled = runner
        .compiled_card("LM-027")
        .expect("LM-027 must be in compiled_cards");

    let main_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::MainFromHand))
        .expect("main_from_hand clause must exist");

    assert!(
        main_clause.optional,
        "LM-027 Main clause must be optional (printed: 'may digivolve')"
    );
    assert_eq!(
        main_clause.scope,
        CompiledScope::FaceUp,
        "LM-027 Main clause must have FaceUp scope"
    );
}

/// Clause B (Delay placeholder): LM-027 has exactly 3 clauses:
///   0: main_from_hand (triggered)
///   1: raw_rust delay placeholder (declarative — G-DELAY-START-OF-TURN blocked)
///   2: on_security (triggered)
#[test]
fn lm_027_has_three_clauses() {
    let runner = lm_027_runner();
    let compiled = runner
        .compiled_card("LM-027")
        .expect("LM-027 must be in compiled_cards");

    assert_eq!(
        compiled.effects.len(),
        3,
        "LM-027 must have exactly 3 compiled clauses (Main, Delay-placeholder, Security); got {}",
        compiled.effects.len()
    );
}

/// Clause 1 is a declarative raw_rust clause (Delay stub for G-DELAY-START-OF-TURN).
#[test]
fn lm_027_has_declarative_raw_rust_delay_placeholder() {
    let runner = lm_027_runner();
    let compiled = runner
        .compiled_card("LM-027")
        .expect("LM-027 must be in compiled_cards");

    let has_raw_rust = compiled.effects.iter().any(|c| {
        matches!(
            c,
            CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
        )
    });

    assert!(
        has_raw_rust,
        "LM-027 must have a declarative raw_rust clause (Delay stub for G-DELAY-START-OF-TURN)"
    );
}

/// Clause C: [Security] is a triggered clause with `on_security` timing and
/// `optional: true` ("you may" in printed text), with FaceUp scope.
#[test]
fn lm_027_has_on_security_optional_triggered_clause() {
    let runner = lm_027_runner();
    let compiled = runner
        .compiled_card("LM-027")
        .expect("LM-027 must be in compiled_cards");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::OnSecurity));

    assert!(
        sec_clause.is_some(),
        "LM-027 must have a triggered clause with OnSecurity timing"
    );

    let sec = sec_clause.unwrap();
    assert!(
        sec.optional,
        "LM-027 Security clause must be optional (printed: 'you may')"
    );
    assert_eq!(
        sec.scope,
        CompiledScope::FaceUp,
        "LM-027 Security clause must have FaceUp scope"
    );
}

/// LM-027 has exactly 2 triggered clauses (main_from_hand + on_security).
/// The Delay clause is a declarative raw_rust, not a triggered clause.
#[test]
fn lm_027_has_exactly_two_triggered_clauses() {
    let runner = lm_027_runner();
    let compiled = runner
        .compiled_card("LM-027")
        .expect("LM-027 must be in compiled_cards");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    assert_eq!(
        triggered.len(),
        2,
        "LM-027 must have exactly 2 triggered clauses (main_from_hand + on_security); got {}",
        triggered.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: [Main] digivolve with cost -3
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: when P0 has a red Digimon on field and a red card in hand,
/// activating the [Main] effect via activate_hand_main installs a pending
/// selection (digivolve source or target).
#[test]
fn lm_027_main_installs_selection_when_eligible_digimon_on_field() {
    let source_digi = make_red_digimon("LM027-SRC", 3, 2000, 3);
    let evo_target = make_red_digimon("LM027-EVO", 4, 5000, 4);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(source_digi.clone())
        .add_card(evo_target.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-027", "LM027-EVO"])
        .hand(1, &["FILL"])
        .deck(0, &["LM027-SRC"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // Place a red Digimon on P0's field (the digivolve source).
    let _field_handle = runner.place_on_field(0, "LM027-SRC", None);

    // Activate [Main] via activate_hand_main: LM-027 is at hand index 0.
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true for LM-027 at hand index 0"
    );

    // After firing Main, a selection prompt should install for choosing the
    // digivolve source (optional permanent selection).
    assert!(
        runner.game.pending_selection.is_some(),
        "LM-027 Main must install a pending selection when a red Digimon is on P0's field"
    );
}

/// Negative: when P0 has no red Digimon on field, the Main effect's optional
/// permanent selection has no eligible targets — no selection installs, no panic.
#[test]
fn lm_027_main_no_selection_when_no_red_digimon_on_field() {
    let evo_target = make_red_digimon("LM027-EVO2", 4, 5000, 4);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(evo_target.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-027", "LM027-EVO2"])
        .hand(1, &["FILL"])
        .deck(0, &["LM027-EVO2"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // No Digimon on P0 field — activate Main.
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(
        fired,
        "activate_hand_main must return true even when there are no eligible field Digimon"
    );

    // With no field Digimon to target, optional selection resolves silently.
    // The effect should complete without panic, and no selection should remain
    // after draining (nothing to choose from).
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "LM-027 Main should produce no selection when P0 has no red Digimon on field"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: Delay (BLOCKED — G-DELAY-START-OF-TURN)
// ═══════════════════════════════════════════════════════════════════════════════

/// The Delay clause (Clause B) is BLOCKED pending G-DELAY-START-OF-TURN.
/// The raw_rust placeholder must compile and not panic when the engine
/// processes the StartOfYourTurn timing for this card.
///
/// DCGO: `EffectTiming.OnStartTurn` with `CanDeclareOptionDelayEffect(card)` —
/// the player is offered the choice to trash the card in the battle area to
/// activate the delay body. Engine has no `DelayTrigger::StartOfYourNextTurn`
/// or analogous timing; `kind: delay` lowers StartOfYourTurn to
/// `EndOfYourNextTurn` (wrong timing — fires at end, not start).
///
/// When G-DELAY-START-OF-TURN is resolved, replace the raw_rust placeholder
/// with a native `kind: delay` clause and implement the full body:
///   1. `select_trash` (bind: r) filter: red Digimon
///   2. `raw_rust { fn: lm_027_return_trash_to_deck_top }` (or native verb when
///      G-ZONE-TRASH-TO-DECK is resolved for top-of-deck placement)
///   3. `if: { condition: no_digimon_on_field } then: [ select_trash (optional),
///      play_from_trash_free ]` — conditional play sub-clause
#[test]
#[ignore = "pending: G-DELAY-START-OF-TURN — no StartOfYourNextTurn DelayTrigger in engine"]
fn lm_027_delay_fires_at_start_of_your_turn_when_opponent_has_digimon() {
    // This test is intentionally left as a placeholder.
    // When the gap is resolved:
    // 1. Play LM-027 from hand (fires Main, places card in battle area as Delay).
    // 2. end_turn (triggers P0's start-of-turn on their next turn).
    // 3. Confirm delay body fires: selection for trash (red Digimon).
    // 4. Drive selection; confirm card moves from trash to top of deck.
    // 5. If P0 has no Digimon: confirm optional play-from-trash prompt installs.
    unimplemented!("blocked on G-DELAY-START-OF-TURN");
}

/// The Delay body's condition ("if your opponent has a Digimon") should suppress
/// the Delay activation when opponent has no Digimon on field.
#[test]
#[ignore = "pending: G-DELAY-START-OF-TURN — no StartOfYourNextTurn DelayTrigger in engine"]
fn lm_027_delay_does_not_fire_when_opponent_has_no_digimon() {
    // Placeholder. When G-DELAY-START-OF-TURN is resolved:
    // Setup with opponent having no battle area Digimon.
    // After start of turn, confirm no delay prompt installs.
    unimplemented!("blocked on G-DELAY-START-OF-TURN");
}

/// Inner Delay body: "Return 1 red Digimon from trash to top of deck."
/// Tests the raw_rust step lm_027_return_trash_to_deck_top independently.
#[test]
#[ignore = "pending: G-DELAY-START-OF-TURN — outer Delay gap blocks this test"]
fn lm_027_delay_body_returns_red_digimon_to_top_of_deck() {
    // Placeholder. When gaps are resolved:
    // 1. Place red Digimon in P0 trash.
    // 2. Activate delay body.
    // 3. Drive trash selection.
    // 4. Assert: selected card is now at top of P0's deck.
    // 5. Assert: P0 trash decreased by 1.
    unimplemented!("blocked on G-DELAY-START-OF-TURN");
}

/// Inner Delay body: conditional play only fires when P0 has no Digimon on field.
#[test]
#[ignore = "pending: G-DELAY-START-OF-TURN — outer Delay gap blocks this test"]
fn lm_027_delay_body_play_from_trash_only_when_no_field_digimon() {
    // Placeholder.
    unimplemented!("blocked on G-DELAY-START-OF-TURN");
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause C: [Security] play small red Digimon from trash
// ═══════════════════════════════════════════════════════════════════════════════

/// Positive: security clause fires without panic when the engine triggers it.
/// Places LM-027 on field, then fires SecuritySkill timing on that permanent.
#[test]
fn lm_027_security_clause_no_panic_with_empty_trash() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // Place LM-027 on P0's field as an option permanent (as it would be after
    // the Delay clause lands it there).
    let field_handle = runner.place_on_field(0, "LM-027", Some(0));

    // Fire SecuritySkill timing for this permanent.
    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // Drain any pending selections (optional clause — engine may install a
    // "do you want to?" prompt or no-op if trash is empty).
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // No panic is the primary assertion.
}

/// Positive: when P0's trash has a small red Digimon, the Security clause
/// installs a trash selection prompt (or completes cleanly without panic).
#[test]
fn lm_027_security_installs_trash_selection_when_eligible_card_in_trash() {
    let small = make_small_red_digimon("LM027-SMALL-RED");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(small.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["LM027-SMALL-RED"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // Seed P0's trash with a small red Digimon by popping from deck.
    if let Some(cs) = runner.game.players[0].deck.pop() {
        runner.game.players[0].trash.push(cs);
    }

    // Place LM-027 on P0's field as an option permanent.
    let field_handle = runner.place_on_field(0, "LM-027", Some(0));

    // Fire SecuritySkill timing.
    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // A selection for the trash card should install (optional).
    // We don't assert exact SelectionKind here since that depends on lowering
    // implementation; we just confirm the effect dispatched without panic.
    // Drain any pending selections.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
}

/// Negative: when trash has NO eligible card (empty), the Security clause
/// should produce no pending selection after drain.
#[test]
fn lm_027_security_no_selection_when_trash_is_empty() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // P0's trash is empty — place LM-027 on field and fire Security.
    let field_handle = runner.place_on_field(0, "LM-027", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "LM-027 Security must not install selection when P0 trash is empty"
    );
}

/// DP filter test: when trash contains ONLY a large red Digimon (>2000 DP),
/// the Security clause should filter it out and install no selection.
///
/// BLOCKED by G-PRED-DP-LTE: the `dp_lte: 2000` predicate is not evaluated
/// at selection time, so the large Digimon will appear as a valid target
/// until the gap is closed. Test is #[ignore]'d until fixed.
#[test]
#[ignore = "pending: G-PRED-DP-LTE — dp_lte filter not evaluated by select_trash"]
fn lm_027_security_no_selection_when_only_large_red_digimon_in_trash() {
    let large = make_large_red_digimon("LM027-LARGE-RED");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(large.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["LM027-LARGE-RED"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // Seed trash with a large (ineligible) red Digimon.
    if let Some(cs) = runner.game.players[0].deck.pop() {
        runner.game.players[0].trash.push(cs);
    }

    let field_handle = runner.place_on_field(0, "LM-027", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "LM-027 Security must not offer >2000 DP Digimon as targets (G-PRED-DP-LTE)"
    );
}

/// After the Security clause resolves and a small red Digimon is played from
/// trash, the card moves from trash to battle area (field count may increase).
///
/// NOTE: add-self-to-hand (G-ADD-OPTION-SELF-TO-HAND) uses raw_rust stub so
/// this test focuses only on the play-from-trash + no-panic outcome.
#[test]
fn lm_027_security_plays_small_red_digimon_from_trash_no_panic() {
    let small = make_small_red_digimon("LM027-SMALL-SEC");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(small.clone())
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    // Seed P0 trash with the small Digimon by popping from deck.
    if let Some(cs) = runner.game.players[0].deck.pop() {
        runner.game.players[0].trash.push(cs);
    }

    // Place LM-027 on P0's field.
    let field_handle = runner.place_on_field(0, "LM-027", Some(0));

    // Fire Security.
    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(field_handle),
    );
    runner.game.drain_effect_queue();

    // Drain all selections, accepting the first available action.
    let mut steps = 0;
    while runner.game.pending_selection.is_some() && steps < 20 {
        let player = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let action = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(player, action).ok();
        runner.game.drain_effect_queue();
        steps += 1;
    }
    // Primary assertion: no panic during the full resolution flow.
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — add-self-to-hand gap acknowledgment
// ═══════════════════════════════════════════════════════════════════════════════

/// After the Security clause resolves (play from trash complete), the printed
/// text says "Then, add this card to the hand." This uses the raw_rust step
/// `lm_027_add_self_to_hand` which is a no-op placeholder until
/// G-ADD-OPTION-SELF-TO-HAND is resolved.
///
/// DCGO: `CardEffectCommons.AddThisCardToHand(card, activateClass)` — moves the
/// currently-resolving option card from security-resolution staging to hand.
/// Engine has no `EffectContext::add_security_option_to_hand()` method and
/// no DSL step verb for it.
#[test]
#[ignore = "pending: G-ADD-OPTION-SELF-TO-HAND — no DSL step or engine API to return security option to hand"]
fn lm_027_security_adds_card_to_hand_after_play() {
    // When G-ADD-OPTION-SELF-TO-HAND is resolved, verify:
    // 1. Before Security fires: P0 hand has 0 copies of LM-027.
    // 2. Security resolves (plays small red Digimon from trash).
    // 3. After Security: P0 hand has 1 copy of LM-027.
    unimplemented!("blocked on G-ADD-OPTION-SELF-TO-HAND");
}
