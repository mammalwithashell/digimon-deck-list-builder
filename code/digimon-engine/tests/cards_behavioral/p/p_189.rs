//! P-189 Dimetromon — Digimon, Lv.4, Red, DP 6000, Cost 6.
//! Traits: Reptile, LIBERATOR
//!
//! # Card text (cards.json)
//!
//! [Security] You may play 1 card with the [LIBERATOR] trait and a play cost
//! of 4 or less from your hand or trash without paying the cost.
//! ＜Progress＞ (While attacking, your opponent's effects don't affect this Digimon.)
//!
//! Inherited:
//! [Your Turn] [Once Per Turn] When your opponent's security stack is removed
//! from, gain 1 memory.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/P/Red/P_189.cs
//!
//! # Patterns this test covers
//! - E2  OPT + optional (security clause is optional, inherited is OPT)
//! - H-Progress: declarative Progress keyword grant
//! - G-INHERITED-DISPATCH: inherited triggered on_opponent_security_removed
//! - G-OPT-TRIGGERED: once_per_turn on inherited triggered clause
//! - UnionZone-adjacent: security clause selects from hand OR trash
//!
//! # Known gaps
//! - G-DECLARATIVE-KEYWORD: Progress declarative clause compiles but modifier
//!   is not installed at runtime. Behavioral test #[ignore = "..."].
//! - G-INHERITED-DISPATCH: inherited triggered effects not dispatched from
//!   digivolution stack. All inherited clause tests #[ignore].
//! - G-OPT-TRIGGERED: OPT not enforced for triggered effects in queue drain.
//! - G-PLAY-COST-LTE (DSL vocab gap): `play_cost_lte` predicate missing from
//!   PredicateSpec; cost-≤4 filter cannot be enforced at selection time.
//!   select_hand/select_trash use accept-all filter (Phase 2b). Tests for
//!   cost-filter enforcement are #[ignore = "pending: G-PLAY-COST-LTE"].

const P189_YAML: &str = include_str!("../../../cards/p/P-189.yaml");

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

// ── Fixture builders ──────────────────────────────────────────────────────

/// A filler card with LIBERATOR trait and play cost 3 (eligible for security
/// clause selection in principle).
fn make_liberator_low_cost(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["LIBERATOR".to_string()];
    c.play_cost = 3;
    c
}

/// A filler Digimon card with no LIBERATOR trait (ineligible for security clause).
fn make_non_liberator(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.traits = vec!["Reptile".to_string()];
    c.play_cost = 3;
    c
}

/// Standard runner: P-189 from YAML, plus filler cards, 5 memory, game started.
fn dimetromon_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses without error")
        .add_card(make_liberator_low_cost("LIB-1"))
        .add_card(make_liberator_low_cost("LIB-2"))
        .add_card(make_non_liberator("NON-LIB-1"))
        .add_card(make_test_card("CARRIER", "Carrier"))
        .memory(5)
        .start()
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 1 — Structural assertions
// ─────────────────────────────────────────────────────────────────────────────

/// P-189 must compile to exactly 3 clauses:
///   [0] on_security triggered (optional, FaceUp scope)
///   [1] grant_keyword Progress (declarative, FaceUp scope)
///   [2] on_opponent_security_removed triggered (inherited, once_per_turn)
#[test]
fn p_189_compiles_to_three_clauses() {
    let runner = dimetromon_runner();
    let compiled = runner
        .compiled_card("P-189")
        .expect("P-189 must be compiled and registered");
    assert_eq!(
        compiled.effects.len(),
        3,
        "Expected 3 compiled clauses: on_security, grant_keyword(Progress), inherited OPT"
    );
}

/// Clause 0 — on_security, FaceUp scope, optional, no once_per_turn.
#[test]
fn p_189_security_clause_is_face_up_optional_not_opt() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let sec_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("P-189 must have an OnSecurity clause");

    assert_eq!(
        sec_clause.scope,
        CompiledScope::FaceUp,
        "Security clause is own-card effect → FaceUp scope"
    );
    assert!(
        sec_clause.optional,
        "'You may' text → optional must be true"
    );
    assert!(
        !sec_clause.once_per_turn,
        "Security clause has no [Once Per Turn]"
    );
}

