//! EX10-034 Blastmon — Digimon, Black/Purple, Level 6, Cost 13, DP 13000.
//! Traits: [Mineral, Bagra Army]
//!
//! # Card text (EX10-034.json / data/cards.json — verbatim)
//!
//! ```text
//! <Collision> <Fragment (3)> <Blocker>
//! [On Play] [When Digivolving] Until your opponent's turn ends, give 1 of
//!   their Digimon "[Start of Your Main Phase] This Digimon attacks."
//! [All Turns] [Once Per Turn] When Digimon attack, by trashing any 2 of this
//!   Digimon's digivolution cards, this Digimon gains <Security A. +1> and
//!   +3000 DP until your turn ends.
//! ```
//!
//! # Patterns this test covers
//!
//! | Clause | Pattern | Status |
//! |--------|---------|--------|
//! | Keywords Collision/Fragment(3)/Blocker | declarative grant_keyword | PASS |
//! | Clause A [OP][WD] grant forced-attack to 1 opp Digimon | grant_triggered_effect target binding | PASS |
//! | Clause B [All Turns][OPT] when_attacking + on_opponent_attack + all_turns | observer multi-timing | PASS |
//! | Clause B once_per_turn + optional structural flags | compiled struct | PASS |
//! | Clause B positive: SourceMulti(2,2) prompt with ≥2 own sources | select_own_sources from:source | PASS |
//! | Clause B negative: no prompt when <2 own sources | select_own_sources silent-skip | PASS |
//! | Clause B: SecurityAttackChange +1 granted after cost | add_modifier SecurityAttackChange | PASS |
//! | Clause B: +3000 DP granted after cost | add_dp_modifier end_of_turn | PASS |
//! | Clause B: 2 sources removed from stack after cost | trash_selected_sources | PASS |
//! | Clause B OPT lockout: second when_attacking same turn no prompt | once_per_turn | PASS |

use digimon_dsl::compiled::{
    CompiledBindingRef, CompiledClause, CompiledModifierTarget, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::{encode_attack, SECURITY_TARGET};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword, ModifierType};
use digimon_engine::selection::{SelectionKind, TriggerSource};

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn blastmon_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .build()
}

/// Build a plain source card (level 3, no special traits).
fn make_source(id: &str) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, &format!("{id} Source"));
    c.level = Some(3);
    c
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 1 — Keywords (original test preserved)
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn ex10_034_on_field_has_collision_fragment_and_blocker() {
    let mut runner = blastmon_runner();
    let handle = runner.place_on_field(0, "EX10-034", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Collision));
    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
    assert!(runner.game.has_keyword(handle, Keyword::Blocker));
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 2 — Clause A: structural assertions
// ═══════════════════════════════════════════════════════════════════════════════
//
// Clause A grants "1 of opponent's Digimon [Start of Your Main Phase] attack"
// until opponent's turn ends. It selects exactly 1 opponent Digimon, then
// grants the triggered body to that selected binding only.

/// Clause A structural: EX10-034 must have a triggered clause with
/// `on_play` or `when_digivolving` timing containing a
/// `grant_triggered_effect` step.
///
#[test]
fn ex10_034_clause_a_has_on_play_when_digivolving_grant_triggered_effect() {
    let runner = blastmon_runner();
    let card = runner
        .compiled_card("EX10-034")
        .expect("EX10-034 in embedded pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| {
            t.when.contains(&CompiledTiming::OnPlay)
                && t.when.contains(&CompiledTiming::WhenDigivolving)
        })
        .expect("EX10-034 must have a shared on_play/when_digivolving Clause A");

    let grant = clause
        .process
        .iter()
        .find_map(|step| match step {
            CompiledStep::GrantTriggeredEffect {
                target,
                timing,
                expiry,
                body,
            } => Some((target, timing, expiry, body)),
            _ => None,
        })
        .expect("Clause A must contain grant_triggered_effect");

    assert!(
        matches!(
            grant.0,
            CompiledModifierTarget::Binding(CompiledBindingRef::Named(name)) if name == "forced_target"
        ),
        "Clause A must grant to the selected binding, not broadcast to a predicate"
    );
    assert_eq!(grant.1, "start_of_your_main_phase");
    assert_eq!(grant.2, "end_of_opponents_turn");
    assert!(
        grant
            .3
            .iter()
            .any(|step| matches!(step, CompiledStep::ForceAttack { .. })),
        "granted body must force the carrier to attack"
    );
}

