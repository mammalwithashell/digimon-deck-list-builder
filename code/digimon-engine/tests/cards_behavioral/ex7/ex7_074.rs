//! EX7-074 Vortex Resonance — Option card, Cost 3, Multi-color (Green + Blue), [LIBERATOR].
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
//! - D3 Color ignore / bypass through option use_requirement
//! - Effect-initiated digivolve with cost reduce 4 (main_from_hand option)
//! - Security: hand-or-trash play (LIBERATOR, cost ≤ 4) via select_union_zone +
//!   play_union_bound_free + native add_this_option_to_hand
//!
//! # Harness idioms
//! - `[Main]` clause driven through `activate_hand_main` (same as P-035/P-103/P-151
//!   sibling Option tests). The reveal-pick, optional digivolve-permanent pick, and
//!   hand-card pick each surface as a `pending_selection` resolved with explicit
//!   `execute_action` so branch-specific behavior is asserted (no blind auto_resolve).
//! - `[Security]` inherited clause driven through the real combat/security-check path:
//!   EX7-074 seeded into the defender's security stack, then `attack_player` flips it
//!   and runs its inherited [Security] clause (same idiom as bt17_095's Clause C).

#![allow(unused_imports, dead_code, unused_mut, unused_variables)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledDpConstraint, CompiledScope, CompiledStep,
    CompiledTiming,
};
use digimon_engine::action::space::PASS;
use digimon_engine::action::PLAY_HAND_START;
use digimon_engine::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

// Load the YAML at compile time so tests run against the exact shipped spec.
const EX7_074_YAML: &str = include_str!("../../../cards/ex7/EX7-074.yaml");

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

/// A LIBERATOR-trait Lv.5 Digimon — eligible reveal-pick / digivolve result.
fn make_liberator_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(6000);
    c.play_cost = 4;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A LIBERATOR-trait Lv.3 Digimon with a low play cost (≤ 4) — a valid
/// security free-play candidate.
fn make_liberator_low_cost(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A LIBERATOR-trait Lv.6 Digimon with a high play cost (> 4) — excluded from
/// the security free-play by the `play_cost_lte: 4` filter.
fn make_liberator_high_cost(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = 7;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A LIBERATOR Tamer.
fn make_liberator_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.traits = vec!["LIBERATOR".to_string()];
    c.colors = vec![CardColor::Green];
    c
}

/// A non-LIBERATOR filler Digimon — never eligible for a LIBERATOR-trait pick.
fn make_filler(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = Vec::new();
    c.colors = vec![CardColor::Red];
    c
}

/// A Lv.4 Red carrier Digimon for the field — the digivolve sub-step target.
fn make_carrier(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(5000);
    c.play_cost = 5;
    c.colors = vec![CardColor::Red];
    c
}

/// A Lv.5 Red Digimon card whose printed digivolution cost from a Lv.4 Red
/// base is `memory_cost`. Used as the digivolve result in the cost-reduction
/// test so the effective memory spend is deterministic.
fn make_evo_target(id: &str, memory_cost: u16) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(5);
    c.dp = Some(7000);
    c.play_cost = 6;
    c.colors = vec![CardColor::Red];
    c.evo_costs = vec![EvoCost {
        // card_color 0 == Red (CardColor::Red); level 4 == carrier's level.
        card_color: 0,
        level: 4,
        memory_cost,
    }];
    c
}

/// A Tamer card — never a valid digivolve result (must be a Digimon).
fn make_tamer(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
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

/// Resolve every mandatory pending selection by picking its first valid action.
/// Stops as soon as no selection is pending or the pending one is optional.
fn resolve_required_until_optional_or_done(runner: &mut DebugRunner) {
    while runner.pending_selection().is_some() && !runner.pending_is_optional() {
        let view = runner
            .pending_selection_view()
            .expect("pending selection must expose a view");
        let action = *view
            .valid_action_ids
            .first()
            .expect("a mandatory selection must offer at least one action");
        runner
            .execute_action(view.selecting_player, action)
            .expect("mandatory selection resolves");
    }
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
///   2: on_security inherited triggered (optional play LIBERATOR ≤4 from hand or trash)
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
        assert!(
            !t.optional,
            "Main clause must NOT be optional at clause level (printed text has no outer 'you may')"
        );
    }
}

