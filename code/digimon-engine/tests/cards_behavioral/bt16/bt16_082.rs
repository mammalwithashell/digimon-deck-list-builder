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
//! # Status: IMPLEMENTED
//!
//! The YAML uses native `on_move`, reveal/select/add/remainder steps, and a
//! `can_hatch`-guarded EffectChoice so the hatch prompt only appears when the
//! controller can legally hatch.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::action::space::SEL_REVEAL_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

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

fn digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card.colors = vec![CardColor::Red];
    card
}

fn tamer(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.colors = vec![CardColor::White];
    card
}

fn option(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Option;
    card.level = None;
    card.dp = None;
    card.play_cost = 2;
    card.colors = vec![CardColor::White];
    card
}

fn bt16_behavior_runner(deck: &[&str], digitama: &[&str]) -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT16-082")
        .expect("BT16-082 YAML parses")
        .add_card(digimon("MOVER"))
        .add_card(digimon("PICK-DIGI"))
        .add_card(tamer("PICK-TAMER"))
        .add_card(option("NOPE-OPTION"))
        .add_card(digimon("FILL"))
        .add_card(make_test_egg("EGG-HATCH", "Egg Hatch"))
        .deck(0, deck)
        .digitama(0, digitama)
        .memory(10)
        .build()
}

fn trigger_ukkomon_move(deck: &[&str], digitama: &[&str]) -> DebugRunner {
    let mut runner = bt16_behavior_runner(deck, digitama);
    runner.place_on_field(0, "BT16-082", Some(0));
    runner.place_in_breeding(0, "MOVER");
    assert!(
        runner.move_from_breeding(0),
        "move_from_breeding should succeed"
    );
    runner
}

fn top_three_with_digimon_tamer_option() -> [&'static str; 4] {
    ["FILL", "NOPE-OPTION", "PICK-TAMER", "PICK-DIGI"]
}

fn pick_reveal_and_order_remainder(runner: &mut DebugRunner, action_id: u16) {
    let view = runner.pending_selection_view().expect("reveal prompt");
    runner
        .execute_action(view.selecting_player, action_id)
        .expect("pick revealed card");
    runner.game.drain_effect_queue();

    while runner
        .pending_selection_view()
        .is_some_and(|view| matches!(view.kind, SelectionKind::OrderedPermutation { .. }))
    {
        let view = runner.pending_selection_view().expect("ordered prompt");
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("place revealed remainder");
        runner.game.drain_effect_queue();
    }
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

#[test]
fn bt16_082_clause_uses_on_move_timing_and_native_steps() {
    let runner = ukkomon();
    let compiled = runner
        .compiled_card("BT16-082")
        .expect("BT16-082 compiled card present");

    let clause = compiled
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .next()
        .expect("triggered clause present");

    assert!(
        clause.when.contains(&CompiledTiming::OnMove),
        "Ukkomon's printed timing is [When Moving], not a placeholder OnPlay"
    );
    assert!(matches!(
        clause.process[0],
        CompiledStep::RevealTopDeck { count: 3, .. }
    ));
    assert!(
        !clause
            .process
            .iter()
            .any(|step| matches!(step, CompiledStep::RawRust { .. })),
        "BT16-082 should use native reveal/add/remainder/hatch steps"
    );
}

// ---------------------------------------------------------------------------
// Section 2 — Behavioral tests
// ---------------------------------------------------------------------------
//
// Shared OnMove dispatch and DSL event context are covered elsewhere; these
// tests exercise BT16-082's card-specific reveal/select/hatch body.
//

/// After a Digimon moves from breeding to battle, Ukkomon's trigger should
/// install a Reveal+Select prompt (3 cards, add 1 Digimon or Tamer to hand).
#[test]
fn bt16_082_trigger_fires_on_move_from_breeding() {
    let runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    assert!(
        runner.pending_selection().is_some(),
        "Ukkomon's trigger must install a pending selection after move_from_breeding"
    );
}

/// The reveal+select step filters to Digimon and Tamer only (not Option cards).
#[test]
fn bt16_082_select_reveal_filters_to_digimon_or_tamer() {
    let runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    let view = runner
        .pending_selection_view()
        .expect("reveal selection should be pending");

    assert_eq!(view.kind, SelectionKind::Reveal);
    assert!(view.valid_action_ids.contains(&SEL_REVEAL_START));
    assert!(view.valid_action_ids.contains(&(SEL_REVEAL_START + 1)));
    assert!(
        !view.valid_action_ids.contains(&(SEL_REVEAL_START + 2)),
        "Option card among the revealed cards must not be selectable"
    );
}

/// When the player picks a Digimon from the 3 revealed cards, it goes to hand.
#[test]
fn bt16_082_picked_digimon_goes_to_hand() {
    let mut runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "PICK-DIGI"),
        "selected revealed Digimon should move to hand"
    );
}

