//! BT8-094 Digimon Emperor — Tamer, Black, Cost 3.
//!
//! # Card text (cards.json — authoritative)
//!
//! - [All Turns] When one of your opponent's level 5 or lower Digimon is
//!   deleted, you may suspend this Tamer to ＜Draw 1＞.
//! - [Opponent's Turn] When one of your opponent's level 3 Digimon is moved
//!   from their breeding area to their battle area, gain 2 memory.
//! - Security Effect [Security] Play this card without paying the cost.
//!
//! # Pass history
//!
//! - Pass 1 (initial): Security clause authored; deletion + breeding-move
//!   clauses deferred pending `event-target level predicates` and `OnMove`.
//! - Pass 2 (this file): `on_move` substrate resolved (G-ON-MOVE 2026-04-29);
//!   both clauses blocked by `G-EVENT-TARGET-LEVEL-LTE` — no
//!   `event_target_level_lte` / `event_target_level_eq` predicate in the DSL
//!   (`code/digimon-dsl/src/predicate.rs` has no such field). Without the
//!   level filter the clause would fire on ALL opponent Digimon
//!   deletions/moves, violating the no-approximations policy. Both clauses
//!   remain OMITTED in the YAML; all behavioral tests are #[ignore]'d.
//!
//! # Patterns this test covers
//!
//! - [Security] play-self-free from the security stack.
//! - B3 Trigger-on-event tamer: on_any_deletion observer with optional
//!   activation_cost suspend-self (gap-blocked).
//! - B3 Trigger-on-event tamer: on_move observer, [Opponent's Turn] gate,
//!   event_target_owner: opponent + level filter (gap-blocked).
//! - Gap documentation: G-EVENT-TARGET-LEVEL-LTE.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::permanent::Permanent;
use digimon_engine::replacement::ReplacementCause;

// ─── Security: play-self-free ──────────────────────────────────────────────

#[test]
fn bt8_094_has_security_play_from_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("BT8-094").expect("BT8-094 compiled");
    let security = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(triggered)
        }
        _ => None,
    });
    let security = security.expect("BT8-094 must have security clause");
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlayFromSecurity)));
}

// ─── Clause A: [All Turns] opponent's Lv.5-or-lower deletion observer ─────
//
// Printed: "When one of your opponent's level 5 or lower Digimon is deleted,
// you may suspend this Tamer to <Draw 1>."
//
// Intended YAML shape (once G-EVENT-TARGET-LEVEL-LTE closes):
//   - when: on_any_deletion
//     active_when: { all_turns: true }
//     optional: true
//     condition:
//       all_of:
//         - event_target_owner: opponent
//         - event_target_kind: digimon
//         - event_target_level_lte: 5         # G-EVENT-TARGET-LEVEL-LTE
//         - source_is_unsuspended: true
//     process:
//       - activation_cost: { suspend_self: true }
//       - draw: { of: you, count: 1 }
//
// BLOCKED: event_target_level_lte is absent from digimon-dsl PredicateSpec.
// All tests below are #[ignore]'d.

/// Structural: BT8-094 must ship an [All Turns] on_any_deletion clause once
/// the gap closes and the YAML is authored.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_lte not in DSL; Clause A omitted from YAML"]
fn bt8_094_has_all_turns_deletion_observer_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("BT8-094").expect("BT8-094 compiled");
    let observer = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnAnyDeletion) => Some(t),
        _ => None,
    });
    assert!(
        observer.is_some(),
        "BT8-094 must have an [All Turns] on_any_deletion clause"
    );
    let observer = observer.unwrap();
    assert!(
        observer.optional,
        "[All Turns] deletion clause must be optional (the suspend cost is opt-in)"
    );
    assert!(
        observer.active_when.is_some(),
        "[All Turns] gate must be encoded via active_when"
    );
    assert!(
        observer.condition.is_some(),
        "deletion observer must gate on event context (owner + kind + level)"
    );
}