/// Clause A behavioral: [OP][WD] installs a selection to choose 1 opp Digimon
/// when fired on on_play timing.
///
#[test]
fn ex10_034_clause_a_on_play_prompts_opp_digimon_selection() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(make_test_card("OPP", "Opponent Digi"))
        .hand(0, &["EX10-034"])
        .memory(10)
        .start();

    runner.place_on_field(1, "OPP", None);
    runner.play(0, 0).expect("play Blastmon");

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "Clause A [OP] must prompt for 1 opponent Digimon selection"
    );
}

/// Clause A behavioral: [OP][WD] does nothing when opponent has no Digimon.
///
#[test]
fn ex10_034_clause_a_no_selection_when_no_opp_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .hand(0, &["EX10-034"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Blastmon");

    assert!(
        runner.pending_selection().is_none(),
        "Clause A must not install a selection when no opponent Digimon exist"
    );
}

/// Clause A grants the forced Start of Main attack only to the selected
/// opponent Digimon. A second opponent Digimon must not receive the grant.
#[test]
fn ex10_034_clause_a_selected_digimon_attacks_at_start_of_main_phase() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(make_test_card("OPP-A", "Opponent A"))
        .add_card(make_test_card("OPP-B", "Opponent B"))
        .add_card(make_test_card("FILL-034", "Filler"))
        .hand(0, &["EX10-034"])
        .deck(0, &["FILL-034", "FILL-034", "FILL-034"])
        .deck(1, &["FILL-034", "FILL-034", "FILL-034"])
        .memory(20)
        .start();

    let opp_a = runner.place_on_field(1, "OPP-A", None);
    let opp_b = runner.place_on_field(1, "OPP-B", None);
    runner.play(0, 0).expect("play Blastmon");

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Clause A should ask P0 to choose 1 opponent Digimon");
    assert_eq!(pending.selecting_player, 0);
    let select_opp_a = encode_attack(0, opp_a.index as u16);
    assert!(pending.valid_action_ids.contains(&select_opp_a));
    runner
        .game
        .resolve_selection(0, select_opp_a)
        .expect("select first opponent Digimon");

    assert_eq!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_a, EffectTiming::StartOfYourMainPhase)
            .len(),
        1,
        "selected opponent Digimon should carry the granted Start of Main attack"
    );
    assert!(
        runner
            .game
            .modifiers
            .granted_triggered_for_timing(opp_b, EffectTiming::StartOfYourMainPhase)
            .is_empty(),
        "unselected opponent Digimon must not receive the granted attack"
    );

    runner.end_turn(); // P0 -> P1; fires StartOfYourMainPhase for P1.
    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("granted forced attack should ask the carrier controller for a target");
    assert_eq!(pending.kind, SelectionKind::Target);
    assert_eq!(pending.selecting_player, 1);
    assert!(
        !pending.is_optional,
        "printed granted attack is mandatory once a legal attack exists"
    );
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(opp_a.index as u16, SECURITY_TARGET)));
    assert!(
        !pending
            .valid_action_ids
            .contains(&encode_attack(opp_b.index as u16, SECURITY_TARGET)),
        "the unselected Digimon must not be the forced attacker"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: structural assertions
// ═══════════════════════════════════════════════════════════════════════════════

/// Clause B must have a triggered clause with `when_attacking` timing
/// (plus `on_opponent_attack` for the all-turns form), `once_per_turn: true`,
/// and `optional: true`.
#[test]
fn ex10_034_clause_b_has_when_attacking_once_per_turn_optional() {
    let runner = blastmon_runner();
    let card = runner
        .compiled_card("EX10-034")
        .expect("EX10-034 in embedded pack");

    let clause = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .find(|t| t.when.contains(&CompiledTiming::WhenAttacking))
        .expect("EX10-034 must have a triggered clause with WhenAttacking timing (Clause B)");

    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Clause B is a face-up (non-inherited) effect"
    );
    assert!(
        clause.once_per_turn,
        "[Once Per Turn] is printed — Clause B must set once_per_turn: true"
    );
    assert!(
        clause.optional,
        "'by trashing' is a cost-gated optional activation — optional: true required"
    );
    assert!(
        clause.when.contains(&CompiledTiming::OnOpponentAttack),
        "[All Turns] requires on_opponent_attack timing alongside when_attacking"
    );
    assert!(
        clause
            .active_when
            .as_ref()
            .map(|p| p.all_turns == Some(true))
            .unwrap_or(false),
        "[All Turns] requires active_when: {{ all_turns: true }} in Clause B"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 4 — Clause B: positive behavioral (your turn, ≥2 own sources)
// ═══════════════════════════════════════════════════════════════════════════════
//
// With `optional: true` on the clause, the drainer first installs an outer
// Replacement selection ("you may activate"). After the player accepts, the
// SourceMulti(2,2) selection follows.

/// Positive: when EX10-034 has ≥2 sources and when_attacking fires,
/// an outer Replacement (accept/decline) is installed first, then after
/// accepting a SourceMulti(2,2) selection appears.
#[test]
fn ex10_034_clause_b_when_attacking_prompts_source_multi_2_of_2() {
    let src_a = make_source("SRC-B1");
    let src_b = make_source("SRC-B2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    runner.push_source(blastmon, "SRC-B1");
    runner.push_source(blastmon, "SRC-B2");

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    // Outer Replacement (optional accept/decline) installed first.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "Clause B must first install an outer Replacement (accept/decline) selection; got {:?}",
        runner.pending_kind()
    );
    // Accept the outer optional using accept_optional_trigger (single-step, does NOT drain further).
    runner
        .accept_optional_trigger()
        .expect("accept outer optional");

    // SourceMulti(2,2) must appear now.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 2, max: 2, .. })
        ),
        "after accepting the outer optional, Clause B must prompt SourceMulti(2,2); got {:?}",
        runner.pending_kind()
    );
}

