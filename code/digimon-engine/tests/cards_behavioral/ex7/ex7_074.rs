//! EX7-074 Vortex Resonance — Option card, Cost 3, Multi-color (Green + Blue).
//!
//! # Card text (cards.json)
//!
//! While you have [LIBERATOR] trait Digimon or Tamer, you can ignore this card's color
//! requirements.
//!
//! [Main] Reveal the top 3 cards of your deck. Add 1 card with the [LIBERATOR] trait among
//! them to the hand. Return the rest to the bottom of the deck.
//! Then, 1 of your Digimon may digivolve into a Digimon card in your hand with the
//! digivolution cost reduced by 4.
//!
//! Inherited: Security Effect [Security] You may play 1 card with the [LIBERATOR] trait with a
//! play cost of 4 or less from your hand or trash without paying the cost. Then, add this
//! card to the hand.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/EX7/Green/EX7_074.cs
//!
//! # Patterns this test covers
//! - A1 Searching (top-3 reveal, add by LIBERATOR trait)
//! - D3 Color ignore / bypass [G-IGNORE-COLOR-MASK: runtime enforcement absent]
//! - Effect-initiated digivolve with cost reduce 4 (main_from_hand option)
//! - Security: hand-or-trash play (LIBERATOR, cost ≤ 4) + raw_rust self-to-hand
//!   [G-ADD-OPTION-SELF-TO-HAND: runtime "add to hand" is a no-op placeholder]
//!   [G-PLAY-COST-LTE: cost ≤ 4 predicate not enforced at selection time]
//!
//! # Known gaps inherited from this card
//! - G-IGNORE-COLOR-MASK — flood_gate IgnoreColorRequirement compiles but has no runtime
//!   effect in the Rust action mask (option_color_match_available in mask.rs does not
//!   consult the IgnoreColorRequirement modifier).
//! - G-ADD-OPTION-SELF-TO-HAND — no DSL step / engine method for returning the currently-
//!   resolving security option card to the controller's hand. raw_rust placeholder lm_027_add_self_to_hand
//!   is reused (EX7-074 uses the same no-op as LM-027).
//! - G-PLAY-COST-LTE — `play_cost_lte: 4` predicate is not evaluated in eval_card_fields at
//!   selection time; all LIBERATOR cards in hand/trash appear as candidates.

#![allow(unused_imports, dead_code, unused_mut, unused_variables)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

// Load the YAML at compile time so tests run against the exact shipped spec.
const EX7_074_YAML: &str = include_str!("../../../cards/ex7/EX7-074.yaml");

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn make_liberator_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.colors = vec![CardColor::Red];
    c
}

fn ex7_074_runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .memory(10)
        .start()
}

// ---------------------------------------------------------------------------
// Section 1: Structural assertions
// ---------------------------------------------------------------------------

/// YAML must parse and compile without error.
#[test]
fn ex7_074_yaml_parses_without_error() {
    let _runner = ex7_074_runner();
}

/// EX7-074 must have exactly three clauses:
///   0: flood_gate declarative (IgnoreColorRequirement, conditional on LIBERATOR Digimon/Tamer)
///   1: main_from_hand triggered (reveal-3 + add-LIBERATOR + digivolve cost -4)
///   2: on_security inherited triggered (optional play LIBERATOR ≤4 from hand or trash + raw_rust)
#[test]
fn ex7_074_has_three_clauses() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    assert_eq!(
        compiled.effects.len(),
        3,
        "EX7-074 must have exactly 3 clauses (flood_gate + main + security); got {}",
        compiled.effects.len()
    );
}

/// Clause 0 is the flood_gate declarative for color bypass.
/// DCGO: IgnoreColorConditionClass at EffectTiming.None (non-triggered).
/// G-IGNORE-COLOR-MASK: compiles correctly but has no runtime effect in the Rust action mask.
#[test]
fn ex7_074_clause_0_is_declarative_flood_gate() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    assert!(
        matches!(compiled.effects[0], CompiledClause::Declarative(_)),
        "Clause 0 must be a declarative (flood_gate for IgnoreColorRequirement); got {:?}",
        compiled.effects[0]
    );
}