/// Positive: deleting an opponent's level-5 Digimon offers a suspend-to-draw prompt.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_lte not in DSL; Clause A omitted from YAML"]
fn bt8_094_all_turns_offers_suspend_to_draw_when_opponent_lv5_or_lower_deleted() {
    let mut lv5 = make_test_card("OPP-LV5", "Opp Lv5");
    lv5.card_kind = CardKind::Digimon;
    lv5.level = Some(5);
    lv5.dp = Some(6000);
    lv5.play_cost = 0;
    let draw_filler = make_test_card("BT8094-DF-A", "Draw Filler A");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 loads")
        .add_card(lv5)
        .add_card(draw_filler)
        .deck(0, &["BT8094-DF-A"])
        .security(1, &["BT8094-DF-A"])
        .start();

    let emperor = runner.place_on_field(0, "BT8-094", Some(0));
    let opp = runner.place_on_field(1, "OPP-LV5", Some(1));
    let hand_before = runner.hand_size(0);

    runner
        .game
        .delete_permanent_with_cause(opp, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("deletion of opponent's Lv.5 must offer the suspend-to-draw choice");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("accept");
    runner.auto_resolve().expect("finish");

    assert!(
        runner.game.players[0].battle_area[emperor.index as usize].is_suspended,
        "accepting suspends this Tamer"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "accepting draws 1"
    );
}

/// Negative: opponent's level-6 Digimon deletion must NOT trigger Clause A.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_lte not in DSL; Clause A omitted from YAML"]
fn bt8_094_all_turns_does_not_fire_for_opponent_lv6_deletion() {
    let mut lv6 = make_test_card("OPP-LV6", "Opp Lv6");
    lv6.card_kind = CardKind::Digimon;
    lv6.level = Some(6);
    lv6.dp = Some(10000);
    lv6.play_cost = 0;
    let draw_filler = make_test_card("BT8094-DF-B", "Draw Filler B");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 loads")
        .add_card(lv6)
        .add_card(draw_filler)
        .deck(0, &["BT8094-DF-B"])
        .security(1, &["BT8094-DF-B"])
        .start();

    runner.place_on_field(0, "BT8-094", Some(0));
    let opp = runner.place_on_field(1, "OPP-LV6", Some(1));
    let hand_before = runner.hand_size(0);

    runner
        .game
        .delete_permanent_with_cause(opp, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "Lv.6 opponent deletion must NOT offer the suspend-to-draw choice"
    );
    assert_eq!(runner.hand_size(0), hand_before, "no draw for Lv.6 deletion");
}

/// Negative: own Digimon deletion must NOT trigger Clause A (printed: "your opponent's").
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_lte not in DSL; Clause A omitted from YAML"]
fn bt8_094_all_turns_does_not_fire_for_own_digimon_deletion() {
    let mut own_lv5 = make_test_card("OWN-LV5", "Own Lv5");
    own_lv5.card_kind = CardKind::Digimon;
    own_lv5.level = Some(5);
    own_lv5.dp = Some(6000);
    own_lv5.play_cost = 0;
    let draw_filler = make_test_card("BT8094-DF-C", "Draw Filler C");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 loads")
        .add_card(own_lv5)
        .add_card(draw_filler)
        .deck(0, &["BT8094-DF-C"])
        .security(1, &["BT8094-DF-C"])
        .start();

    runner.place_on_field(0, "BT8-094", Some(0));
    let own = runner.place_on_field(0, "OWN-LV5", Some(0));
    let hand_before = runner.hand_size(0);

    runner
        .game
        .delete_permanent_with_cause(own, ReplacementCause::OwnEffect);

    assert!(
        runner.pending_selection().is_none(),
        "own Digimon deletion must NOT trigger the opponent's-deletion observer"
    );
    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "no draw for own Digimon deletion"
    );
}

/// Decline test: player declines the suspend cost — Tamer stays unsuspended and
/// no draw occurs.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_lte not in DSL; Clause A omitted from YAML"]
fn bt8_094_all_turns_player_may_decline_suspend_cost() {
    let mut lv4 = make_test_card("OPP-LV4-A", "Opp Lv4 A");
    lv4.card_kind = CardKind::Digimon;
    lv4.level = Some(4);
    lv4.dp = Some(5000);
    lv4.play_cost = 0;
    let draw_filler = make_test_card("BT8094-DF-D", "Draw Filler D");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 loads")
        .add_card(lv4)
        .add_card(draw_filler)
        .deck(0, &["BT8094-DF-D"])
        .security(1, &["BT8094-DF-D"])
        .start();

    let emperor = runner.place_on_field(0, "BT8-094", Some(0));
    let opp = runner.place_on_field(1, "OPP-LV4-A", Some(1));
    let hand_before = runner.hand_size(0);

    runner
        .game
        .delete_permanent_with_cause(opp, ReplacementCause::OwnEffect);

    let view = runner
        .pending_selection_view()
        .expect("must offer choice");
    // Decline = last action (PASS or Decline label index).
    let decline = *view.valid_action_ids.last().expect("decline action present");
    runner
        .execute_action(view.selecting_player, decline)
        .expect("decline");
    runner.auto_resolve().expect("resolve after decline");

    assert!(
        !runner.game.players[0].battle_area[emperor.index as usize].is_suspended,
        "declining must leave this Tamer unsuspended"
    );
    assert_eq!(runner.hand_size(0), hand_before, "declining must not draw");
}

// ─── Clause B: [Opponent's Turn] breeding-move observer ───────────────────
//
// Printed: "When one of your opponent's level 3 Digimon is moved from their
// breeding area to their battle area, gain 2 memory."
//
// Intended YAML shape (once G-EVENT-TARGET-LEVEL-LTE closes):
//   - when: on_move
//     active_when: { opponents_turn: true }
//     condition:
//       all_of:
//         - event_target_owner: opponent
//         - event_target_kind: digimon
//         - event_target_level_eq: 3          # G-EVENT-TARGET-LEVEL-LTE
//     process:
//       - gain_memory: 2
//
// BLOCKED: event_target_level_eq is absent from digimon-dsl PredicateSpec.
// All tests below are #[ignore]'d.