/// Clause 2 is the inherited Security triggered clause (on_security timing).
/// "You may" → optional: true.
/// DCGO: EffectTiming.SecuritySkill, activateClass.SetIsSecurityEffect(true).
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
// Section 2: Color bypass (declarative + runtime use_requirement)
// ---------------------------------------------------------------------------

/// The IgnoreColorRequirement flood_gate clause compiles correctly.
#[test]
fn ex7_074_color_bypass_compiles_as_declarative() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    assert!(
        matches!(compiled.effects[0], CompiledClause::Declarative(_)),
        "Flood gate clause (clause 0) must compile as Declarative (got {:?})",
        compiled.effects[0]
    );
}

/// Positive: with a LIBERATOR Digimon on the board, the use_requirement is
/// satisfied and EX7-074 becomes playable despite the color mismatch.
#[test]
fn ex7_074_color_bypass_unlocks_play_when_liberator_on_field() {
    let mut red_liberator = make_liberator_digimon("RED-LIBERATOR");
    red_liberator.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(red_liberator)
        .hand(0, &["EX7-074"])
        .memory(10)
        .start();

    let before = build_action_mask(&runner.game, 0);
    assert_eq!(
        before[PLAY_HAND_START as usize], 0.0,
        "EX7-074 should be blocked without matching color or LIBERATOR bypass"
    );

    runner.place_on_field(0, "RED-LIBERATOR", Some(0));

    let after = build_action_mask(&runner.game, 0);
    assert_eq!(
        after[PLAY_HAND_START as usize], 1.0,
        "EX7-074 should be playable when LIBERATOR board presence enables its use requirement"
    );
}

/// Negative: without any LIBERATOR Digimon or Tamer on the board, the color
/// bypass is inactive and the off-color Option stays unplayable.
#[test]
fn ex7_074_color_bypass_inactive_without_liberator_on_field() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074"])
        .memory(10)
        .start();

    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[PLAY_HAND_START as usize], 0.0,
        "without LIBERATOR board presence the color requirement must still apply"
    );
}

/// A LIBERATOR Tamer (not just a Digimon) also satisfies the use_requirement.
#[test]
fn ex7_074_color_bypass_unlocks_play_with_liberator_tamer() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_tamer("LIB-TAMER"))
        .hand(0, &["EX7-074"])
        .memory(10)
        .start();

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        0.0,
        "blocked before any LIBERATOR is on the board"
    );

    runner.place_on_field(0, "LIB-TAMER", Some(0));

    assert_eq!(
        build_action_mask(&runner.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "a LIBERATOR Tamer must also satisfy the color-bypass use_requirement"
    );
}

/// The security clause enforces `play_cost_lte: 4` through the union-zone card
/// filter and uses the native `add_this_option_to_hand` tail (no legacy
/// raw_rust shim).
#[test]
fn ex7_074_security_uses_play_cost_filters_and_native_self_to_hand() {
    let runner = ex7_074_runner();
    let compiled = runner
        .compiled_card("EX7-074")
        .expect("EX7-074 must be in compiled_cards");
    let CompiledClause::Triggered(security) = &compiled.effects[2] else {
        panic!("security clause must be triggered");
    };

    fn has_union_with_cost_lte(steps: &[CompiledStep], cap: i32) -> bool {
        steps.iter().any(|step| match step {
            CompiledStep::SelectUnionZone { filter, .. } => {
                filter.play_cost_lte == Some(CompiledDpConstraint::Literal(cap))
                    || filter.all_of.iter().any(|nested| {
                        nested.play_cost_lte == Some(CompiledDpConstraint::Literal(cap))
                    })
            }
            CompiledStep::If { then, .. } => has_union_with_cost_lte(then, cap),
            _ => false,
        })
    }

    assert!(
        has_union_with_cost_lte(&security.process, 4),
        "security union-zone selection must enforce play_cost_lte: 4"
    );
    assert!(
        security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::AddThisOptionToHand)),
        "security clause must use native add_this_option_to_hand"
    );
    assert!(
        !security
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "EX7-074 should not use the legacy raw_rust self-to-hand shim"
    );
}

// ---------------------------------------------------------------------------
// Section 3: Main clause — reveal + add to hand + digivolve sub-step
// ---------------------------------------------------------------------------