/// Clause 1 is the Main triggered clause with main_from_hand timing.
/// DCGO: EffectTiming.OptionSkill — this is the standard main option timing.
#[test]
fn ex7_074_clause_1_is_main_from_hand_triggered() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    let main_clause = compiled.effects.get(1).expect("clause 1 must exist");
    assert!(
        matches!(main_clause, CompiledClause::Triggered(_)),
        "Clause 1 must be a triggered clause; got {:?}",
        main_clause
    );
    if let CompiledClause::Triggered(t) = main_clause {
        assert!(
            t.when.contains(&CompiledTiming::MainFromHand),
            "Clause 1 must have MainFromHand timing; got {:?}",
            t.when
        );
        assert_eq!(
            t.scope,
            CompiledScope::FaceUp,
            "Clause 1 must have FaceUp scope"
        );
        assert!(
            !t.once_per_turn,
            "Main clause has no [Once Per Turn] in printed text"
        );
        // Main clause is NOT optional at the clause level — the printed text has no "you may"
        // on the reveal step. The digivolve sub-step is optional via select_own_permanent
        // with optional: true, but the clause itself is mandatory.
        // DCGO: ActivateClass.SetUpActivateClass does not set canNoSelect at the
        // clause-wrapper level; it's the inner SelectPermanentEffect that has canNoSelect: true.
        assert!(
            !t.optional,
            "Main clause must NOT be optional at clause level (printed text has no outer 'you may')"
        );
    }
}

/// Clause 2 is the inherited Security triggered clause (on_security timing).
/// "You may" → optional: true.
/// DCGO: EffectTiming.SecuritySkill, activateClass.SetIsSecurityEffect(true).
/// Scope: Inherited (this is a security effect that is inherited from EX7-074
/// when it is in someone's digivolution stack).
#[test]
fn ex7_074_clause_2_is_inherited_security_optional() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    let sec_clause = compiled.effects.get(2).expect("clause 2 must exist");
    assert!(
        matches!(sec_clause, CompiledClause::Triggered(_)),
        "Clause 2 must be a triggered clause; got {:?}",
        sec_clause
    );
    if let CompiledClause::Triggered(t) = sec_clause {
        assert!(
            t.when.contains(&CompiledTiming::OnSecurity),
            "Clause 2 must have OnSecurity timing; got {:?}",
            t.when
        );
        assert_eq!(
            t.scope,
            CompiledScope::Inherited,
            "Clause 2 must have Inherited scope (security effect)"
        );
        assert!(
            t.optional,
            "Clause 2 must be optional ('you may' in printed text)"
        );
        assert!(
            !t.once_per_turn,
            "Security clause has no [Once Per Turn] in printed text"
        );
    }
}

// ---------------------------------------------------------------------------
// Section 2: Color bypass structural (G-IGNORE-COLOR-MASK)
// ---------------------------------------------------------------------------

/// The IgnoreColorRequirement flood_gate clause compiles correctly.
/// This test verifies compilation — runtime enforcement is absent (G-IGNORE-COLOR-MASK).
#[test]
fn ex7_074_color_bypass_compiles_as_declarative() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    // Clause 0 should be the declarative flood_gate.
    assert!(
        matches!(compiled.effects[0], CompiledClause::Declarative(_)),
        "Flood gate clause (clause 0) must compile as Declarative (got {:?})",
        compiled.effects[0]
    );
}

/// Runtime enforcement is absent (G-IGNORE-COLOR-MASK). This test documents the
/// known gap — color bypass is a no-op at runtime in the Rust engine action mask.
/// When G-IGNORE-COLOR-MASK is resolved, this test can be un-ignored.
#[test]
#[ignore = "pending: G-IGNORE-COLOR-MASK — IgnoreColorRequirement not enforced in Rust option action mask"]
fn ex7_074_color_bypass_unlocks_play_when_liberator_on_field() {
    // Setup: player 0 has a LIBERATOR Digimon on field.
    // EX7-074 is green+blue; if player 0 has only colors not matching, normally unplayable.
    // With LIBERATOR on field + IgnoreColorRequirement modifier, should become playable.
    let _runner = ex7_074_runner();
}

