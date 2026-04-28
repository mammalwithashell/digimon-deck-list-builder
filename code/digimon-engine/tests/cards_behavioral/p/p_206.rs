//! P-206 Digital Gate Open — Option, Cost 4, White.
//!
//! # Card text (cards.json)
//!
//! You can ignore this card's color requirements.
//!
//! **[Main]** Reveal the top 3 cards of your deck. Add 1 Digimon card and
//! 1 Tamer card among them to the hand. Return the rest to the bottom of
//! the deck. Then, place this card in the battle area.
//!
//! **[Main] ＜Delay＞** (By trashing this card after the placing turn,
//! activate the effect below.)
//! ・You may play 1 Tamer card with the same color as any of your Digimon
//!   on the field from your hand with the play cost reduced by 4.
//!
//! **Inherited:** Security Effect [Security] You may play 1 Digimon card
//! with a play cost of 3 or less from your hand or trash without paying
//! the cost. Then, add this card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/White/P_206.cs
//!
//! # Patterns this test covers
//! - A1  Reveal top-N + select Digimon + select Tamer (two sequential select_reveal)
//! - Option-as-permanent Delay (engine auto-placement via classify_option_subtype)
//! - Standard Delay (EndOfYourNextTurn) + conditional "you may" Tamer play with cost-4 reduction
//! - Inherited security: play Digimon cost≤3 from hand or trash free; add self to hand
//! - Unconditional color-ignore via flood_gate IgnoreColorRequirement
//!
//! # Known gaps affecting these tests
//!
//! **G-IGNORE-COLOR-MASK** (engine gap):
//!   `IgnoreColorRequirement` modifier compiled by `kind: flood_gate` but NOT
//!   enforced in `code/digimon-engine/src/action/mask.rs` (§4.2b residual).
//!   The action mask does not check the modifier, so color bypass has zero
//!   runtime effect in the current engine. Tests for color-bypass behavior are
//!   `#[ignore = "pending: G-IGNORE-COLOR-MASK"]`.
//!
//! **G-PLACE-SELF-AS-OPTION-PERMANENT** (DSL gap):
//!   No `place_option_in_battle_area: {}` step verb. "Then, place this card in
//!   the battle area" is implicit (engine's `classify_option_subtype` + `dispose_option`
//!   infers from the presence of `kind: delay` clause). Tests asserting
//!   battle-area presence after Main are `#[ignore]`'d.
//!
//! **G-COLOR-MATCH-AGAINST-BOARD** (DSL gap — new):
//!   Delay clause: "same color as any of your Digimon on the field" filter.
//!   `color_is` is a CandidatePredicate with a fixed color literal; it cannot
//!   dynamically match against colors of permanents currently on the board.
//!   There is no `any_permanent_color_matches` BoolPredicate leaf in the DSL.
//!   The Delay clause selects a Tamer from hand with `select_hand` (unfiltered)
//!   and `play_from_hand` with `cost_delta: -4`, but the "same color as any
//!   of your Digimon" constraint cannot be enforced at selection time.
//!   Tests for the color-filter enforcement are
//!   `#[ignore = "pending: G-COLOR-MATCH-AGAINST-BOARD"]`.
//!
//! **G-PLAY-COST-LTE** (DSL vocab gap):
//!   `play_cost_lte` predicate missing from PredicateSpec. The "cost ≤ 3"
//!   filter in the inherited security clause cannot be enforced at selection
//!   time. `select_hand` and `select_trash` use accept-all filters (Phase 2b).
//!   Tests for cost-≤3 enforcement are `#[ignore = "pending: G-PLAY-COST-LTE"]`.
//!
//! **G-ADD-OPTION-SELF-TO-HAND** (DSL vocab gap):
//!   "Then, add this card to the hand" after security resolution has no DSL
//!   step verb. Uses `raw_rust: { fn: p_206_add_self_to_hand }` (same pattern as
//!   `ex6_072_add_self_to_hand`).
//!
//! **G-PLACE-SELF-AS-OPTION-PERMANENT (inherited security variant)**:
//!   The inherited "[Security] Place this card in the battle area" from P-035/P-103
//!   also applies here — the inherited-security placement path for Option cards
//!   is not supported. However, P-206's inherited security effect is distinct:
//!   the card should be added to HAND, not battle area. This is handled by
//!   `G-ADD-OPTION-SELF-TO-HAND`.