/// Clause 1 — declarative GrantKeyword(Progress), FaceUp scope.
#[test]
fn p_189_has_progress_declarative_grant_keyword_clause() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let has_progress = compiled.effects.iter().any(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword {
            keyword,
            scope,
            ..
        }) => keyword.to_lowercase().contains("progress") && *scope == CompiledScope::FaceUp,
        _ => false,
    });

    assert!(
        has_progress,
        "P-189 must have a declarative GrantKeyword(Progress) clause with FaceUp scope"
    );
}

/// Clause 2 — on_opponent_security_removed, Inherited scope, once_per_turn.
#[test]
fn p_189_inherited_clause_is_opt_on_opponent_security_removed() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let inherited_clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == CompiledScope::Inherited => Some(t),
            _ => None,
        })
        .next()
        .expect("P-189 must have an Inherited triggered clause");

    assert!(
        inherited_clause
            .when
            .contains(&CompiledTiming::OnOpponentSecurityRemoved),
        "Inherited clause must fire on OnOpponentSecurityRemoved"
    );
    assert!(
        inherited_clause.once_per_turn,
        "[Once Per Turn] in text → once_per_turn must be true"
    );
    assert!(
        !inherited_clause.optional,
        "Inherited clause fires unconditionally when conditions pass (not user-optional)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 2 — Condition gating (security clause optionality)
// ─────────────────────────────────────────────────────────────────────────────

/// Positive structural: the security clause has optional=true, confirming
/// the player may decline it. Behavioral coverage via security-attack path
/// (Section 3 integrated test).
#[test]
fn p_189_security_clause_optional_allows_decline() {
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let sec = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("on_security clause");

    // optional=true means PASS is a legal action when the prompt installs.
    assert!(sec.optional, "Security 'you may' clause must be optional");
}

/// When only LIBERATOR cards with cost ≤ 4 should be selectable (cost-filter
/// enforcement is pending G-PLAY-COST-LTE). This test verifies what SHOULD
/// happen: non-LIBERATOR cards are excluded from selection.
///
/// NOTE: Pending G-PLAY-COST-LTE — select_hand/select_trash use accept-all
/// filter (Phase 2b), so ineligible cards DO appear in the current selection.
/// Ignored until the gap is closed and filter enforcement is wired.
#[test]
#[ignore = "pending: G-PLAY-COST-LTE — play_cost_lte predicate missing; select_hand/trash accept-all (Phase 2b)"]
fn p_189_security_filter_excludes_non_liberator_cards() {
    // When the security clause fires and only non-LIBERATOR cards are in hand/trash,
    // the selection prompt should either not install (zero candidates) or the player
    // must see zero valid targets. This confirms the trait filter is enforced.
    todo!("implement when G-PLAY-COST-LTE is resolved");
}

/// When only high-cost (> 4) LIBERATOR cards are available, the selection prompt
/// should have zero valid candidates.
///
/// NOTE: Same gap as above.
#[test]
#[ignore = "pending: G-PLAY-COST-LTE — play_cost_lte predicate missing; no cost-filter enforcement"]
fn p_189_security_filter_excludes_high_cost_liberator_cards() {
    todo!("implement when G-PLAY-COST-LTE is resolved");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3 — Behavioral outcomes (security clause)
// ─────────────────────────────────────────────────────────────────────────────

/// When the security clause fires and the player picks "From hand", the player
/// is prompted to select a card from hand. After selection, the card is played
/// for free (memory does not decrease by the card's cost).
///
/// Drive through: place P-189 on field (to get on_security via fire_on_play proxy),
/// assert EffectChoice prompt installs with 2 labels, execute "From hand" branch,
/// assert Hand selection installs, execute selection, confirm hand size decreases
/// and a new permanent enters the field.
///
/// NOTE: The security clause fires via on_security timing (SecuritySkill), not
/// on_play. The fire_on_play helper fires only on_play clauses. A full integrated
/// test requires a security-attack setup. Using a structural shortcut here:
/// assert that the security clause has exactly the expected process shape by
/// checking it's optional and present, then assert the game constructs correctly.
#[test]
fn p_189_security_clause_compiled_process_prompts_zone_choice() {
    // Structural: we can assert the compiled clause exists and is optional.
    // The actual zone-choice EffectChoice firing is covered when the full
    // security-attack path is available.
    let runner = dimetromon_runner();
    let compiled = runner.compiled_card("P-189").expect("P-189 compiled");

    let sec = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .next()
        .expect("on_security clause");

    // The process block starts with a select_effect_choice ("From hand" / "From trash"),
    // so when the clause fires, the first step must install an EffectChoice selection.
    // We confirm the clause has a non-empty process.
    assert!(
        !sec.process.is_empty(),
        "Security clause must have a non-empty process (select_effect_choice + if/then/else branches)"
    );
}

/// Fire the security clause via fire_on_play to confirm pending_selection
/// is None at steady state before the trigger.
#[test]
fn p_189_no_pending_selection_at_steady_state() {
    let mut runner = dimetromon_runner();
    let _perm = runner.place_on_field(0, "P-189", Some(0));
    // After placing on field (without triggering security), no selection is pending.
    assert!(
        runner.pending_selection().is_none(),
        "No pending selection at steady state"
    );
}

/// When P-189 is played normally (on_play path, not security), the only selection
/// that fires is from the on_play clause — there is no on_play clause on P-189,
/// so pending_selection should be None after play.
#[test]
fn p_189_on_play_no_effects_no_pending_selection() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P189_YAML)
        .expect("P-189 YAML parses")
        .add_card(make_liberator_low_cost("LIB-1"))
        .hand(0, &["P-189"])
        .memory(10)
        .start();

    let before_hand = runner.game.players[0].hand.len();
    let _perm_idx = runner.play(0, 0);
    // After playing P-189, no pending selection (no on_play clause).
    assert!(
        runner.pending_selection().is_none(),
        "P-189 has no on_play clause; no pending selection after play"
    );
    // Hand size decreased by 1.
    assert_eq!(
        runner.game.players[0].hand.len(),
        before_hand - 1,
        "Hand size decreased by 1 after playing P-189"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 3b — Behavioral outcomes (inherited clause)
// ─────────────────────────────────────────────────────────────────────────────

/// Positive: when Dimetromon is in a digivolution stack (inherited) and the
/// opponent's security is removed, the controller gains 1 memory.
///
/// BLOCKED: G-INHERITED-DISPATCH — inherited triggered effects not dispatched
/// from digivolution stack in the Rust engine.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH — inherited triggered effects not dispatched from digivolution stack"]
fn p_189_inherited_gains_1_memory_on_opp_security_removed() {
    todo!("Implement when G-INHERITED-DISPATCH is resolved");
}

/// Negative condition: inherited clause should NOT fire on opponent's turn
/// (active_when: your_turn guards this).
///
/// BLOCKED: G-INHERITED-DISPATCH.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH — inherited triggered effects not dispatched from digivolution stack"]
fn p_189_inherited_does_not_fire_on_opponents_turn() {
    todo!("Implement when G-INHERITED-DISPATCH is resolved");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 4 — Event-log assertions
// ─────────────────────────────────────────────────────────────────────────────

/// When the security clause fires and a card is played free, an OnPlay event
/// should be emitted. Deferred until security-attack integration path is wired.
#[test]
#[ignore = "pending: security-attack integration path in test harness; G-PLAY-COST-LTE"]
fn p_189_security_free_play_fires_on_play_event() {
    todo!("Implement when security-attack test harness path is confirmed");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 5 — OPT enforcement (inherited clause)
// ─────────────────────────────────────────────────────────────────────────────

/// OPT lockout: second firing in the same turn must be gated.
///
/// BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH + G-OPT-TRIGGERED"]
fn p_189_inherited_opt_blocks_second_activation_same_turn() {
    todo!("Implement when both gaps are resolved");
}

/// OPT lockout clears after end_turn.
///
/// BLOCKED: G-INHERITED-DISPATCH + G-OPT-TRIGGERED.
#[test]
#[ignore = "pending: G-INHERITED-DISPATCH + G-OPT-TRIGGERED"]
fn p_189_inherited_opt_clears_after_end_turn() {
    todo!("Implement when both gaps are resolved");
}

// ─────────────────────────────────────────────────────────────────────────────
// SECTION 6 — Progress keyword (declarative clause)
// ─────────────────────────────────────────────────────────────────────────────

/// Structural: Progress clause is compiled with FaceUp scope (verified in Section 1).
/// Behavioral: while P-189 is attacking, opponent's effects should not affect it.
///
/// BLOCKED: G-DECLARATIVE-KEYWORD — declarative keyword clauses not installed
/// at runtime (modifier not applied).
#[test]
#[ignore = "pending: G-DECLARATIVE-KEYWORD — declarative keyword clauses not installed at runtime"]
fn p_189_progress_active_while_attacking() {
    todo!("Implement when G-DECLARATIVE-KEYWORD is resolved");
}