/// Firing the [Main] effect with LIBERATOR cards among the top 3 installs a
/// pending selection (the reveal-pick prompt).
#[test]
fn ex7_074_main_reveals_three_and_installs_pick_prompt() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["EX7-074"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "activate_hand_main must return true for EX7-074");

    assert!(
        runner.game.pending_selection.is_some(),
        "EX7-074 [Main] must install a pending selection (reveal-pick) when LIBERATOR in top 3"
    );
}

/// After reveal + selecting 1 LIBERATOR card, the selected card moves to hand.
/// DCGO: mode: SelectCardEffect.Mode.AddHand → selected card added to controller's hand.
#[test]
fn ex7_074_main_adds_liberator_from_reveal_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["EX7-074"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len(); // 1 (EX7-074)
    runner.game.activate_hand_main(0, 0);

    // Pick the revealed LIBERATOR card explicitly (the reveal-pick prompt).
    let view = runner
        .pending_selection_view()
        .expect("reveal-pick prompt must be pending");
    let action = *view
        .valid_action_ids
        .first()
        .expect("reveal-pick must offer the LIBERATOR card");
    runner
        .execute_action(view.selecting_player, action)
        .expect("reveal-pick resolves");

    resolve_required_until_optional_or_done(&mut runner);

    // Decline the optional digivolve-permanent pick to keep counts clean.
    if runner.pending_selection().is_some() && runner.pending_is_optional() {
        let _ = runner.execute_action(0, PASS);
    }
    resolve_required_until_optional_or_done(&mut runner);

    // EX7-074 stays in hand (activate_hand_main doesn't consume it); one
    // LIBERATOR card was added from the reveal → hand grows by net +1.
    let hand_after = runner.game.players[0].hand.len();
    assert_eq!(
        hand_after,
        hand_before + 1,
        "Hand must grow by 1 (one LIBERATOR added from reveal; EX7-074 stays); \
         before={hand_before}, after={hand_after}"
    );
}

/// After reveal, the non-selected cards return to the BOTTOM of the deck.
/// DCGO: remainingCardsPlace: RemainingCardsPlace.DeckBottom.
/// Net deck delta: 3 revealed − 2 returned − 1 to hand = deck shrinks by 1.
#[test]
fn ex7_074_main_reveal_remainder_placed_at_deck_bottom() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_digimon("LIB"))
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074"])
        // LIB on top (revealed first), then fillers below.
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "LIB"])
        .memory(10)
        .start();

    let deck_before = runner.deck_size(0);
    runner.game.activate_hand_main(0, 0);

    // Pick the revealed LIBERATOR card.
    let view = runner
        .pending_selection_view()
        .expect("reveal-pick prompt must be pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("reveal-pick resolves");
    resolve_required_until_optional_or_done(&mut runner);
    if runner.pending_selection().is_some() && runner.pending_is_optional() {
        let _ = runner.execute_action(0, PASS);
    }
    resolve_required_until_optional_or_done(&mut runner);

    let deck_after = runner.deck_size(0);
    assert_eq!(
        deck_before - deck_after,
        1,
        "Deck must shrink by exactly 1 (3 revealed, 1 added to hand, 2 returned to bottom); \
         before={deck_before}, after={deck_after}"
    );
}

/// Negative: when no LIBERATOR card is among the top 3, no card is added to
/// hand — the optional reveal-pick yields no candidate and all 3 revealed
/// cards return to the bottom of the deck (deck size unchanged).
#[test]
fn ex7_074_main_no_liberator_in_top_3_does_not_add_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_filler("FILL"))
        .hand(0, &["EX7-074"])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(10)
        .start();

    let hand_before = runner.game.players[0].hand.len(); // 1 (EX7-074)
    let deck_before = runner.deck_size(0);

    runner.game.activate_hand_main(0, 0);

    // No LIBERATOR among the top 3 → the optional reveal-pick has no candidate.
    // Decline any optional prompt; resolve remaining mandatory steps.
    resolve_required_until_optional_or_done(&mut runner);
    while runner.pending_selection().is_some() && runner.pending_is_optional() {
        let _ = runner.execute_action(0, PASS);
        resolve_required_until_optional_or_done(&mut runner);
    }

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "no LIBERATOR in the top 3 → nothing added to hand"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before,
        "with no card added, all 3 revealed cards return to the deck bottom"
    );
}