/// Positive (opponent's turn / on_opponent_attack path): outer Replacement then
/// SourceMulti(2,2) also fires when EX10-034 observes the opponent's Digimon attacking.
#[test]
fn ex10_034_clause_b_on_opponent_attack_prompts_source_multi_2_of_2() {
    let src_a = make_source("OA-SRC1");
    let src_b = make_source("OA-SRC2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    runner.push_source(blastmon, "OA-SRC1");
    runner.push_source(blastmon, "OA-SRC2");

    runner.game.enqueue_triggered(
        EffectTiming::OnOpponentAttack,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    // Outer Replacement installed.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "Clause B on on_opponent_attack must first install Replacement; got {:?}",
        runner.pending_kind()
    );
    // Accept (single-step — does NOT drain the SourceMulti that follows).
    runner
        .accept_optional_trigger()
        .expect("accept outer optional");

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 2, max: 2, .. })
        ),
        "after accepting, Clause B on on_opponent_attack must prompt SourceMulti(2,2); got {:?}",
        runner.pending_kind()
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 5 — Clause B: negative (fewer than 2 own sources)
// ═══════════════════════════════════════════════════════════════════════════════
//
// With `optional: true` and `SelectOwnSources { min: 2 }`, the outer
// Replacement prompt installs unconditionally (no guard exists for
// SelectOwnSources in first_step_candidate_guard). After the player accepts,
// the SourceMulti silently skips because <2 candidates exist. No buffs granted.