/// Structural: BT8-094 must ship an [Opponent's Turn] on_move clause once the
/// gap closes and the YAML is authored.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_eq not in DSL; Clause B omitted from YAML"]
fn bt8_094_has_opponents_turn_on_move_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("BT8-094").expect("BT8-094 compiled");
    let observer = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnMove) => Some(t),
        _ => None,
    });
    assert!(
        observer.is_some(),
        "BT8-094 must have an [Opponent's Turn] on_move clause"
    );
    let observer = observer.unwrap();
    assert!(
        observer.active_when.is_some(),
        "[Opponent's Turn] gate must be encoded via active_when"
    );
    assert!(
        observer.condition.is_some(),
        "on_move clause must gate on event_target_owner + event_target_kind + level"
    );
}

/// Positive: opponent moves a level-3 Digimon from breeding on their turn →
/// controller gains 2 memory.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_eq not in DSL; Clause B omitted from YAML"]
fn bt8_094_opponents_turn_gains_2_memory_when_opponent_lv3_moves_from_breeding() {
    let mut opp_lv3 = make_test_card("OPP-LV3-B", "Opp Lv3 B");
    opp_lv3.card_kind = CardKind::Digimon;
    opp_lv3.level = Some(3);
    opp_lv3.dp = Some(3000);
    opp_lv3.play_cost = 0;
    let filler = make_test_card("BT8094-FILLER-B", "Filler B");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 loads")
        .add_card(opp_lv3)
        .add_card(filler)
        .deck(0, &["BT8094-FILLER-B"])
        .deck(1, &["BT8094-FILLER-B"])
        .start();

    runner.place_on_field(0, "BT8-094", Some(0));
    runner.end_turn(); // P0 ends → P1's turn

    let memory_before = runner.memory();

    // Place Lv.3 Digimon in P1's breeding area then move it.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPP-LV3-B")
        .expect("OPP-LV3-B registered");
    let card_index = runner.game.next_card_index();
    let source = CardSource::new(data_idx, 1, card_index);
    runner.game.players[1].breeding_area =
        Some(Permanent::new(source, runner.game.turn_count));
    runner.move_from_breeding(1);

    assert_eq!(
        runner.memory(),
        memory_before + 2,
        "controller gains 2 memory when opponent's Lv.3 moves from breeding"
    );
}

/// Negative: opponent moves a level-4 Digimon — no memory gain.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_eq not in DSL; Clause B omitted from YAML"]
fn bt8_094_opponents_turn_does_not_fire_for_opponent_non_lv3_from_breeding() {
    let mut opp_lv4 = make_test_card("OPP-LV4-B", "Opp Lv4 B");
    opp_lv4.card_kind = CardKind::Digimon;
    opp_lv4.level = Some(4);
    opp_lv4.dp = Some(4000);
    opp_lv4.play_cost = 0;
    let filler = make_test_card("BT8094-FILLER-C", "Filler C");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 loads")
        .add_card(opp_lv4)
        .add_card(filler)
        .deck(0, &["BT8094-FILLER-C"])
        .deck(1, &["BT8094-FILLER-C"])
        .start();

    runner.place_on_field(0, "BT8-094", Some(0));
    runner.end_turn();

    let memory_before = runner.memory();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPP-LV4-B")
        .expect("OPP-LV4-B registered");
    let card_index = runner.game.next_card_index();
    let source = CardSource::new(data_idx, 1, card_index);
    runner.game.players[1].breeding_area =
        Some(Permanent::new(source, runner.game.turn_count));
    runner.move_from_breeding(1);

    assert_eq!(
        runner.memory(),
        memory_before,
        "level 4 opponent Digimon move from breeding must NOT gain memory"
    );
}

/// Negative: own turn — [Opponent's Turn] gate blocks even if opponent moves
/// a Lv.3 from breeding.
#[test]
#[ignore = "pending: G-EVENT-TARGET-LEVEL-LTE — event_target_level_eq not in DSL; Clause B omitted from YAML"]
fn bt8_094_does_not_gain_memory_on_own_turn_when_opponent_lv3_moves() {
    let mut opp_lv3 = make_test_card("OPP-LV3-OT", "Opp Lv3 OT");
    opp_lv3.card_kind = CardKind::Digimon;
    opp_lv3.level = Some(3);
    opp_lv3.dp = Some(3000);
    opp_lv3.play_cost = 0;
    let filler = make_test_card("BT8094-FILLER-D", "Filler D");

    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 loads")
        .add_card(opp_lv3)
        .add_card(filler)
        .deck(0, &["BT8094-FILLER-D"])
        .deck(1, &["BT8094-FILLER-D"])
        .start();

    // Remain on P0's turn — [Opponent's Turn] gate must reject.
    runner.place_on_field(0, "BT8-094", Some(0));
    let memory_before = runner.memory();

    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "OPP-LV3-OT")
        .expect("OPP-LV3-OT registered");
    let card_index = runner.game.next_card_index();
    let source = CardSource::new(data_idx, 1, card_index);
    runner.game.players[1].breeding_area =
        Some(Permanent::new(source, runner.game.turn_count));
    runner.move_from_breeding(1);

    assert_eq!(
        runner.memory(),
        memory_before,
        "[Opponent's Turn] gate must block memory gain during own turn"
    );
}