/// The digivolve sub-step: select_own_permanent is optional (canNoSelect: true).
/// After the reveal+add resolves, the digivolve-permanent prompt is optional.
#[test]
fn ex7_074_digivolve_substep_is_optional_declinable() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_digimon("LIB"))
        .add_card(make_carrier("CARRIER"))
        .add_card(make_evo_target("EVO", 6))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    // A Digimon on the field + a Digimon in hand → digivolve sub-step is reachable.
    runner.place_on_field(0, "CARRIER", Some(0));

    runner.game.activate_hand_main(0, 0);

    // Resolve the reveal-pick (mandatory branch of the flow).
    let view = runner
        .pending_selection_view()
        .expect("reveal-pick prompt must be pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("reveal-pick resolves");
    resolve_required_until_optional_or_done(&mut runner);

    // The next pending prompt is the optional digivolve-permanent pick.
    assert!(
        runner.pending_selection().is_some(),
        "the digivolve-permanent pick must install when a Digimon is on the field"
    );
    assert!(
        runner.pending_is_optional(),
        "the digivolve-permanent pick must be optional (DCGO canNoSelect: true)"
    );

    // Declining it must leave the field untouched (no digivolve).
    let field_before = runner.battle_area_size(0);
    let _ = runner.execute_action(0, PASS);
    resolve_required_until_optional_or_done(&mut runner);
    while runner.pending_selection().is_some() && runner.pending_is_optional() {
        let _ = runner.execute_action(0, PASS);
        resolve_required_until_optional_or_done(&mut runner);
    }
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "declining the digivolve sub-step must not change the battle area"
    );
}

/// The digivolve sub-step reduces the digivolution cost by 4.
/// DCGO: reduceCostTuple: (reduceCost: 4, reduceCostCardCondition: null).
/// Carrier is Lv.4 Red; EVO's printed digivolve cost from a Lv.4 Red base is 6
/// → after reduce-4 the controller pays 2 memory.
#[test]
fn ex7_074_digivolve_substep_cost_reduced_by_4() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_digimon("LIB"))
        .add_card(make_carrier("CARRIER"))
        .add_card(make_evo_target("EVO", 6))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "CARRIER", Some(0));

    runner.game.activate_hand_main(0, 0);

    // Resolve the reveal-pick.
    let view = runner
        .pending_selection_view()
        .expect("reveal-pick prompt must be pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("reveal-pick resolves");
    resolve_required_until_optional_or_done(&mut runner);

    let memory_before = runner.memory();

    // Drive the digivolve sub-step: pick the carrier permanent, then the EVO
    // card from hand. Both prompts are resolved with explicit first-action
    // picks (only one valid target / one Digimon in hand each).
    assert!(
        runner.pending_selection().is_some(),
        "digivolve-permanent pick must install"
    );
    let view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("pick the carrier permanent to digivolve");
    resolve_required_until_optional_or_done(&mut runner);

    // The select_hand prompt (digivolve result card) — pick EVO.
    if runner.pending_selection().is_some() {
        let view = runner.pending_selection_view().unwrap();
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("pick the EVO Digimon card from hand");
    }
    resolve_required_until_optional_or_done(&mut runner);
    while runner.pending_selection().is_some() && runner.pending_is_optional() {
        let _ = runner.execute_action(0, PASS);
        resolve_required_until_optional_or_done(&mut runner);
    }

    // EVO printed digivolve cost from a Lv.4 base is 6; reduce-4 → pay 2.
    let memory_spent = memory_before - runner.memory();
    assert_eq!(
        memory_spent, 2,
        "digivolve cost must be reduced by 4 (printed 6 → effective 2); spent {memory_spent}"
    );
    // The carrier's stack grew (EVO stacked on top).
    let carrier_top = runner.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        carrier_top, "EVO",
        "after the digivolve, EVO must be the top card of the carrier's stack"
    );
}

