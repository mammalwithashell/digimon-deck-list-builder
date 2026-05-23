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
//!   option card (main_from_hand timing, optional permanent selection).
//!   IMPLEMENTED in pure DSL.
//! - Clause B ([Start of Your Turn] <Delay>): `kind: delay` clause —
//!   `active_when` gates on the opponent having a Digimon; the body returns a
//!   selected red Digimon from trash to the TOP of the deck, then optionally
//!   plays a ≤2000-DP red Digimon from trash if the controller has no Digimon.
//!   IMPLEMENTED in pure DSL.
//! - Clause C (Security): on_security with select_trash (red Digimon ≤2000 DP),
//!   play_from_trash_free, add_this_option_to_hand. IMPLEMENTED in pure DSL —
//!   `dp_lte: 2000` is now enforced (G-PRED-DP-LTE resolved 2026-05-17).
//!
//! # Resolved gaps (no longer blockers)
//! - **G-DELAY-START-OF-TURN** — RESOLVED 2026-05-02. `DelayTrigger::Start-
//!   OfYourNextTurn` exists; the DSL `kind: delay` lowerer maps
//!   `trigger: start_of_your_turn` to it.
//! - **G-PRED-DP-LTE** — RESOLVED 2026-05-17. `dp_lte` is evaluated against
//!   `CardData.dp` for trash card subjects in `select_*` filters. Clause C's
//!   `dp_lte: 2000` filter is enforced at selection time.
//! - **G-ADD-OPTION-SELF-TO-HAND** — RESOLVED. `add_this_option_to_hand`
//!   lowers to `EffectContext::add_pending_security_to_hand`; Clause C uses it.
//! - **G-ZONE-SELECTED-TRASH-TO-DECK-TOP** — RESOLVED 2026-05-21.
//!   `EffectContext::return_trash_cards_to_deck_top` plus the `destination`
//!   parameter on the trash-return DSL step move a selected trash card to the
//!   deck top. Clause B's mandatory return-to-deck-top step now ships in pure
//!   DSL — no raw_rust placeholder.

#![allow(unused_imports, dead_code)]

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledScope, CompiledTiming,
};
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

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

/// A red Level-4 Digimon usable as a digivolve target for Clause A. Carries
/// an `EvoCost` (from a Level-3 red, 4 memory) so the effect-initiated
/// digivolve actually pays a reduced memory cost (4 − 3 = 1).
fn make_red_evo_target(id: &str) -> CardData {
    let mut c = make_red_digimon(id, 4, 5000, 5);
    c.evo_costs = vec![EvoCost {
        card_color: CardColor::Red as u8,
        level: 3,
        memory_cost: 4,
    }];
    c
}

fn make_filler(id: &str) -> CardData {
    make_test_card(id, id)
}

/// Push a registered card into a player's trash zone. Trash fixtures are seeded
/// post-`start()` — the DebugRunner builder has no `trash(...)` helper.
fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let game = runner.game_mut();
    let data_idx = game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {card_id}"));
    let next = game.next_card_index();
    game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, next));
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

/// LM-027 has exactly 3 clauses:
///   0: main_from_hand (triggered)
///   1: kind:delay (declarative — [Start of Your Turn] <Delay>)
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

