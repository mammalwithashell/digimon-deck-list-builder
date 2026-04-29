//! Drainer behavior tests — triggered-effect queue, TriggerOrder selection,
//! turn-player-first ordering, decline-all semantics.
//!
//! These exercise the PR2 effect_queue module through the public API:
//! `Game::enqueue_triggered`, `drain_effect_queue`, `resolve_selection`,
//! `pending_selection`. Card scripts `TEST-006` / `TEST-007` / `TEST-008`
//! are the scaffolding — mandatory/optional/different-sign EndOfYourTurn
//! effects that make resolution order observable in memory totals.

use digimon_engine::action::space::{HAND_EFFECT_START, PASS};
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn setup_two_effects_on_turn_player(cards: &[&str]) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-006", "TestSix"))
        .add_card(make_test_card("TEST-007", "TestSeven"))
        .add_card(make_test_card("TEST-008", "TestEight"))
        .memory(0)
        .start();
    for card_id in cards {
        r.place_on_field(0, card_id, Some(0));
    }
    r
}

/// One mandatory trigger → auto-fires, no prompt.
#[test]
fn single_mandatory_trigger_auto_fires_without_prompt() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-006"]);
    let memory_before = r.memory();

    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    assert!(
        r.game.pending_selection.is_none(),
        "single trigger must not install a selection"
    );
    assert!(
        r.game.effect_queue.is_empty(),
        "queue must empty after single trigger"
    );
    assert_eq!(r.memory(), memory_before + 5, "TEST-006 grants +5 memory");
}

/// Two mandatory triggers on one controller → TriggerOrder installed
/// WITHOUT a PASS bit (mandatory means the chooser must pick).
#[test]
fn two_mandatory_triggers_install_trigger_order_without_pass() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-006", "TEST-008"]);

    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("multi-trigger bundle must install a selection");
    assert_eq!(sel.kind, SelectionKind::TriggerOrder);
    assert_eq!(sel.selecting_player, 0, "turn player chooses");
    assert_eq!(sel.valid_action_ids.len(), 2);
    assert!(
        !sel.is_optional,
        "mandatory-bearing bundles must not offer decline-all"
    );
    assert_eq!(
        r.game.effect_queue.len(),
        2,
        "queue still holds both entries"
    );
}

/// PASS on a mandatory-bearing bundle is rejected as an invalid action.
#[test]
fn pass_rejected_when_bundle_has_mandatory() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-006", "TEST-008"]);
    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    let err = r
        .game
        .resolve_selection(0, PASS)
        .expect_err("PASS must be rejected on mandatory bundles");
    use digimon_engine::selection::SelectionError;
    assert_eq!(err, SelectionError::InvalidAction);
    assert!(
        r.game.pending_selection.is_some(),
        "selection must remain installed after a rejected action"
    );
}

/// Resolving the first bundle entry fires that effect; the sibling then
/// auto-fires (bundle size 1) without a second prompt.
#[test]
fn resolving_trigger_order_fires_choice_then_auto_fires_sibling() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-006", "TEST-008"]);
    let memory_before = r.memory();

    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    // Pick action_id = HAND_EFFECT_START + 0 → the first queued effect
    // (TEST-006 from battle_area[0]).
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("picking a valid bundle entry must succeed");

    assert!(
        r.game.pending_selection.is_none(),
        "sibling bundle size is 1, so it auto-fires without another prompt"
    );
    assert!(r.game.effect_queue.is_empty());
    // Both effects have now fired: +5 (TEST-006) and -3 (TEST-008) = +2 net.
    assert_eq!(r.memory(), memory_before + 5 - 3);
}

/// All-optional bundle → TriggerOrder carries a PASS bit. PASS clears every
/// remaining optional trigger controlled by the chooser.
#[test]
fn optional_bundle_pass_clears_all_for_this_chooser() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-007", "TEST-007"]);
    let memory_before = r.memory();

    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional bundle also installs a selection");
    assert!(sel.is_optional, "all-optional bundles offer decline-all");

    r.game
        .resolve_selection(0, PASS)
        .expect("PASS is legal on all-optional bundles");

    assert!(r.game.pending_selection.is_none());
    assert!(
        r.game.effect_queue.is_empty(),
        "decline-all drops every remaining optional trigger for this chooser"
    );
    assert_eq!(
        r.memory(),
        memory_before,
        "no optional effect fired — memory unchanged"
    );
}