/// The digivolve sub-step: the result card in hand must be a Digimon.
/// With only a Tamer in hand (besides EX7-074), the select_hand prompt offers
/// no candidate and no digivolve occurs.
#[test]
fn ex7_074_digivolve_substep_only_offers_digimon_hand_cards() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_digimon("LIB"))
        .add_card(make_carrier("CARRIER"))
        .add_card(make_tamer("TAMER"))
        .hand(0, &["EX7-074", "TAMER"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    let carrier = runner.place_on_field(0, "CARRIER", Some(0));
    let field_before = runner.battle_area_size(0);

    runner.game.activate_hand_main(0, 0);

    // Resolve the reveal-pick.
    let view = runner
        .pending_selection_view()
        .expect("reveal-pick prompt must be pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("reveal-pick resolves");
    resolve_required_until_optional_or_done(&mut runner);

    // Digivolve-permanent pick is optional; resolve the whole flow by taking
    // first action / passing. No Digimon in hand → digivolve cannot complete.
    while runner.pending_selection().is_some() {
        let view = runner.pending_selection_view().unwrap();
        let action = view
            .valid_action_ids
            .first()
            .copied()
            .unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
    }

    // The Tamer cannot be a digivolve result → no new permanent, carrier
    // top unchanged.
    assert_eq!(
        runner.battle_area_size(0),
        field_before,
        "no Digimon in hand → the digivolve sub-step adds nothing to the field"
    );
    let carrier_top = runner.game.players[0].battle_area[carrier.index as usize]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        carrier_top, "CARRIER",
        "with only a Tamer in hand, the carrier must not have digivolved"
    );
}

/// If no own Digimon is on the field, the digivolve sub-step's optional
/// permanent pick has no candidate and is skipped — the effect resolves cleanly.
#[test]
fn ex7_074_digivolve_substep_skipped_when_no_eligible_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(make_liberator_digimon("LIB"))
        .add_card(make_evo_target("EVO", 6))
        .hand(0, &["EX7-074", "EVO"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    // No Digimon on the field — the digivolve-permanent pick has no candidate.
    assert_eq!(runner.battle_area_size(0), 0, "precondition: empty field");

    runner.game.activate_hand_main(0, 0);

    // Resolve the reveal-pick.
    let view = runner
        .pending_selection_view()
        .expect("reveal-pick prompt must be pending");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("reveal-pick resolves");

    // Resolve the remainder of the flow without panic.
    while runner.pending_selection().is_some() {
        let view = runner.pending_selection_view().unwrap();
        let action = view
            .valid_action_ids
            .first()
            .copied()
            .unwrap_or(PASS);
        let _ = runner.execute_action(view.selecting_player, action);
    }

    // The effect resolved with no digivolve (no field Digimon).
    assert_eq!(
        runner.battle_area_size(0),
        0,
        "with no own Digimon on the field, the digivolve sub-step adds nothing"
    );
}

/// Full-flow integration: play EX7-074 as an Option from hand, auto-resolve all
/// prompts → no panic. Confirms the real Option play path works end-to-end.
#[test]
fn ex7_074_full_main_play_flow_no_panic() {
    let mut red_liberator = make_liberator_digimon("RED-LIB");
    red_liberator.colors = vec![CardColor::Red];

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(red_liberator)
        .add_card(make_liberator_digimon("LIB"))
        .hand(0, &["EX7-074"])
        .deck(0, &["LIB", "LIB", "LIB", "LIB", "LIB", "LIB"])
        .memory(10)
        .start();

    // Color bypass: a LIBERATOR Digimon on the field unlocks the off-color play.
    runner.place_on_field(0, "RED-LIB", Some(0));

    runner.play(0, 0);
    runner.auto_resolve().expect("Option play resolves");
    // Reaching here without panic == PASS.
}

// ---------------------------------------------------------------------------
// Section 4: Inherited Security clause — play LIBERATOR ≤4 from hand or trash
// ---------------------------------------------------------------------------
//
// Driven through the real combat/security-check path: EX7-074 seeded into the
// defender's security stack, an attacker on the attacker's field, then
// `attack_player` flips the security card and runs EX7-074's inherited
// [Security] clause for the defender (same idiom as bt17_095's Clause C).

/// With a LIBERATOR card (cost ≤ 4) ONLY in the defender's hand, the security
/// clause's union-zone selection collapses to the hand zone and installs a
/// pending selection so the defender can pick the card to play free.
#[test]
fn ex7_074_security_installs_hand_selection_when_only_hand_eligible() {
    let mut attacker = make_filler("EX7074-ATK-HAND");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_liberator_low_cost("LIB-LOW"))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-LOW"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-HAND", Some(0));
    assert_eq!(runner.security_count(1), 1, "precondition: EX7-074 in security");

    let _ = runner.attack_player(attacker_handle, 1, false);

    // The security check flips EX7-074 and runs its inherited [Security] clause.
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("security clause must install a union-zone selection");
    assert!(
        matches!(sel.kind, SelectionKind::UnionZone { .. }),
        "the security prompt must be the hand-or-trash union-zone pick; got {:?}",
        sel.kind
    );
    // Only one eligible card (the hand LIBERATOR); the selection is optional
    // ("you may"), so PASS is also legal.
    let card_actions: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        card_actions.len(),
        1,
        "exactly one eligible LIBERATOR card (in hand); got {card_actions:?}"
    );
}