/// Clause 1 is a declarative `kind: delay` clause — the [Start of Your Turn]
/// <Delay> body. (Formerly a raw_rust no-op while G-ZONE-SELECTED-TRASH-TO-
/// DECK-TOP was open; now a real Delay clause.)
#[test]
fn lm_027_has_delay_clause() {
    let runner = lm_027_runner();
    let compiled = runner
        .compiled_card("LM-027")
        .expect("LM-027 must be in compiled_cards");

    let delay_trigger = compiled.effects.iter().find_map(|c| match c {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { trigger, .. }) => {
            Some(*trigger)
        }
        _ => None,
    });

    assert!(
        delay_trigger.is_some(),
        "LM-027 must have a declarative kind:delay clause for the [Start of Your Turn] <Delay>"
    );
    assert_eq!(
        delay_trigger.unwrap(),
        CompiledTiming::StartOfYourTurn,
        "the Delay clause must trigger at the start of the controller's turn"
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

/// Integrated Clause A: drive the full [Main] effect through the real
/// activate-hand-main path — choose the red field Digimon, choose the red
/// hand card, finish the effect-initiated digivolve with cost reduced by 3.
///
/// The evo target carries `EvoCost { memory_cost: 4 }`; reduced by 3 the
/// digivolve should pay exactly 1 memory. Only red cards are offered at both
/// the field and hand selection steps (a blue field Digimon and a blue hand
/// Digimon must be excluded by the `color_is: red` filters).
#[test]
fn lm_027_main_digivolve_pays_evo_cost_reduced_by_three() {
    let red_base = make_red_digimon("LM027-RED-BASE", 3, 2000, 3);
    let blue_base = {
        let mut c = make_red_digimon("LM027-BLUE-BASE", 3, 2000, 3);
        c.colors = vec![CardColor::Blue];
        c
    };
    let red_evo = make_red_evo_target("LM027-RED-EVO");
    let blue_hand = {
        let mut c = make_red_digimon("LM027-BLUE-HAND", 4, 5000, 5);
        c.colors = vec![CardColor::Blue];
        c
    };

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(red_base)
        .add_card(blue_base)
        .add_card(red_evo)
        .add_card(blue_hand)
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-027", "LM027-RED-EVO", "LM027-BLUE-HAND"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let base = runner.place_on_field(0, "LM027-RED-BASE", Some(0));
    runner.place_on_field(0, "LM027-BLUE-BASE", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));

    // Step 1: choose the red field Digimon (the blue one is filtered out).
    let field_view = runner
        .pending_selection_view()
        .expect("Main must ask which red Digimon digivolves");
    assert_eq!(field_view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "field selection must expose PASS for the printed 'may'"
    );
    assert_eq!(
        field_view.valid_action_ids.len(),
        1,
        "only the red field Digimon should be selectable"
    );
    let mask = build_action_mask(&runner.game, field_view.selecting_player);
    assert_eq!(mask[field_view.valid_action_ids[0] as usize], 1.0);
    runner
        .execute_action(field_view.selecting_player, field_view.valid_action_ids[0])
        .expect("select red field Digimon");

    // Step 2: choose the red hand card (the blue hand Digimon is filtered out).
    let hand_view = runner
        .pending_selection_view()
        .expect("Main must ask which red hand card to digivolve into");
    assert_eq!(hand_view.kind, SelectionKind::Hand);
    assert_eq!(
        hand_view.valid_action_ids.len(),
        1,
        "only the red hand Digimon should be selectable"
    );
    runner
        .execute_action(hand_view.selecting_player, hand_view.valid_action_ids[0])
        .expect("select red hand Digimon");
    runner.auto_resolve().expect("finish digivolve");

    // EvoCost 4 reduced by 3 → 1 memory paid (started at 10).
    assert_eq!(
        runner.memory(),
        9,
        "evo cost 4 reduced by 3 should pay exactly 1 memory"
    );
    // The selected evo card left hand and is now the top card of the stack.
    assert!(
        runner
            .game
            .player(0)
            .hand
            .iter()
            .all(|c| c.card_id(&runner.game.card_data) != "LM027-RED-EVO"),
        "selected red evo card must leave hand"
    );
    let evolved = &runner.game.player(0).battle_area[base.index as usize];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "LM027-RED-EVO",
        "the red base digivolved into the red evo card"
    );
}