/// When the player picks a Tamer from the 3 revealed cards, it goes to hand.
#[test]
fn bt16_082_picked_tamer_goes_to_hand() {
    let mut runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START + 1);

    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "PICK-TAMER"),
        "selected revealed Tamer should move to hand"
    );
}

/// The 2 un-picked revealed cards return to the BOTTOM of the deck.
#[test]
fn bt16_082_remainder_placed_at_deck_bottom() {
    let mut runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);

    let bottom_ids: Vec<_> = runner.game.players[0]
        .deck
        .iter()
        .take(2)
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert_eq!(
        bottom_ids,
        vec!["NOPE-OPTION".to_string(), "PICK-TAMER".to_string()],
        "unpicked revealed cards should be returned to the bottom in the chosen order"
    );
}

/// After reveal+add, the "you may hatch" step installs an EffectChoice prompt
/// ("Hatch" / "Don't hatch") when the player can hatch (digi-egg deck non-empty
/// AND breeding area empty).
#[test]
fn bt16_082_may_hatch_prompts_effect_choice_when_can_hatch() {
    let mut runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);

    let hatch = runner
        .pending_selection_view()
        .expect("hatch choice should be pending");
    assert_eq!(hatch.kind, SelectionKind::EffectChoice);
    let choices = hatch.effect_choices.expect("effect choice labels");
    let labels: Vec<_> = choices.iter().map(|choice| choice.label.as_str()).collect();
    assert_eq!(labels, vec!["Hatch", "Don't hatch"]);
}

/// If the player chooses "Hatch", a new Digimon egg moves into the breeding area.
#[test]
fn bt16_082_hatch_yes_moves_egg_to_breeding() {
    let mut runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);
    let hatch = runner.pending_selection_view().expect("hatch choice");
    runner
        .execute_action(hatch.selecting_player, hatch.valid_action_ids[0])
        .expect("choose Hatch");
    runner.game.drain_effect_queue();

    let breeding = runner.game.players[0]
        .breeding_area
        .as_ref()
        .expect("hatch choice should put an egg in breeding");
    assert_eq!(
        breeding.top_card().card_id(&runner.game.card_data),
        "EGG-HATCH"
    );
}

/// If the player chooses "Don't hatch" (or cannot hatch), no hatching occurs.
#[test]
fn bt16_082_hatch_no_does_not_change_breeding_area() {
    let mut runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);
    let hatch = runner.pending_selection_view().expect("hatch choice");
    runner
        .execute_action(hatch.selecting_player, hatch.valid_action_ids[1])
        .expect("choose Don't hatch");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.players[0].breeding_area.is_none(),
        "declining hatch should leave breeding empty after the moved Digimon left"
    );
    assert_eq!(
        runner.game.players[0].digitama_deck.len(),
        1,
        "declining hatch should not consume the digitama deck"
    );
}

#[test]
fn bt16_082_does_not_prompt_hatch_when_player_cannot_hatch() {
    let mut runner = trigger_ukkomon_move(&top_three_with_digimon_tamer_option(), &[]);
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);

    assert!(
        runner.pending_selection().is_none(),
        "no hatch choice should be surfaced when the player has no digitama"
    );
}