/// With a LIBERATOR card in BOTH the defender's hand and trash, the union-zone
/// selection spans both zones (HAND | TRASH).
#[test]
fn ex7_074_security_installs_zone_choice_when_liberator_in_both_zones() {
    let mut attacker = make_filler("EX7074-ATK-BOTH");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_liberator_low_cost("LIB-HAND"))
        .add_card(make_liberator_low_cost("LIB-TRASH"))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["LIB-TRASH"])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    // Seed defender's trash with a LIBERATOR card.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("LIB-TRASH seeded in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-BOTH", Some(0));

    let _ = runner.attack_player(attacker_handle, 1, false);

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("security clause must install a union-zone selection");
    assert_eq!(
        sel.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        },
        "with eligible cards in both zones the union-zone pick must span hand AND trash"
    );
    let card_actions: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        card_actions.len(),
        2,
        "two eligible LIBERATOR cards (one hand, one trash); got {card_actions:?}"
    );
}

/// Negative: with no LIBERATOR card (cost ≤ 4) in hand or trash, the security
/// clause plays nothing, but the mandatory `add_this_option_to_hand` tail still
/// routes EX7-074 from the security stack to the defender's hand.
#[test]
fn ex7_074_security_no_op_when_no_liberator_available() {
    let mut attacker = make_filler("EX7074-ATK-EMPTY");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_filler("FILL"))
        .hand(0, &["FILL"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-EMPTY", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    assert_eq!(runner.trash_size(1), 0, "precondition: defender trash empty");
    assert_eq!(runner.security_count(1), 1, "precondition: EX7-074 in security");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // No eligible LIBERATOR card → nothing played to the field.
    assert_eq!(
        runner.battle_area_size(1),
        0,
        "no LIBERATOR in hand or trash → nothing played"
    );
    // The mandatory tail still ran: EX7-074 left security and went to hand.
    assert_eq!(runner.security_count(1), 0, "EX7-074 left the security stack");
    assert_eq!(
        runner.hand_size(1),
        1,
        "add_this_option_to_hand must route EX7-074 to the defender's hand even with no play"
    );
    let hand_id = runner.game.players[1].hand[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        hand_id, "EX7-074",
        "the card added to the defender's hand must be EX7-074 itself"
    );
}

/// Positive: the security clause plays the selected LIBERATOR card from hand
/// without paying its cost (memory unchanged), then routes EX7-074 to hand.
#[test]
fn ex7_074_security_plays_liberator_from_hand_free() {
    let mut attacker = make_filler("EX7074-ATK-PLAYHAND");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_liberator_low_cost("LIB-LOW"))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-LOW"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-PLAYHAND", Some(0));
    let memory_before = runner.memory();

    let _ = runner.attack_player(attacker_handle, 1, false);

    // Pick the hand LIBERATOR explicitly from the union-zone selection.
    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must be pending");
    let card_action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("union-zone must offer the hand LIBERATOR");
    runner
        .execute_action(view.selecting_player, card_action)
        .expect("select the hand LIBERATOR to play free");
    runner.auto_resolve().expect("security resolves");

    // The LIBERATOR card is on the defender's field, played without paying cost.
    let field_ids: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        field_ids.contains(&"LIB-LOW".to_string()),
        "the selected LIBERATOR card must be played to the defender's field; got {field_ids:?}"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_union_bound_free must not charge the LIBERATOR card's play cost"
    );
    // EX7-074 routed to the defender's hand by the mandatory tail.
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "EX7-074"),
        "add_this_option_to_hand must route EX7-074 to the defender's hand"
    );
}