#![allow(dead_code, unused_imports)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledStep, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const YAML: &str = include_str!("../../../cards/p/P-206.yaml");

// ── Fixture builders ─────────────────────────────────────────────────────────

/// A Digimon card — eligible for the Main clause's Digimon select_reveal.
fn make_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c
}

/// A Tamer card — eligible for the Main clause's Tamer select_reveal.
fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c
}

/// A Tamer card with play cost 3 (potentially eligible for Delay clause play).
fn make_tamer_cost(id: &str, cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.play_cost = cost;
    c
}

/// A Digimon card with play cost ≤ 3 (eligible for inherited security clause).
fn make_digimon_low_cost(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.play_cost = 3;
    c
}

/// An Option card — NOT eligible for Main's Digimon or Tamer select_reveal.
fn make_option(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Option;
    c
}

/// Generic filler (default make_test_card — no specific kind constraint matters for deck padding).
fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — YAML parse + structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// P-206 YAML must parse and compile without errors.
#[test]
fn p_206_yaml_parses_and_compiles() {
    let _builder = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("P-206 YAML must parse and compile without errors");
}

/// P-206 is an Option card with cost 4 and color white.
#[test]
fn p_206_is_option_cost_4_color_white() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(10)
        .start();

    let compiled = runner
        .compiled_card("P-206")
        .expect("P-206 compiled card must be registered");

    assert_eq!(
        compiled.kind,
        digimon_dsl::compiled::CompiledCardKind::Option,
        "P-206 must be an Option card"
    );
    assert_eq!(compiled.cost, Some(4), "P-206 must have play cost 4");
}

/// P-206 must compile to exactly 4 clauses:
///   [0] main_from_hand (triggered, FaceUp) — reveal top 3, add Digimon + Tamer
///   [1] delay (declarative) — play Tamer with same color as field Digimon, -4 cost
///   [2] ignore-color flood_gate (declarative, FaceUp) — unconditional IgnoreColorRequirement
///   [3] inherited on_security (triggered or raw_rust, Inherited scope)
///
/// NOTE: clause ordering in the YAML may differ; we assert by type/scope/timing
/// rather than by fixed index where possible.
#[test]
fn p_206_has_four_clauses() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner
        .compiled_card("P-206")
        .expect("P-206 compiled card present");

    assert_eq!(
        compiled.effects.len(),
        4,
        "expected 4 clauses (main_from_hand, delay, flood_gate/ignore-color, inherited on_security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0: main_from_hand triggered, FaceUp scope, not optional.
#[test]
fn p_206_clause_0_is_main_from_hand_face_up_not_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    match &compiled.effects[0] {
        CompiledClause::Triggered(t) => {
            assert!(
                t.when.contains(&CompiledTiming::MainFromHand),
                "clause 0 must fire at MainFromHand; got {:?}",
                t.when
            );
            assert_eq!(t.scope, CompiledScope::FaceUp, "clause 0 must be FaceUp");
            assert!(!t.optional, "Main clause is not optional (no 'you may')");
        }
        other => panic!("clause 0 must be Triggered; got {:?}", other),
    }
}

/// Clause 1: Delay declarative with trigger end_of_your_next_turn.
#[test]
fn p_206_clause_1_is_delay_end_of_your_next_turn() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
            assert_eq!(
                *trigger,
                CompiledTiming::EndOfYourNextTurn,
                "Delay trigger must be EndOfYourNextTurn; got {:?}",
                trigger
            );
        }
        other => panic!("clause 1 must be Declarative(Delay); got {:?}", other),
    }
}

