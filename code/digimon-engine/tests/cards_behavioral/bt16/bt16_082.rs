//! BT16-082 Ukkomon — Digimon, Lv.3, White, 2000 DP, Cost 3.
//! Traits: Ancient Fairy
//!
//! # Card text (cards.json)
//!
//! [Your Turn] [Once Per Turn] When one of your Digimon moves from the
//! breeding area to the battle area, reveal the top 3 cards of your deck.
//! Add 1 Digimon card or Tamer card among them to the hand. Return the
//! rest to the bottom of the deck. Then, you may hatch in your breeding area.
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT16/White/BT16_082.cs
//!
//! Trigger timing: EffectTiming.OnMove — fires when this player's Digimon
//! moves from breeding area to battle area. Ukkomon must be in the battle
//! area to observe. CanUseCondition checks:
//!   1. Ukkomon is in the battle area (IsExistOnBattleArea).
//!   2. It is the owner's turn (IsOwnerTurn).
//!   3. The breeding-to-battle move event matches the trigger (CanTriggerOnMove).
//! Effect body (ActivateCoroutine):
//!   1. RevealDeckTopCardsAndSelect: reveal 3, pick 1 Digimon or Tamer → hand,
//!      remainder → DeckBottom.
//!   2. If card.Owner.CanHatch: present yes/no ("Hatch" / "Not hatch").
//!      If yes → HatchDigiEggClass.Hatch().
//!
//! # Patterns this test covers
//! - OnMove dispatch and DSL event context are covered by shared timing_dispatch
//!   and phase3d_event_context tests.
//! - A1 (reveal 3 → add Digimon or Tamer to hand) — behavioral body documented
//! - E2-adjacent (OPT + [Your Turn] gate) — structural assertions pass
//! - B-hatch: "you may hatch" step — behavioral body documented
//!
//! # Status: BLOCKED (card body placeholder)
//!
//! The YAML uses a `kind: raw_rust` no-op escape hatch (bt16_082_on_move_noop)
//! as a placeholder for the real reveal/select/hatch body and still uses a
//! structural stub timing. Behavioral tests remain #[ignore]'d until BT16-082's
//! card-specific move-trigger effect is implemented. Structural tests (compiled
//! card shape) pass.

use digimon_dsl::compiled::{CompiledClause, CompiledScope};
use digimon_engine::debug_runner::DebugRunner;

// ---------------------------------------------------------------------------
// Helper builders
// ---------------------------------------------------------------------------

fn ukkomon() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT16-082")
        .expect("BT16-082 YAML parses and compiles")
        .memory(10)
        .build()
}

// ---------------------------------------------------------------------------
// Section 1 — Structural assertions (active — do not ignore)
// ---------------------------------------------------------------------------

/// Ukkomon has exactly one triggered clause.
#[test]
fn bt16_082_has_one_triggered_clause() {
    let runner = ukkomon();
    let compiled = runner
        .compiled_card("BT16-082")
        .expect("BT16-082 compiled card present");

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
        1,
        "Ukkomon has exactly one triggered clause"
    );
}

/// The triggered clause has a FaceUp (own, non-inherited) scope.
/// Ukkomon sits in the battle area and watches for any of its owner's Digimon
/// to move — this is an own-effect, not an inherited trigger.
#[test]
fn bt16_082_clause_scope_is_face_up() {
    let runner = ukkomon();
    let compiled = runner
        .compiled_card("BT16-082")
        .expect("BT16-082 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert_eq!(
        clause.scope,
        CompiledScope::FaceUp,
        "Ukkomon's trigger is FaceUp (own scope, not inherited)"
    );
}

/// The triggered clause is marked [Once Per Turn].
#[test]
fn bt16_082_clause_is_once_per_turn() {
    let runner = ukkomon();
    let compiled = runner
        .compiled_card("BT16-082")
        .expect("BT16-082 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert!(
        clause.once_per_turn,
        "Ukkomon's clause must be marked once_per_turn"
    );
}