/// Negative: with 0 sources, the outer Replacement is still installed
/// (no guard suppresses it), but after accepting, no SourceMulti appears —
/// the body silently skips because min: 2 cannot be satisfied.
#[test]
fn ex10_034_clause_b_no_source_multi_when_zero_own_sources() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    // No sources pushed — stack has only the top card (EX10-034).

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    // Outer Replacement is installed (no guard suppresses it for SelectOwnSources).
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "outer Replacement should still be installed (no guard for SelectOwnSources)"
    );
    // Accept — body runs but silently skips (no candidates for min: 2).
    runner.auto_resolve().expect("accept outer optional");
    runner.auto_resolve().ok(); // drain any follow-up

    // No SourceMulti should appear — body skipped silently.
    assert!(
        !matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { .. })
        ),
        "Clause B must not install SourceMulti when EX10-034 has 0 digivolution sources; got {:?}",
        runner.pending_kind()
    );
    // No SecurityAttackChange granted.
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(blastmon, ModifierType::SecurityAttackChange),
        0,
        "no SecurityAttackChange must be granted when cost cannot be paid"
    );
}

/// Negative: with exactly 1 source, outer Replacement is installed but after
/// accepting, no SourceMulti appears (only 1 candidate, min: 2).
#[test]
fn ex10_034_clause_b_no_source_multi_when_only_one_source() {
    let src = make_source("ONE-SRC");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(src)
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    runner.push_source(blastmon, "ONE-SRC");

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "outer Replacement should still be installed with 1 source (no guard)"
    );
    runner.auto_resolve().expect("accept outer optional");
    runner.auto_resolve().ok();

    assert!(
        !matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { .. })
        ),
        "Clause B must not install SourceMulti when EX10-034 has only 1 digivolution source; got {:?}",
        runner.pending_kind()
    );
    assert_eq!(
        runner
            .game
            .modifiers
            .sum(blastmon, ModifierType::SecurityAttackChange),
        0,
        "no SecurityAttackChange must be granted when cost cannot be paid (1 source < 2 needed)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 6 — Clause B: SecurityAttackChange +1 after cost
// ═══════════════════════════════════════════════════════════════════════════════

/// After paying the 2-source cost, EX10-034 gains SecurityAttackChange +1.
#[test]
fn ex10_034_clause_b_grants_security_attack_plus_1_after_cost() {
    let src_a = make_source("SA-SRC1");
    let src_b = make_source("SA-SRC2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    runner.push_source(blastmon, "SA-SRC1");
    runner.push_source(blastmon, "SA-SRC2");

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    // Step 1: outer Replacement (accept/decline) installed.
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "outer Replacement selection must be pending first"
    );
    // Accept the outer optional (single-step — does NOT drain the SourceMulti that follows).
    runner
        .accept_optional_trigger()
        .expect("accept outer optional");

    // Step 2: SourceMulti(2,2) now pending.
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 2, max: 2, .. })
        ),
        "SourceMulti(2,2) must follow the outer accept"
    );
    // Pay the cost: pick both sources.
    runner.auto_resolve().expect("source selection resolves");
    runner.auto_resolve().ok(); // drain any follow-up

    assert_eq!(
        runner
            .game
            .modifiers
            .sum(blastmon, ModifierType::SecurityAttackChange),
        1,
        "EX10-034 must gain SecurityAttackChange +1 after paying the 2-source cost"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 7 — Clause B: +3000 DP after cost
// ═══════════════════════════════════════════════════════════════════════════════

/// After paying the 2-source cost, EX10-034 gains +3000 DP.
#[test]
fn ex10_034_clause_b_grants_plus_3000_dp_after_cost() {
    let src_a = make_source("DP-SRC1");
    let src_b = make_source("DP-SRC2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    runner.push_source(blastmon, "DP-SRC1");
    runner.push_source(blastmon, "DP-SRC2");

    let dp_before = runner.dp_of(blastmon).unwrap_or(0);

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    // Accept outer optional.
    runner.auto_resolve().expect("accept outer optional");
    // Pick both sources.
    runner.auto_resolve().expect("source selection resolves");
    runner.auto_resolve().ok();

    let dp_after = runner.dp_of(blastmon).unwrap_or(0);
    assert_eq!(
        dp_after - dp_before,
        3000,
        "EX10-034 must gain +3000 DP after paying the 2-source cost (before: {dp_before}, after: {dp_after})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 8 — Clause B: 2 sources are trashed from the stack
// ═══════════════════════════════════════════════════════════════════════════════

/// After paying the cost, EX10-034's digivolution stack shrinks by exactly 2
/// (both selected sources are moved to trash).
#[test]
fn ex10_034_clause_b_removes_2_sources_from_stack() {
    let src_a = make_source("TR-SRC1");
    let src_b = make_source("TR-SRC2");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    runner.push_source(blastmon, "TR-SRC1");
    runner.push_source(blastmon, "TR-SRC2");

    // Stack is: EX10-034 (top) + TR-SRC1 + TR-SRC2 = 3 cards.
    let stack_before = runner.game.players[0].battle_area[blastmon.index as usize]
        .card_sources
        .len();
    // stack_before should be 2 (2 sources below top card EX10-034).
    // The top card itself is not counted in card_sources (it's the permanent's card_idx).

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    // Accept outer optional.
    runner.auto_resolve().expect("accept outer optional");
    // Pick both sources.
    runner.auto_resolve().expect("source selection resolves");
    runner.auto_resolve().ok();

    let stack_after = runner.game.players[0].battle_area[blastmon.index as usize]
        .card_sources
        .len();

    assert_eq!(
        stack_before - stack_after,
        2,
        "Clause B cost must trash exactly 2 digivolution sources from EX10-034's stack (before: {stack_before}, after: {stack_after})"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 9 — Clause B: OPT lockout (second attack same turn)
// ═══════════════════════════════════════════════════════════════════════════════

/// OPT: a second when_attacking fire in the same turn must NOT install another
/// SourceMulti selection (once_per_turn lockout).
#[test]
fn ex10_034_clause_b_opt_blocks_second_when_attacking_same_turn() {
    let src_a = make_source("OPT-SRC1");
    let src_b = make_source("OPT-SRC2");
    let src_c = make_source("OPT-SRC3");
    let src_d = make_source("OPT-SRC4");

    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .add_card(src_a)
        .add_card(src_b)
        .add_card(src_c)
        .add_card(src_d)
        .memory(10)
        .start();

    let blastmon = runner.place_on_field(0, "EX10-034", None);
    runner.push_source(blastmon, "OPT-SRC1");
    runner.push_source(blastmon, "OPT-SRC2");
    runner.push_source(blastmon, "OPT-SRC3");
    runner.push_source(blastmon, "OPT-SRC4");

    // First activation: outer Replacement installs, then SourceMulti after accept.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::Replacement),
        "First activation must install outer Replacement selection"
    );
    // Accept outer optional (single-step — does NOT drain the SourceMulti that follows).
    runner
        .accept_optional_trigger()
        .expect("accept outer optional");
    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::SourceMulti { min: 2, max: 2, .. })
        ),
        "After accepting, SourceMulti(2,2) must appear"
    );
    runner
        .auto_resolve()
        .expect("first activation source selection resolves");
    runner.auto_resolve().ok();

    // Second activation same turn: OPT flag blocks it — nothing installed.
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(blastmon),
    );
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection().is_none(),
        "OPT must block Clause B from installing a second selection in the same turn"
    );
}