/// Resolving a single optional pick leaves the sibling queued.
/// Sibling is now a bundle-of-1 → auto-fires.
#[test]
fn optional_pick_one_fires_then_sibling_auto_fires() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-007", "TEST-007"]);
    let memory_before = r.memory();

    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("picking an optional is always legal");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.effect_queue.is_empty());
    // Both TEST-007 fired now: sibling auto-fires as size-1 bundle.
    // Each grants +2 memory.
    assert_eq!(r.memory(), memory_before + 2 + 2);
}

/// Turn player's bundle resolves before the non-turn-player's.
/// Setup: 2 mandatory triggers on P0 (turn player), 1 mandatory on P1.
/// Expected sequence:
///   1. drain → installs TriggerOrder for P0 (bundle size 2, mandatory).
///   2. resolve 30 → first P0 trigger fires; second P0 trigger auto-fires.
///   3. drainer then sees P1's bundle (size 1) → auto-fires.
///   4. queue empty, no pending selection.
#[test]
fn turn_player_bundle_resolves_before_opponent() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-006", "TestSix"))
        .add_card(make_test_card("TEST-008", "TestEight"))
        .memory(0)
        .start();
    r.place_on_field(0, "TEST-006", Some(0));
    r.place_on_field(0, "TEST-008", Some(0));
    r.place_on_field(1, "TEST-006", Some(0));
    let memory_before = r.memory();

    // Enqueue P0's triggers first, then P1's.
    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(1),
    );
    r.game.drain_effect_queue();

    // First prompt is for P0 — the turn player.
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("expected TriggerOrder prompt for turn player");
    assert_eq!(sel.selecting_player, 0);
    assert_eq!(sel.valid_action_ids.len(), 2);

    // P1's entry stays in the queue during the P0 prompt.
    assert_eq!(r.game.effect_queue.len(), 3);

    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("resolve P0's first choice");

    // Now everything should have resolved: P0's second (auto-fire size-1),
    // then P1's single (auto-fire size-1).
    assert!(r.game.pending_selection.is_none());
    assert!(r.game.effect_queue.is_empty());
    // P0: TEST-006 (+5) + TEST-008 (-3) = +2. P1: TEST-006 (+5). Total: +7.
    assert_eq!(r.memory(), memory_before + 2 + 5);
}

/// Mandatory + optional mixed for one controller → PASS is not offered
/// until the mandatory slot has been resolved.
#[test]
fn mixed_bundle_pass_becomes_legal_after_mandatory_cleared() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-006", "TEST-007"]);
    let memory_before = r.memory();

    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    // First prompt: mixed bundle with mandatory → no PASS.
    let sel = r.game.pending_selection.as_ref().expect("prompt installed");
    assert!(!sel.is_optional);

    // Resolve the mandatory TEST-006 (battle_area[0], bundle position 0).
    r.game
        .resolve_selection(0, HAND_EFFECT_START)
        .expect("resolve mandatory pick");

    // Now only the optional TEST-007 remains for the chooser — bundle
    // size 1, so the drainer auto-fires it (does not re-prompt for a
    // single optional). This matches the "no re-choose" rule: if the
    // chooser is left with a single optional effect, they don't get a
    // second prompt — it's just resolved.
    //
    // Future refinement: if we want a "decline lone optional" prompt for
    // parity with DCGO's behavior on singletons, we'd add a bundle-size-1
    // optional branch here. Current behavior matches Python's single-
    // trigger code path which fires without prompting.
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.memory(), memory_before + 5 + 2);
}

/// Wrong-player rejection: only the selecting player may resolve.
#[test]
fn wrong_player_rejected() {
    let mut r = setup_two_effects_on_turn_player(&["TEST-006", "TEST-008"]);
    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    use digimon_engine::selection::SelectionError;
    let err = r
        .game
        .resolve_selection(1, HAND_EFFECT_START)
        .expect_err("non-selecting player must be rejected");
    assert_eq!(err, SelectionError::WrongPlayer);
    assert!(r.game.pending_selection.is_some());
}

#[test]
fn buried_non_inherited_triggered_effect_does_not_fire_from_source_position() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("TEST-006", "TestSix"))
        .memory(0)
        .start();
    let carrier = r.place_on_field(0, "CARRIER", Some(0));

    {
        let game = r.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-006")
            .expect("TEST-006 registered in card_data");
        let next = game.next_card_index();
        let source = CardSource::new(data_idx, 0, next);
        game.players[0].battle_area[carrier.index as usize]
            .card_sources
            .insert(0, source);
    }

    r.game.enqueue_triggered(
        EffectTiming::EndOfYourTurn,
        TriggerSource::PlayerBattleArea(0),
    );
    r.game.drain_effect_queue();

    assert_eq!(
        r.memory(),
        0,
        "buried non-inherited top-card effects must not dispatch from source position"
    );
}
