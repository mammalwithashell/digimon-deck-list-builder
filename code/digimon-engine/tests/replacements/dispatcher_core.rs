//! Phase 7 Task 2 — `try_replace` dispatcher core tests.
//!
//! These tests drive `Game::try_replace` directly (no fire-site wiring yet
//! — Task 3+ handles that) to exercise:
//! - Empty-candidate case returns `None`.
//! - Mandatory candidate composition (cancel, redirect, substitute, handled).
//! - Optional candidate installs a `PendingSelection::Replacement`.
//! - Accept / decline resolution paths for optional replacements.
//! - Depth-guard safety rail.
//! - Controller-first / opponent-second layering.
//!
//! The dispatcher is exposed as `#[doc(hidden)] pub fn try_replace` for
//! exactly this purpose — fire-sites in Task 3+ will call it via normal
//! crate-local dispatch.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, GamePhase, Zone};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};
use digimon_engine::selection::SelectionKind;

fn card(id: &str) -> digimon_engine::CardData {
    make_test_card(id, id)
}

/// Test 1: No candidates in scope → outcome is `None` and depth is unchanged.
#[test]
fn try_replace_returns_none_when_no_candidates() {
    let mut r = DebugRunner::builder()
        .add_card(card("FILLER"))
        .hand(0, &[])
        .start();

    // Place a filler permanent (no would-timed effects) so we have a
    // subject. Its effects don't include any WhenWouldBeDeleted replacement.
    let handle = r.place_on_field(0, "FILLER", Some(0));

    assert_eq!(r.game.replacement_depth, 0);
    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(outcome, ReplacementOutcome::None);
    assert_eq!(r.game.replacement_depth, 0);
    assert!(r.game.pending_selection.is_none());
}

/// Test 2: Mandatory cancel replacement → outcome is `Cancelled`.
#[test]
fn mandatory_cancel_replacement_applies() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-CANCEL"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-CANCEL", Some(0));

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(outcome, ReplacementOutcome::Cancelled);
    assert_eq!(r.game.replacement_depth, 0);
    assert!(r.game.pending_selection.is_none());
}

/// Test 3: Mandatory redirect replacement → outcome is `Redirected(Deck)`.
#[test]
fn redirect_replacement_applies() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-REDIRECT"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-REDIRECT", Some(0));

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(outcome, ReplacementOutcome::Redirected(Zone::Deck));
}

/// Test 4: Mandatory substitute replacement → outcome is `Substituted(other)`.
#[test]
fn substitute_replacement_applies() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-SUBSTITUTE"))
        .add_card(card("FILLER"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-SUBSTITUTE", Some(0));
    let other = r.place_on_field(0, "FILLER", Some(0));
    assert_eq!(
        other,
        PermanentHandle {
            player: 0,
            index: 1
        }
    );

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(
        outcome,
        ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other))
    );
}

/// Test 5: Custom-handled replacement applies its side effects AND reports
/// `CustomHandled`. The side effect trashes the top of the owner's deck.
#[test]
fn custom_handled_replacement_applies() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-HANDLED"))
        .add_card(card("FILLER"))
        .deck(0, &["FILLER", "FILLER", "FILLER"])
        .start();
    let handle = r.place_on_field(0, "TEST-P7-HANDLED", Some(0));
    assert_eq!(r.game.player(0).deck.len(), 3);

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(outcome, ReplacementOutcome::CustomHandled);
    assert_eq!(
        r.game.player(0).deck.len(),
        2,
        "top of deck should be trashed"
    );
    assert_eq!(r.game.player(0).trash.len(), 1);
}