/// Negative Clause A: the printed "may" lets the controller decline. Passing
/// the optional field selection leaves the field and hand untouched (no
/// digivolve, no memory spent).
#[test]
fn lm_027_main_decline_leaves_field_and_hand_unchanged() {
    let red_base = make_red_digimon("LM027-DECL-BASE", 3, 2000, 3);
    let red_evo = make_red_evo_target("LM027-DECL-EVO");

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(red_base)
        .add_card(red_evo)
        .add_card(make_filler("FILL"))
        .memory(10)
        .hand(0, &["LM-027", "LM027-DECL-EVO"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    runner.place_on_field(0, "LM027-DECL-BASE", Some(0));
    let hand_before = runner.hand_size(0);
    let stack_before = runner.game.player(0).battle_area[0].card_sources.len();
    let memory_before = runner.memory();

    assert!(runner.game.activate_hand_main(0, 0));
    assert!(
        runner.pending_is_optional(),
        "field selection must be declinable for the printed 'may'"
    );
    runner
        .execute_action(0, digimon_engine::action::space::PASS)
        .expect("decline the optional Main digivolve");
    runner
        .auto_resolve()
        .expect("effect completes after decline");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "declining must not move any hand card"
    );
    assert_eq!(
        runner.game.player(0).battle_area[0].card_sources.len(),
        stack_before,
        "declining must not digivolve the field Digimon"
    );
    assert_eq!(
        runner.memory(),
        memory_before,
        "declining must not spend memory"
    );
}

// ═══════════════════════════════════════════════════════════════════════════════
// Section 3 — Clause B: [Start of Your Turn] <Delay>
// ═══════════════════════════════════════════════════════════════════════════════
//
// G-ZONE-SELECTED-TRASH-TO-DECK-TOP resolved 2026-05-21 — Clause B now ships as
// a real `kind: delay` clause. The body is fired directly via
// `enqueue_triggered(DelayEffect, Permanent)` — the same dispatch the engine's
// `resolve_delayed_options_matching` uses — so these tests exercise the
// `active_when` gate (opponent has a Digimon) and the body steps (return a
// selected red Digimon to deck top; conditionally play from trash) without
// driving the full start-of-turn delayed-option lifecycle.

/// `active_when` positive: with an opponent Digimon on the field the Delay
/// body's condition passes — firing it installs the mandatory return-to-deck-
/// top trash selection.
#[test]
fn lm_027_delay_fires_at_start_of_your_turn_when_opponent_has_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(make_small_red_digimon("LM027-DLY-RED"))
        .add_card(make_red_digimon("LM027-OPP-DIGI", 3, 3000, 3))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();

    // Opponent (P1) has a Digimon → the Delay `active_when` passes.
    runner.place_on_field(1, "LM027-OPP-DIGI", Some(0));
    // LM-027 parked on P0's field as the Delay carrier.
    let lm027 = runner.place_on_field(0, "LM-027", Some(0));
    // A red Digimon in P0's trash is the mandatory return target.
    push_to_trash(&mut runner, 0, "LM027-DLY-RED");

    runner
        .game
        .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(lm027));
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_some(),
        "with an opponent Digimon the Delay body fires — the mandatory \
         return-to-deck-top trash selection must install"
    );
}

/// `active_when` negative: with NO opponent Digimon the Delay body's condition
/// fails — nothing fires and the red Digimon stays in trash.
#[test]
fn lm_027_delay_does_not_fire_when_opponent_has_no_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(make_small_red_digimon("LM027-DLY-RED2"))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();

    // No opponent Digimon on the field → the Delay `active_when` fails.
    let lm027 = runner.place_on_field(0, "LM-027", Some(0));
    push_to_trash(&mut runner, 0, "LM027-DLY-RED2");
    let trash_before = runner.trash_size(0);

    runner
        .game
        .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(lm027));
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "with no opponent Digimon the Delay body must not fire — no selection installs"
    );
    assert_eq!(
        runner.trash_size(0),
        trash_before,
        "the red Digimon stays in trash when the Delay body does not fire"
    );
}