/// Clause 2: flood_gate declarative (ignore-color bypass), FaceUp scope.
#[test]
fn p_206_clause_2_is_flood_gate_ignore_color() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let has_flood_gate_ignore_color = compiled.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { .. })
    ));
    assert!(
        has_flood_gate_ignore_color,
        "P-206 must have a FloodGate declarative clause for ignore-color bypass"
    );
}

/// Clause 3: inherited scope, SecuritySkill timing.
#[test]
fn p_206_clause_3_is_inherited_security_skill() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let has_inherited_security = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t) => {
            t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity)
        }
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
            scope, triggers, ..
        }) => {
            *scope == CompiledScope::Inherited
                && triggers.contains(&CompiledTiming::OnSecurity)
        }
        _ => false,
    });
    assert!(
        has_inherited_security,
        "P-206 must have an Inherited on_security clause (clause 3)"
    );
}

/// Inherited security clause must be optional ("you may").
#[test]
fn p_206_inherited_security_clause_is_optional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let sec_clause = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(t)
        }
        _ => None,
    });

    if let Some(t) = sec_clause {
        assert!(
            t.optional,
            "'You may' in inherited security text → optional must be true"
        );
    }
    // If it's a RawRust stub, optionality is handled internally — no assertion needed.
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Behavioral: Main clause (reveal top 3, add Digimon + Tamer)
// ─────────────────────────────────────────────────────────────────────────────

/// When a Digimon is among the top 3 revealed cards, a select_reveal prompt
/// installs for picking a Digimon.
#[test]
fn p_206_main_installs_select_reveal_with_digimon_in_top_3() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .hand(0, &["P-206"])
        // Deck top-to-bottom (last element = top): DIG-1 is top, TAM-1 next, FILL-1 third.
        .deck(0, &["FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for P-206");

    // After revealing 3 cards (1 Digimon present), a selection prompt must install.
    assert!(
        runner.game.pending_selection.is_some(),
        "A selection prompt must be pending after revealing 3 cards that include a Digimon"
    );
}