/// OPT lockout: a second move-from-breeding in the same turn does NOT
/// re-trigger Ukkomon's [Once Per Turn] clause.
///
/// First move: Ukkomon fires — install + fully resolve the reveal/select
/// (digitama empty so no hatch prompt follows), consuming the OPT slot.
/// Second move same turn: place a fresh mover, move it; Ukkomon's clause is
/// OPT-locked and must NOT install another reveal prompt.
#[test]
fn bt16_082_opt_blocks_second_trigger_same_turn() {
    // Six deck cards: three feed the first reveal, three remain so a second
    // reveal *could* draw if the OPT lockout were absent.
    let mut runner = bt16_behavior_runner(
        &[
            "PICK-DIGI",
            "PICK-TAMER",
            "FILL",
            "PICK-DIGI",
            "PICK-TAMER",
            "FILL",
        ],
        &[],
    );
    runner.place_on_field(0, "BT16-082", Some(0));

    // First move from breeding — Ukkomon's clause fires.
    runner.place_in_breeding(0, "MOVER");
    assert!(runner.move_from_breeding(0), "first move_from_breeding");
    assert!(
        runner.pending_selection().is_some(),
        "first move must install Ukkomon's reveal prompt"
    );
    // Resolve the reveal (pick first revealed card, order the remainder).
    // Digitama is empty so no hatch prompt follows — the clause fully
    // resolves and the OPT slot is now consumed.
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);
    assert!(
        runner.pending_selection().is_none(),
        "first trigger must fully resolve (no hatch prompt — empty digitama)"
    );

    // Second move from breeding, SAME turn — OPT lockout must suppress.
    runner.place_in_breeding(0, "MOVER");
    assert!(runner.move_from_breeding(0), "second move_from_breeding");
    assert!(
        runner.pending_selection().is_none(),
        "OPT lockout: second move in the same turn must NOT re-trigger Ukkomon"
    );
}

/// OPT resets after the turn rotates back: Ukkomon's clause fires again on
/// the controller's next turn.
#[test]
fn bt16_082_opt_resets_after_end_turn() {
    let mut runner = bt16_behavior_runner(
        &[
            "PICK-DIGI",
            "PICK-TAMER",
            "FILL",
            "PICK-DIGI",
            "PICK-TAMER",
            "FILL",
        ],
        &[],
    );
    runner.place_on_field(0, "BT16-082", Some(0));

    // First move on P0's turn — fires and consumes the OPT slot.
    runner.place_in_breeding(0, "MOVER");
    assert!(runner.move_from_breeding(0), "first move_from_breeding");
    assert!(
        runner.pending_selection().is_some(),
        "first move must install Ukkomon's reveal prompt"
    );
    pick_reveal_and_order_remainder(&mut runner, SEL_REVEAL_START);

    // Rotate the turn away and back to P0 so the OPT counter resets.
    runner.end_turn();
    if runner.game.pending_selection.is_some() {
        // Defensive: drain any end-of-turn prompt.
        let p = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .selecting_player;
        let a = runner
            .game
            .pending_selection
            .as_ref()
            .unwrap()
            .valid_action_ids[0];
        runner.game.resolve_selection(p, a).ok();
        runner.game.drain_effect_queue();
    }
    assert_eq!(
        runner.turn_player(),
        1,
        "precondition: P1's turn after first end_turn"
    );
    runner.end_turn();
    assert_eq!(runner.turn_player(), 0, "precondition: back to P0's turn");

    // New turn: move from breeding again — OPT has reset, clause re-fires.
    runner.place_in_breeding(0, "MOVER");
    assert!(runner.move_from_breeding(0), "next-turn move_from_breeding");
    assert!(
        runner.pending_selection().is_some(),
        "OPT reset: Ukkomon's clause must fire again on the controller's next turn"
    );
}

/// The trigger does NOT fire on the opponent's turn ([Your Turn] gate).
#[test]
fn bt16_082_does_not_trigger_on_opponent_turn() {
    let mut runner = bt16_behavior_runner(&top_three_with_digimon_tamer_option(), &["EGG-HATCH"]);
    runner.place_on_field(0, "BT16-082", Some(0));
    runner.place_in_breeding(0, "MOVER");
    runner.game.turn_player_idx = 1;

    assert!(
        runner.move_from_breeding(0),
        "move_from_breeding should still be possible in direct setup"
    );
    assert!(
        runner.pending_selection().is_none(),
        "[Your Turn] gate must suppress Ukkomon when it is not P0's turn"
    );
}