// ---------------------------------------------------------------------------
// Section 3: Main clause — reveal + add to hand + digivolve sub-step
// ---------------------------------------------------------------------------

/// When played as a Main option, the reveal-pick prompt installs (SelectReveal kind).
/// After auto-resolving the reveal, the digivolve select_own_permanent prompt installs.
/// This test verifies the full Main clause flow.
#[test]
#[ignore = "pending: main_from_hand option play integration not yet wired in DebugRunner"]
fn ex7_074_main_reveals_three_and_installs_pick_prompt() {
    // Setup: player 0 hand has EX7-074; deck has 3+ fillers.
    // Play the option → reveal prompt installs.
    let _runner = ex7_074_runner();
}

/// After reveal + selecting 1 LIBERATOR card, the selected card moves to hand.
/// DCGO: mode: SelectCardEffect.Mode.AddHand → selected card added to controller's hand.
#[test]
#[ignore = "pending: main_from_hand option play integration not yet wired in DebugRunner"]
fn ex7_074_main_adds_liberator_from_reveal_to_hand() {
    // Arrange: place a known LIBERATOR card at the top of the deck.
    // Play EX7-074 → reveal 3 → select the LIBERATOR card → add to hand.
    // Assert: LIBERATOR card is now in hand; deck has 2 fewer revealed cards.
    let _runner = ex7_074_runner();
}

/// After reveal, the non-selected cards return to the BOTTOM of the deck.
/// DCGO: remainingCardsPlace: RemainingCardsPlace.DeckBottom.
/// Engine: deck stored front=bottom, back=top → returned cards go to deck[0..].
#[test]
#[ignore = "pending: main_from_hand option play integration not yet wired in DebugRunner"]
fn ex7_074_main_reveal_remainder_placed_at_deck_bottom() {
    // After reveal + selection, the 2 non-selected cards are at the bottom
    // of the deck (lower indices in the engine's Vec<CardSource>).
    let _runner = ex7_074_runner();
}

/// The digivolve sub-step: select_own_permanent is optional (canNoSelect: true).
/// DCGO: SelectPermanentEffect with canNoSelect: true at the digivolve sub-step.
/// If player declines (no permanent selected), no digivolve occurs.
#[test]
#[ignore = "pending: main_from_hand option play integration not yet wired in DebugRunner"]
fn ex7_074_digivolve_substep_is_optional_declinable() {
    // After reveal+add step completes, the digivolve prompt installs.
    // Assert: runner.pending_is_optional() == true.
    // If player passes: no digivolve occurs; effect resolves cleanly.
    let _runner = ex7_074_runner();
}

/// The digivolve sub-step reduces cost by 4.
/// DCGO: reduceCostTuple: (reduceCost: 4, reduceCostCardCondition: null).
/// The selected Digimon pays max(0, original_digivolve_cost - 4) memory.
#[test]
#[ignore = "pending: main_from_hand option play integration not yet wired in DebugRunner"]
fn ex7_074_digivolve_substep_cost_reduced_by_4() {
    // Setup: player 0 has a Digimon on field; hand has a Digimon card.
    // Trigger digivolve sub-step → select Digimon on field → select hand card.
    // Assert: memory spent = max(0, normal_cost - 4).
    let _runner = ex7_074_runner();
}

/// The digivolve sub-step: the target card in hand must be a Digimon.
/// DCGO: CanSelectHandCardCondition checks `cardSource.IsDigimon`.
#[test]
#[ignore = "pending: main_from_hand option play integration not yet wired in DebugRunner"]
fn ex7_074_digivolve_substep_only_offers_digimon_hand_cards() {
    // Arrange: hand has both a Digimon and a Tamer.
    // After reveal+add step, the select_hand prompt must only show the Digimon.
    let _runner = ex7_074_runner();
}