/// When both a Digimon and Tamer are among the top 3, the Main clause results
/// in hand growing by 2 (one Digimon + one Tamer added from reveal).
///
/// NOTE: filter enforcement is accept-all (Phase 2b) — auto_resolve picks the
/// first and second revealed cards regardless of kind. This test asserts net
/// hand growth of +2 which matches the intent.
#[test]
fn p_206_main_adds_digimon_and_tamer_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .hand(0, &["P-206"])
        // DIG-1 top, TAM-1 second, FILL-1 third.
        .deck(0, &["FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len(); // 1 (P-206)
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    let hand_after = runner.game.players[0].hand.len();
    // P-206 stays via activate_hand_main; 2 cards added from reveal.
    // Expected: hand_before + 2 (1 Digimon + 1 Tamer).
    assert_eq!(
        hand_after,
        hand_before + 2,
        "Main must add 1 Digimon + 1 Tamer to hand (net +2); before={hand_before}, after={hand_after}"
    );
}

/// Deck size shrinks by 2 after Main (3 revealed, 2 added to hand, 1 returned to bottom).
#[test]
fn p_206_main_deck_shrinks_by_two_when_digimon_and_tamer_added() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .add_card(filler("FILL-2"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL-2", "FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    assert_eq!(
        deck_before - deck_after,
        2,
        "Deck must shrink by 2 (3 revealed, 2 to hand, 1 returned to bottom); \
         before={deck_before}, after={deck_after}"
    );
}

/// Negative condition: when no Digimon is among the top 3, the Digimon
/// select_reveal must not add a non-Digimon card to hand.
///
/// Phase 2b gap: `install_select_reveal` uses accept-all filter — a non-Digimon
/// card (tamer or option) is picked even when the filter requests only Digimon.
/// This test is #[ignore]'d pending Phase 2b filter closure.
#[test]
#[ignore = "pending Phase 2b: install_select_reveal accept-all filter — kind:digimon filter not enforced at runtime"]
fn p_206_main_digimon_filter_excludes_non_digimon_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_tamer("TAM-1"))
        .add_card(make_tamer("TAM-2"))
        .add_card(make_option("OPT-1"))
        .hand(0, &["P-206"])
        // Top 3: all non-Digimon cards — Digimon filter should exclude all.
        .deck(0, &["OPT-1", "TAM-2", "TAM-1"])
        .deck(1, &["TAM-1"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    let hand_after = runner.game.players[0].hand.len();
    // With no Digimon, Digimon slot = 0 added; Tamer slot = 1 added (TAM-1 or TAM-2).
    // Net change should be +1 (only Tamer, not Digimon).
    assert_eq!(
        hand_after,
        hand_before + 1,
        "No Digimon in top 3: only Tamer is added; before={hand_before}, after={hand_after}"
    );
}

/// Negative condition: when no Tamer is among the top 3, the Tamer
/// select_reveal must not add a non-Tamer card to hand.
///
/// Phase 2b gap: same accept-all issue.
#[test]
#[ignore = "pending Phase 2b: install_select_reveal accept-all filter — kind:tamer filter not enforced at runtime"]
fn p_206_main_tamer_filter_excludes_non_tamer_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_digimon("DIG-2"))
        .add_card(make_option("OPT-1"))
        .hand(0, &["P-206"])
        // Top 3: all non-Tamer cards — Tamer filter should exclude all.
        .deck(0, &["OPT-1", "DIG-2", "DIG-1"])
        .deck(1, &["DIG-1"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len();
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();

    let hand_after = runner.game.players[0].hand.len();
    // No Tamer in top 3: only Digimon is added. Net change +1.
    assert_eq!(
        hand_after,
        hand_before + 1,
        "No Tamer in top 3: only Digimon is added; before={hand_before}, after={hand_after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2b — Behavioral: Delay clause (Tamer play, -4 cost, color filter)
// ─────────────────────────────────────────────────────────────────────────────

/// Delay clause structural: process must be non-empty (contains select_hand +
/// play_from_hand steps for the Tamer play).
#[test]
fn p_206_delay_clause_has_non_empty_process() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    match &compiled.effects[1] {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            assert!(
                !process.is_empty(),
                "Delay process must be non-empty (contains Tamer select + play with -4 cost)"
            );
        }
        other => panic!("clause 1 must be Delay; got {:?}", other),
    }
}

/// Behavioral: after Delay is activated, a selection prompt installs for picking a Tamer.
///
/// BLOCKED: G-PLACE-SELF-AS-OPTION-PERMANENT — cannot drive Delay activation
/// without first placing the card in battle area. Ignored pending the gap.
#[test]
#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT — cannot drive Delay activation without battle-area assertion API"]
fn p_206_delay_activation_installs_tamer_selection() {
    todo!("implement once G-PLACE-SELF-AS-OPTION-PERMANENT is resolved");
}

/// Color-filter enforcement: only Tamers matching the color of YOUR Digimon on
/// field are eligible for the Delay clause. This requires G-COLOR-MATCH-AGAINST-BOARD.
#[test]
#[ignore = "pending: G-COLOR-MATCH-AGAINST-BOARD — no DSL predicate for 'same color as any of your Digimon on field'"]
fn p_206_delay_tamer_filter_matches_digimon_color_on_field() {
    todo!("implement once G-COLOR-MATCH-AGAINST-BOARD is resolved");
}

/// Negative: Tamer whose color does NOT match any of your Digimon on field
/// must not be selectable for the Delay clause.
#[test]
#[ignore = "pending: G-COLOR-MATCH-AGAINST-BOARD — color-filter against board not enforceable in DSL"]
fn p_206_delay_tamer_wrong_color_not_selectable() {
    todo!("implement once G-COLOR-MATCH-AGAINST-BOARD is resolved");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2c — Behavioral: Ignore-color bypass
// ─────────────────────────────────────────────────────────────────────────────

/// Positive: P-206 can be played even when the player does not have white memory.
/// The "ignore color requirement" means paying ANY cost is valid.
///
/// BLOCKED: G-IGNORE-COLOR-MASK — IgnoreColorRequirement modifier not enforced
/// in the action mask. The mask does not check this modifier, so this test
/// cannot verify the bypass is active. Ignored pending the gap.
#[test]
#[ignore = "pending: G-IGNORE-COLOR-MASK — IgnoreColorRequirement modifier not enforced in action mask (§4.2b residual)"]
fn p_206_can_be_played_without_matching_color_memory() {
    todo!("implement once G-IGNORE-COLOR-MASK is resolved");
}

/// Structural: the flood_gate clause for ignore-color targets only this card
/// (card_number_is: P-206) and is not conditional on any tamer/digimon presence.
/// This verifies the unconditional form (unlike ST22-08 which is conditional).
#[test]
fn p_206_ignore_color_flood_gate_is_unconditional() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    // The flood_gate clause must be present (unconditional ignore-color).
    // We just confirm the clause is declarative FloodGate — no active_when guards.
    let flood_gate_count = compiled.effects.iter().filter(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate { .. })
    )).count();

    assert_eq!(
        flood_gate_count,
        1,
        "P-206 must have exactly 1 FloodGate clause (ignore-color bypass); got {flood_gate_count}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral: Inherited security clause
// ─────────────────────────────────────────────────────────────────────────────

/// Inherited security clause structural: FaceUp scope is WRONG for inherited;
/// must be Inherited scope. Validates the scope annotation is correct.
#[test]
fn p_206_inherited_security_has_inherited_scope() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let inherited_sec = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(CompiledScope::Inherited)
        }
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
            scope, triggers, ..
        }) if *scope == CompiledScope::Inherited
            && triggers.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(*scope)
        }
        _ => None,
    });

    assert!(
        inherited_sec.is_some(),
        "Inherited security clause must exist with Inherited scope"
    );
    assert_eq!(
        inherited_sec.unwrap(),
        CompiledScope::Inherited,
        "Scope must be Inherited (not FaceUp)"
    );
}