/// Body step 1: the selected red Digimon moves from trash to the TOP of the
/// deck — the next card drawn (G-ZONE-SELECTED-TRASH-TO-DECK-TOP). P0 keeps a
/// Digimon on the field, so the optional play-from-trash sub-clause is skipped.
#[test]
fn lm_027_delay_body_returns_red_digimon_to_top_of_deck() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(make_small_red_digimon("LM027-RET-RED"))
        .add_card(make_red_digimon("LM027-OPP-D3", 3, 3000, 3))
        .add_card(make_red_digimon("LM027-OWN-D3", 3, 3000, 3))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();

    runner.place_on_field(1, "LM027-OPP-D3", Some(0));
    // P0 keeps a Digimon on the field → the conditional play sub-clause (which
    // fires only "if you don't have a Digimon") is skipped.
    runner.place_on_field(0, "LM027-OWN-D3", Some(0));
    let lm027 = runner.place_on_field(0, "LM-027", Some(1));
    push_to_trash(&mut runner, 0, "LM027-RET-RED");

    runner
        .game
        .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(lm027));
    runner.game.drain_effect_queue();

    // The mandatory return select_trash installs — pick the red Digimon.
    let view = runner
        .pending_selection_view()
        .expect("mandatory return-to-deck trash selection installs");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("select the red Digimon to return");

    // P0 has a field Digimon → no optional play sub-clause → body completes.
    assert!(
        runner.game.pending_selection.is_none(),
        "with a field Digimon, no optional play-from-trash prompt installs"
    );
    // The returned card is on TOP of P0's deck (the next card drawn).
    let deck = &runner.game.player(0).deck;
    assert_eq!(
        deck[deck.len() - 1].card_id(&runner.game.card_data),
        "LM027-RET-RED",
        "the selected red Digimon is returned to the TOP of the deck"
    );
    // ...and it has left the trash zone.
    assert!(
        !runner
            .game
            .player(0)
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM027-RET-RED"),
        "the returned card leaves the trash zone"
    );
}

/// Body step 2: "Then, if you don't have a Digimon, you may play 1 red Digimon
/// card with 2000 DP or less from your trash without paying the cost." The
/// optional play-from-trash prompt installs only when the controller has no
/// Digimon on the field (the negative branch is covered by
/// `lm_027_delay_body_returns_red_digimon_to_top_of_deck`, which has a field
/// Digimon and asserts no play prompt).
#[test]
fn lm_027_delay_body_play_from_trash_only_when_no_field_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(make_small_red_digimon("LM027-PFT-A"))
        .add_card(make_small_red_digimon("LM027-PFT-B"))
        .add_card(make_red_digimon("LM027-OPP-D4", 3, 3000, 3))
        .add_card(make_filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .start();

    runner.place_on_field(1, "LM027-OPP-D4", Some(0));
    // P0 has NO Digimon on the field → the conditional play sub-clause fires.
    let lm027 = runner.place_on_field(0, "LM-027", Some(0));
    // Two red Digimon in trash: one is returned to deck, the other remains as
    // the play-from-trash candidate.
    push_to_trash(&mut runner, 0, "LM027-PFT-A");
    push_to_trash(&mut runner, 0, "LM027-PFT-B");

    runner
        .game
        .enqueue_triggered(EffectTiming::DelayEffect, TriggerSource::Permanent(lm027));
    runner.game.drain_effect_queue();

    // Step 1: mandatory return-to-deck-top selection — return one red Digimon.
    let ret_view = runner
        .pending_selection_view()
        .expect("mandatory return-to-deck trash selection installs");
    runner
        .execute_action(ret_view.selecting_player, ret_view.valid_action_ids[0])
        .expect("return one red Digimon to deck top");

    // Step 2: P0 has no field Digimon → the optional play-from-trash prompt
    // installs for the remaining ≤2000-DP red Digimon.
    let play_view = runner
        .pending_selection_view()
        .expect("optional play-from-trash prompt installs when P0 has no Digimon");
    assert!(
        runner.pending_is_optional(),
        "the play-from-trash sub-clause is printed 'you may' — PASS must be legal"
    );
    assert_eq!(
        play_view.valid_action_ids.len(),
        1,
        "exactly the one remaining ≤2000-DP red Digimon in trash is a play candidate"
    );
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

/// Negative DP-filter test: when trash contains ONLY a large red Digimon
/// (>2000 DP), the Security clause's `dp_lte: 2000` filter rules it out, so
/// the optional select_trash has no eligible candidate and installs no
/// pending selection.
///
/// G-PRED-DP-LTE was resolved 2026-05-17 (qa/resolved-gaps.md): `dp_lte`
/// now evaluates against `CardData.dp` for trash card subjects. The Security
/// clause filter authors `dp_lte: 2000`, so this is now an active assertion.
#[test]
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
        "LM-027 Security must not offer >2000 DP Digimon as targets (dp_lte: 2000)"
    );
}