/// Test 6: Optional replacement installs a `PendingSelection::Replacement`.
#[test]
fn optional_replacement_emits_pending_selection() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-OPTIONAL"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-OPTIONAL", Some(0));
    let pre_phase = r.game.current_phase;

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    // Selection pending → dispatcher returns None (outcome will be read
    // from game.replacement_pending_outcome once the selection resolves).
    assert_eq!(outcome, ReplacementOutcome::None);
    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("pending_selection should be installed for optional replacement");
    assert_eq!(sel.kind, SelectionKind::Replacement);
    assert!(sel.valid_action_ids.contains(&REPLACEMENT_ACCEPT));
    assert!(sel.is_optional, "replacement prompts always admit PASS");
    assert_eq!(sel.previous_phase, pre_phase);
    assert_eq!(sel.selecting_player, 0, "subject controller is P0");
    assert_eq!(
        r.game.current_phase,
        GamePhase::EffectChoice,
        "installer reuses EffectChoice phase to satisfy is_selection_phase"
    );

    // Mask includes both REPLACEMENT_ACCEPT and PASS.
    let mask = digimon_engine::action::build_action_mask(&r.game, 0);
    assert_eq!(mask[REPLACEMENT_ACCEPT as usize], 1.0);
    assert_eq!(mask[PASS as usize], 1.0);
}

/// Test 7: Accept path — `resolve_selection` with `REPLACEMENT_ACCEPT` fires
/// the replacement process and writes the outcome into
/// `game.replacement_pending_outcome`.
#[test]
fn optional_replacement_accept_path_applies_outcome() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-OPTIONAL"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-OPTIONAL", Some(0));
    let _ = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("resolve_selection should accept REPLACEMENT_ACCEPT");
    assert_eq!(
        r.game.replacement_pending_outcome,
        Some(ReplacementOutcome::Cancelled),
    );
    assert!(r.game.pending_selection.is_none());
}

/// Test 8: Decline path — `resolve_selection` with `PASS` leaves the outcome
/// slot at `None`.
#[test]
fn optional_replacement_decline_path_leaves_outcome_none() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-OPTIONAL"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-OPTIONAL", Some(0));
    let _ = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );

    r.game
        .resolve_selection(0, PASS)
        .expect("resolve_selection should accept PASS on optional prompt");
    assert_eq!(r.game.replacement_pending_outcome, None);
    assert!(r.game.pending_selection.is_none());
}

/// Test 9: Depth guard caps recursion. `TEST-P7-RECURSE` re-enters
/// `try_replace` from inside its own process; the guard kicks in at
/// MAX_REPLACEMENT_DEPTH and the chain unwinds cleanly.
#[test]
fn depth_guard_caps_recursion() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-RECURSE"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-RECURSE", Some(0));

    // Outer call enters; its process re-invokes try_replace ~8 times before
    // the guard trips. Final outcome should still be Cancelled (each level
    // calls cancel() after the recursive call returns).
    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(outcome, ReplacementOutcome::Cancelled);
    assert_eq!(
        r.game.replacement_depth, 0,
        "depth must return to 0 after the outer call unwinds"
    );
}

/// Test 10: Controller-owned candidates fire before opponent-owned ones.
///
/// We set up two cards whose processes set *different* outcomes, and the
/// v1 dispatcher composes mandatory candidates with "last non-None wins".
/// Therefore the final outcome tells us which candidate ran *last* — the
/// opponent's. We verify that by placing:
///   - P0's TEST-P7-CANCEL (sets Cancelled)   — subject's controller
///   - P1's TEST-P7-CANCEL2 (sets CustomHandled) — opponent
/// Subject is P0's permanent → layering puts P0's card first, P1's last,
/// so the final outcome is CustomHandled.
#[test]
fn controller_owned_candidates_fire_before_opponent_owned() {
    let mut r = DebugRunner::builder()
        .add_card(card("TEST-P7-CANCEL"))
        .add_card(card("TEST-P7-CANCEL2"))
        .start();
    let handle = r.place_on_field(0, "TEST-P7-CANCEL", Some(0));
    let _opp = r.place_on_field(1, "TEST-P7-CANCEL2", Some(0));

    let outcome = r.game.try_replace(
        EffectTiming::WhenWouldBeDeleted,
        ReplacementSubject::Permanent(handle),
        ReplacementCause::OwnEffect,
        None,
    );
    assert_eq!(
        outcome,
        ReplacementOutcome::CustomHandled,
        "opponent's card ran after P0's, overwriting the outcome"
    );
}