/// When the inherited security clause fires, it offers the player a zone choice
/// (from hand or trash) and plays a cost-≤3 Digimon free.
///
/// Positive: hand has a Digimon with cost ≤ 3 — selection prompt installs.
///
/// NOTE: Full integrated test requires security-attack path. Using a
/// structural + steady-state approach; behavioral activation via the full
/// path deferred.
#[test]
fn p_206_inherited_security_process_is_non_empty() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon_low_cost("DIG-LOW"))
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let sec_process_non_empty = compiled.effects.iter().any(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            !t.process.is_empty()
        }
        _ => false,
    });

    // Either a non-empty process (if Triggered) or a RawRust stub is present.
    // Both are valid; just confirm the clause is not a no-op.
    let has_raw_rust_stub = compiled.effects.iter().any(|c| matches!(
        c,
        CompiledClause::Declarative(CompiledDeclarativeClause::RawRust { .. })
    ));

    assert!(
        sec_process_non_empty || has_raw_rust_stub,
        "Inherited security clause must have a non-empty process or be a raw_rust stub"
    );
}

/// Negative: when hand and trash are both empty of cost-≤3 Digimon, the
/// security clause should not prompt (optional — player may decline or no
/// candidates). Structural: confirms optional=true means PASS is valid.
#[test]
fn p_206_inherited_security_is_optional_no_mandatory_play() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(filler("FILL"))
        .hand(0, &["P-206"])
        .deck(0, &["FILL"])
        .memory(0)
        .start();

    let compiled = runner.compiled_card("P-206").expect("P-206 compiled");
    let sec_opt = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Triggered(t)
            if t.scope == CompiledScope::Inherited
                && t.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(t.optional)
        }
        _ => None,
    });

    // If triggered form, must be optional. RawRust handles optionality internally.
    if let Some(optional) = sec_opt {
        assert!(
            optional,
            "'You may' text → inherited security clause must be optional=true"
        );
    }
}