/// If no own Digimon can digivolve (no Digimon on field OR no valid Digimon in hand),
/// the digivolve sub-step is skipped entirely.
/// DCGO: CanSelectOwnPermanentCondition checks battle area + hand eligibility; if
/// HasMatchConditionPermanent returns false, the SelectPermanentEffect block is skipped.
#[test]
#[ignore = "pending: main_from_hand option play integration not yet wired in DebugRunner"]
fn ex7_074_digivolve_substep_skipped_when_no_eligible_digimon() {
    let _runner = ex7_074_runner();
}

// ---------------------------------------------------------------------------
// Section 4: Security clause — optional play from hand or trash
// ---------------------------------------------------------------------------

/// Positive case (hand only): LIBERATOR card (cost ≤ 4) in hand → security
/// clause installs a direct hand selection prompt (no zone choice needed).
/// DCGO: if canSelectHand && !canSelectTrash, auto-resolves from=hand.
#[test]
#[ignore = "pending: security effect integration not yet wired in DebugRunner"]
fn ex7_074_security_installs_hand_selection_when_only_hand_eligible() {
    let _runner = ex7_074_runner();
}

/// Positive case (both zones): LIBERATOR card in both hand and trash →
/// zone-choice prompt ("From hand" / "From trash") installs first.
/// DCGO: "From which area do you play a card?" EffectChoice prompt.
#[test]
#[ignore = "pending: security effect integration not yet wired in DebugRunner"]
fn ex7_074_security_installs_zone_choice_when_liberator_in_both_zones() {
    let _runner = ex7_074_runner();
}

/// Negative case: no LIBERATOR cards (cost ≤ 4) in hand or trash →
/// security clause resolves as a no-op. The raw_rust add-self-to-hand step still fires
/// (no-op under G-ADD-OPTION-SELF-TO-HAND).
/// DCGO: if !canSelectHand && !canSelectTrash, skips the if (canSelectHand || canSelectTrash) block,
/// then proceeds to AddThisCardToHand.
#[test]
#[ignore = "pending: security effect integration not yet wired in DebugRunner"]
fn ex7_074_security_no_op_when_no_liberator_available() {
    let _runner = ex7_074_runner();
}

/// The security clause plays the selected LIBERATOR card without paying cost.
/// DCGO: PlayPermanentCards(payCost: false, ...).
/// Assert: selected card moves from hand/trash to battle area; memory unchanged.
#[test]
#[ignore = "pending: security effect integration not yet wired in DebugRunner"]
fn ex7_074_security_plays_liberator_from_hand_free() {
    let _runner = ex7_074_runner();
}

/// The security clause plays the selected LIBERATOR card from trash without paying cost.
/// DCGO: fromHand=false → SelectCardEffect with Root.Trash → PlayPermanentCards.
#[test]
#[ignore = "pending: security effect integration not yet wired in DebugRunner"]
fn ex7_074_security_plays_liberator_from_trash_free() {
    let _runner = ex7_074_runner();
}

/// Cost ≤ 4 filter: LIBERATOR cards with play cost > 4 must NOT appear as candidates.
/// G-PLAY-COST-LTE: predicate not enforced at selection time in current engine.
/// When G-PLAY-COST-LTE is resolved, un-ignore this test.
#[test]
#[ignore = "pending: G-PLAY-COST-LTE — play_cost_lte predicate not evaluated in eval_card_fields"]
fn ex7_074_security_filters_liberator_by_cost_lte_4() {
    // Arrange: hand has a LIBERATOR with cost 5 and another with cost 3.
    // Assert: only the cost-3 card appears in the selection prompt.
    let _runner = ex7_074_runner();
}

/// After security resolves, this card adds itself to hand (raw_rust placeholder).
/// G-ADD-OPTION-SELF-TO-HAND: currently a no-op. When the gap is resolved,
/// EX7-074 should appear in the controller's hand after security resolution.
#[test]
#[ignore = "pending: G-ADD-OPTION-SELF-TO-HAND — EffectContext::add_security_option_to_hand missing"]
fn ex7_074_security_adds_self_to_hand_after_resolving() {
    // After security effect resolves: EX7-074 must be in player 0's hand.
    let _runner = ex7_074_runner();
}