/// The clause is NOT optional at the clause level — the trigger fires
/// automatically when a Digimon moves. The "you may hatch" is an internal
/// optional step, not a clause-level opt-in.
#[test]
fn bt16_082_clause_is_not_optional() {
    let runner = ukkomon();
    let compiled = runner
        .compiled_card("BT16-082")
        .expect("BT16-082 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert!(
        !clause.optional,
        "Ukkomon's main clause fires automatically (not player-opt-in)"
    );
}

/// The [Your Turn] gate must be encoded as active_when on the clause.
#[test]
fn bt16_082_clause_has_your_turn_active_when() {
    let runner = ukkomon();
    let compiled = runner
        .compiled_card("BT16-082")
        .expect("BT16-082 compiled card present");

    let triggered: Vec<_> = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();

    let clause = triggered[0];
    assert!(
        clause.active_when.is_some(),
        "Ukkomon's clause must have active_when ([Your Turn] gate)"
    );
}

// ---------------------------------------------------------------------------
// Section 2 — Behavioral tests (ignored: BT16-082 body placeholder)
// ---------------------------------------------------------------------------
//
// Shared OnMove dispatch and DSL event context are covered elsewhere. BT16-082's
// YAML still has a structural stub clause with a raw_rust no-op body, so these
// tests remain ignored until the card-specific reveal/select/hatch behavior is
// implemented.
//

/// After a Digimon moves from breeding to battle, Ukkomon's trigger should
/// install a Reveal+Select prompt (3 cards, add 1 Digimon or Tamer to hand).
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_trigger_fires_on_move_from_breeding() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT16-082")
        .expect("BT16-082 YAML parses")
        .dsl_card("ST1-03")
        .expect("ST1-03 parses")
        .memory(10)
        .build();

    // Place Ukkomon in battle area as the observer
    runner.place_on_field(0, "BT16-082", None);

    // Set up: a Digimon in the breeding area (hatched) and deck to reveal from
    runner.game.hatch(0); // move egg from digitama deck to breeding

    // Move Digimon from breeding to battle — should fire Ukkomon's trigger
    let moved = runner.move_from_breeding(0);
    assert!(moved, "move_from_breeding should succeed");

    assert!(
        runner.pending_selection().is_some(),
        "Ukkomon's trigger must install a pending selection after move_from_breeding"
    );
}

/// The reveal+select step filters to Digimon and Tamer only (not Option cards).
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_select_reveal_filters_to_digimon_or_tamer() {
    // After OnMove fires and reveals 3 cards, the select_reveal pending
    // selection must exclude Option cards from valid_action_ids.
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// When the player picks a Digimon from the 3 revealed cards, it goes to hand.
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_picked_digimon_goes_to_hand() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// When the player picks a Tamer from the 3 revealed cards, it goes to hand.
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_picked_tamer_goes_to_hand() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// The 2 un-picked revealed cards return to the BOTTOM of the deck.
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_remainder_placed_at_deck_bottom() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// After reveal+add, the "you may hatch" step installs an EffectChoice prompt
/// ("Hatch" / "Don't hatch") when the player can hatch (digi-egg deck non-empty
/// AND breeding area empty).
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_may_hatch_prompts_effect_choice_when_can_hatch() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// If the player chooses "Hatch", a new Digimon egg moves into the breeding area.
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_hatch_yes_moves_egg_to_breeding() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// If the player chooses "Don't hatch" (or cannot hatch), no hatching occurs.
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_hatch_no_does_not_change_breeding_area() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// OPT lockout: second move-from-breeding in the same turn does NOT re-trigger.
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_opt_blocks_second_trigger_same_turn() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// OPT resets after end_turn: the trigger fires again on the player's next turn.
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_opt_resets_after_end_turn() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}

/// The trigger does NOT fire on the opponent's turn ([Your Turn] gate).
#[test]
#[ignore = "pending: BT16-082 card body still lacks the real move-trigger effect; shared OnMove dispatch is covered by timing_dispatch and DSL event-context tests"]
fn bt16_082_does_not_trigger_on_opponent_turn() {
    unimplemented!("pending BT16-082 card-specific move-trigger body");
}