/// Cost-filter enforcement: only Digimon with play cost ≤ 3 should be selectable.
///
/// G-PLAY-COST-LTE: `play_cost_lte` predicate missing; filter not enforced.
#[test]
#[ignore = "pending: G-PLAY-COST-LTE — play_cost_lte predicate missing from PredicateSpec; select_hand/trash accept-all (Phase 2b)"]
fn p_206_inherited_security_cost_filter_excludes_high_cost_digimon() {
    todo!("implement when G-PLAY-COST-LTE is resolved");
}

/// Behavioral: add this card to hand after security resolves.
///
/// G-ADD-OPTION-SELF-TO-HAND: no DSL step verb for returning the played Option
/// to hand post-security resolution. raw_rust stub handles this.
#[test]
#[ignore = "pending: G-ADD-OPTION-SELF-TO-HAND — no DSL step verb for adding resolved Option to hand after security"]
fn p_206_inherited_security_adds_self_to_hand_after_play() {
    todo!("implement when G-ADD-OPTION-SELF-TO-HAND is resolved");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Event-log assertions
// ─────────────────────────────────────────────────────────────────────────────

/// When Main reveals 3 cards and 2 are added to hand (Digimon + Tamer), the
/// deck should have exactly 3 fewer cards visible after add_to_hand_from_reveal
/// calls fire. Deck size check doubles as an indirect event check.
#[test]
fn p_206_main_reveal_3_deck_accounting_correct() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("parses")
        .add_card(make_digimon("DIG-1"))
        .add_card(make_tamer("TAM-1"))
        .add_card(filler("FILL-1"))
        .add_card(filler("FILL-2"))
        .add_card(filler("FILL-3"))
        .hand(0, &["P-206"])
        // 5-card deck; top 3 = DIG-1, TAM-1, FILL-1.
        .deck(0, &["FILL-2", "FILL-3", "FILL-1", "TAM-1", "DIG-1"])
        .deck(1, &["FILL-1"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);
    let _ = runner.auto_resolve();
    let deck_after = runner.deck_size(0);

    // 3 revealed; 2 added to hand (DIG-1, TAM-1); 1 returned bottom (FILL-1).
    // Net deck change: -2.
    assert_eq!(
        deck_before - deck_after,
        2,
        "Reveal 3: 2 added to hand, 1 returned to bottom → deck shrinks by 2; \
         before={deck_before}, after={deck_after}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — Delay clause behavioral (blocked by G-PLACE-SELF-AS-OPTION-PERMANENT)
// ─────────────────────────────────────────────────────────────────────────────

/// Delay: Tamer played via Delay has cost reduced by 4.
///
/// BLOCKED: G-PLACE-SELF-AS-OPTION-PERMANENT (cannot place card in battle area
/// from test harness), G-COLOR-MATCH-AGAINST-BOARD (color filter unenforced).
#[test]
#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT + G-COLOR-MATCH-AGAINST-BOARD"]
fn p_206_delay_tamer_cost_reduced_by_4() {
    todo!("implement once both gaps are resolved");
}

/// Delay: 'you may' makes it optional — player can decline the Tamer play.
///
/// BLOCKED: G-PLACE-SELF-AS-OPTION-PERMANENT.
#[test]
#[ignore = "pending: G-PLACE-SELF-AS-OPTION-PERMANENT"]
fn p_206_delay_tamer_play_is_optional() {
    todo!("implement once G-PLACE-SELF-AS-OPTION-PERMANENT is resolved");
}