/// Positive: the security clause plays the selected LIBERATOR card from trash
/// without paying its cost. The union-zone selection collapses to the trash
/// zone when only the trash holds an eligible card.
#[test]
fn ex7_074_security_plays_liberator_from_trash_free() {
    let mut attacker = make_filler("EX7074-ATK-PLAYTRASH");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_liberator_low_cost("LIB-LOW"))
        .add_card(make_filler("FILL"))
        .deck(0, &["FILL"; 5])
        .deck(1, &["LIB-LOW"])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    // Seed a LIBERATOR card ONLY into the defender's trash.
    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("LIB-LOW seeded in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-PLAYTRASH", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");
    let memory_before = runner.memory();

    let _ = runner.attack_player(attacker_handle, 1, false);

    // The union-zone selection's only card action targets the trash slot.
    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must be pending");
    let card_action = *view
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .expect("union-zone must offer the trash LIBERATOR");
    runner
        .execute_action(view.selecting_player, card_action)
        .expect("select the trash LIBERATOR to play free");
    runner.auto_resolve().expect("security resolves");

    let field_ids: Vec<String> = runner.game.players[1]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(
        field_ids.contains(&"LIB-LOW".to_string()),
        "the selected LIBERATOR card must be played from trash to the field; got {field_ids:?}"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "play_union_bound_free must not charge the LIBERATOR card's play cost"
    );
}

/// Cost ≤ 4 filter: LIBERATOR cards with play cost > 4 must NOT appear as
/// candidates in the security union-zone selection.
#[test]
fn ex7_074_security_filters_liberator_by_cost_lte_4() {
    let mut attacker = make_filler("EX7074-ATK-COSTFILTER");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_liberator_low_cost("LIB-LOW"))   // cost 3 — eligible
        .add_card(make_liberator_high_cost("LIB-HIGH")) // cost 7 — excluded
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-LOW", "LIB-HIGH"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-COSTFILTER", Some(0));

    let _ = runner.attack_player(attacker_handle, 1, false);

    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("security clause must install a union-zone selection");
    assert!(
        matches!(sel.kind, SelectionKind::UnionZone { .. }),
        "the security prompt must be the union-zone pick; got {:?}",
        sel.kind
    );
    let card_actions: Vec<u16> = sel
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        card_actions.len(),
        1,
        "only the cost-3 LIBERATOR is eligible; the cost-7 LIBERATOR must be filtered out \
         by play_cost_lte: 4; got {card_actions:?}"
    );
}

/// After the security clause resolves, EX7-074 adds itself to the defender's
/// hand (native `add_this_option_to_hand`). Verified after a real play branch.
#[test]
fn ex7_074_security_adds_self_to_hand_after_resolving() {
    let mut attacker = make_filler("EX7074-ATK-SELFHAND");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_liberator_low_cost("LIB-LOW"))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-LOW"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-SELFHAND", Some(0));
    assert_eq!(runner.security_count(1), 1, "precondition: EX7-074 in security");

    let _ = runner.attack_player(attacker_handle, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(
        runner.security_count(1),
        0,
        "EX7-074 must leave the security stack after its [Security] clause resolves"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "EX7-074"),
        "after security resolution EX7-074 must be in the defender's hand"
    );
}

/// Declining the optional security play (PASS on the union-zone selection)
/// plays nothing, but the mandatory `add_this_option_to_hand` tail still fires.
#[test]
fn ex7_074_security_decline_play_still_adds_self_to_hand() {
    let mut attacker = make_filler("EX7074-ATK-DECLINE");
    attacker.dp = Some(8000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(EX7_074_YAML)
        .expect("EX7-074 YAML must parse and compile")
        .add_card(attacker.clone())
        .add_card(make_liberator_low_cost("LIB-LOW"))
        .add_card(make_filler("FILL"))
        .hand(1, &["LIB-LOW"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .security(1, &["EX7-074"])
        .memory(10)
        .start();

    let attacker_handle = runner.place_on_field(0, "EX7074-ATK-DECLINE", Some(0));

    let _ = runner.attack_player(attacker_handle, 1, false);

    // Decline the optional union-zone play with PASS.
    let view = runner
        .pending_selection_view()
        .expect("union-zone selection must be pending");
    assert!(
        runner.pending_is_optional(),
        "the security play is optional ('you may') — PASS must be legal"
    );
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the optional security play");
    runner.auto_resolve().expect("security resolves after decline");

    // Nothing played — LIB-LOW remains in the defender's hand.
    assert!(
        runner.battle_area_size(1) == 0,
        "declining the security play must not put any card on the field"
    );
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LIB-LOW"),
        "the declined LIBERATOR card must remain in the defender's hand"
    );
    // The mandatory tail still routes EX7-074 to hand.
    assert_eq!(runner.security_count(1), 0, "EX7-074 left the security stack");
    assert!(
        runner.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "EX7-074"),
        "add_this_option_to_hand must fire even when the optional play is declined"
    );
}