/// Positive DP-filter test: when trash holds BOTH a small (≤2000 DP) and a
/// large (>2000 DP) red Digimon, only the small one is a legal play target.
/// Drives a security attack to the end and asserts the played card is the
/// small one — the large red Digimon stays in trash.
#[test]
fn lm_027_security_offers_only_small_red_digimon_when_trash_mixed() {
    let small = make_small_red_digimon("LM027-MIX-SMALL");
    let large = make_large_red_digimon("LM027-MIX-LARGE");
    let mut attacker = make_filler("LM027-MIX-ATK");
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(small)
        .add_card(large)
        .add_card(attacker)
        .memory(10)
        .deck(1, &["LM027-MIX-SMALL", "LM027-MIX-LARGE"])
        .security(1, &["LM-027"])
        .start();

    // Seed defender trash with both Digimon (small + large red).
    for _ in 0..2 {
        if let Some(cs) = runner.game.players[1].deck.pop() {
            runner.game.players[1].trash.push(cs);
        }
    }
    assert_eq!(runner.trash_size(1), 2, "precondition: 2 cards in trash");

    let attacker = runner.place_on_field(0, "LM027-MIX-ATK", Some(0));
    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    // The large red Digimon is filtered out by dp_lte: 2000, so only the
    // small one could have been played. Exactly one Digimon entered the
    // defender's battle area, and it is the small one.
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "exactly one red Digimon played from trash"
    );
    let played_id = runner.game.players[1].battle_area[0].card_sources[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        played_id, "LM027-MIX-SMALL",
        "only the <=2000-DP red Digimon is a legal target (dp_lte: 2000)"
    );
    // The large red Digimon never left trash.
    assert!(
        runner.game.players[1]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "LM027-MIX-LARGE"),
        "the >2000-DP red Digimon stays in trash (filtered out)"
    );
}

/// After the Security clause resolves and a small red Digimon is played from
/// trash, the card moves from trash to battle area (field count may increase).
///
/// The dedicated add-self-to-hand assertion below checks the final hand move;
/// this test stays focused on the play-from-trash + no-panic outcome.
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
/// text says "Then, add this card to the hand." This must move the currently
/// resolving security Option out of `pending_security`, not a card from trash
/// or the battle area.
#[test]
fn lm_027_security_adds_card_to_hand_after_play() {
    let small = make_small_red_digimon("LM027-SMALL-HAND");
    let mut attacker = make_filler("LM027-ATTACKER");
    attacker.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(LM_027_YAML)
        .expect("LM-027 YAML parses")
        .add_card(small)
        .add_card(attacker)
        .memory(10)
        .deck(1, &["LM027-SMALL-HAND"])
        .security(1, &["LM-027"])
        .start();

    let trash_seed = runner.game.players[1]
        .deck
        .pop()
        .expect("small red seed in deck");
    runner.game.players[1].trash.push(trash_seed);

    let attacker = runner.place_on_field(0, "LM027-ATTACKER", Some(0));
    assert_eq!(runner.hand_size(1), 0, "precondition: defender hand empty");

    let _ = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security selections resolve");

    assert_eq!(runner.security_count(1), 0, "LM-027 left security");
    assert_eq!(runner.hand_size(1), 1, "LM-027 moved to defender hand");
    assert_eq!(
        runner.trash_size(1),
        0,
        "LM-027 was not trashed and the selected Digimon left trash"
    );
    assert_eq!(
        runner.battle_area_size(1),
        1,
        "selected small red Digimon was played from trash"
    );
    let played_id = runner.game.players[1].battle_area[0].card_sources[0]
        .card_id(&runner.game.card_data)
        .to_string();
    assert_eq!(
        played_id, "LM027-SMALL-HAND",
        "the specifically seeded trash Digimon was played"
    );
}
