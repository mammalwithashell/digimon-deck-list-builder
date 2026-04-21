//! Phase 7 Task 6 — native-keyword auto-install replacement tests.
//!
//! When a card's `CardData::keywords` contains one of the Phase-7 replacement
//! keywords (`Barrier`, `Evade`, `Fragment(N)`, `Decode`), the engine should
//! auto-install a matching `WhenWouldBe*` replacement effect at
//! `Game::effects_for_card` so the card behaves as printed without needing a
//! hand-authored `CardEffect` script.
//!
//! Partition / ArmorPurge are explicitly **deferred** from this task —
//! see TODO(phase-7-followup) in `cards/keyword_effects.rs`.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

fn card_with_keywords(id: &str, keywords: Vec<Keyword>) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.keywords = keywords;
    c
}

/// Printed `<Barrier>` on a card's `CardData::keywords` should auto-install
/// an optional `WhenWouldBeDeleted` replacement that, on accept, trashes the
/// top of the owner's deck and cancels the deletion.
#[test]
fn printed_barrier_keyword_auto_installs_replacement() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("BARRIER_CARD", vec![Keyword::Barrier]))
        .add_card(make_test_card("FILLER", "FILLER"))
        .deck(0, &["FILLER"; 5])
        .start();
    let handle = r.place_on_field(0, "BARRIER_CARD", Some(0));

    assert_eq!(r.game.player(0).deck.len(), 5);
    assert_eq!(r.game.player(0).trash.len(), 0);
    assert_eq!(r.battle_area_size(0), 1);

    r.game.delete_permanent_with_effects(handle);

    // Barrier is optional — selection should be installed.
    assert!(
        r.game.pending_selection.is_some(),
        "optional replacement should install PendingSelection"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("resolve with REPLACEMENT_ACCEPT should succeed");

    assert_eq!(
        r.battle_area_size(0),
        1,
        "Barrier should prevent the deletion"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        4,
        "top of deck should be trashed"
    );
    assert_eq!(r.game.player(0).trash.len(), 1);
}

/// Declining an optional Barrier replacement allows the deletion to proceed.
#[test]
fn printed_barrier_keyword_decline_allows_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("BARRIER_CARD", vec![Keyword::Barrier]))
        .add_card(make_test_card("FILLER", "FILLER"))
        .deck(0, &["FILLER"; 5])
        .start();
    let handle = r.place_on_field(0, "BARRIER_CARD", Some(0));

    r.game.delete_permanent_with_effects(handle);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, PASS)
        .expect("resolve with PASS should succeed (optional)");

    assert_eq!(
        r.battle_area_size(0),
        0,
        "declined Barrier should allow deletion"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        5,
        "deck unchanged on decline"
    );
    assert_eq!(r.game.player(0).trash.len(), 1, "the permanent itself goes to trash");
}

/// Printed `<Evade>` auto-installs an optional `WhenWouldBeDeleted`
/// replacement that redirects the deletion to the owner's deck (bottom).
#[test]
fn printed_evade_keyword_redirects_to_deck_bottom() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("EVADE_CARD", vec![Keyword::Evade]))
        .start();
    let handle = r.place_on_field(0, "EVADE_CARD", Some(0));

    let deck_before = r.game.player(0).deck.len();
    r.game.delete_permanent_with_effects(handle);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Evade replacement");

    assert_eq!(
        r.battle_area_size(0),
        0,
        "Evade removes the permanent from the field"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before + 1,
        "Evade redirects to deck (grew by 1)"
    );
}

/// Printed `<Decode>` auto-installs an optional
/// `WhenWouldBeReturnedToDeck` replacement that redirects the return to the
/// owner's hand. Decode also installs a `WhenWouldBeReturnedToHand`
/// replacement (symmetric route). Pre-§7.5 guard, the deck→hand redirect
/// cascaded into a second selection for the hand-timing Decode replacement;
/// post-guard, the once-per-event guard suppresses the second prompt so the
/// redirect commits cleanly with exactly one accept.
#[test]
fn printed_decode_keyword_redirects_return_to_deck_into_hand() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("DECODE_CARD", vec![Keyword::Decode]))
        .start();
    let handle = r.place_on_field(0, "DECODE_CARD", Some(0));

    let deck_before = r.game.player(0).deck.len();
    assert_eq!(r.game.player(0).hand.len(), 0);

    // Drive a return-to-deck flow; Decode should redirect into hand.
    let ok = r
        .game
        .return_to_deck(handle, digimon_engine::enums::StackPosition::Bottom);
    assert!(!ok, "return_to_deck short-circuits while selection pending");
    assert!(
        r.game.pending_selection.is_some(),
        "optional Decode (deck) replacement should install selection"
    );
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Decode (deck) replacement");

    // Post-§7.5 guard: the `return_to_hand` commit path does NOT re-install a
    // second Decode replacement prompt — the once-per-event guard blocks a
    // same-subject refire during the commit continuation. The redirect
    // completes in a single accept.
    assert!(
        r.game.pending_selection.is_none(),
        "§7.5 guard must suppress the cascading Decode (hand) prompt during \
         the deck→hand commit continuation"
    );

    assert_eq!(
        r.battle_area_size(0),
        0,
        "Decode Digimon left the battle area via the redirect"
    );
    assert_eq!(
        r.game.player(0).hand.len(),
        1,
        "permanent ended up in hand via the deck→hand redirect"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before,
        "deck unchanged — redirect bypassed the deck route"
    );
}

/// Decode installs both `WhenWouldBeReturnedToDeck` and
/// `WhenWouldBeReturnedToHand` effects — printed rules cover both routes.
/// This test drives a direct `return_to_hand` on a Decode Digimon and verifies
/// that a replacement selection actually installs (proving the hand-timing
/// effect is present). The previous Task 6 commit only installed the deck
/// timing, so `return_to_hand` would have completed synchronously with no
/// pending selection — failing the installation assertion below.
///
/// Post-§7.5 guard (Task 7): the `(Zone::Hand, Redirected(Zone::Hand))` arm
/// in `commit_deferred_outcome` now calls `return_to_hand(perm)` directly —
/// safe because the once-per-event guard blocks the re-entry from re-firing
/// the just-resolved Decode replacement for the same subject. The end state
/// is the Digimon moving to hand as expected.
#[test]
fn printed_decode_keyword_also_handles_return_to_hand() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("DECODE_CARD", vec![Keyword::Decode]))
        .start();
    let handle = r.place_on_field(0, "DECODE_CARD", Some(0));

    // Drive a return-to-hand flow directly. Because Decode installs a
    // `WhenWouldBeReturnedToHand` replacement, the call should short-circuit
    // while a PendingSelection::Replacement is installed for P0 to resolve.
    let result = r.game.return_to_hand(handle);
    assert!(
        result.is_none(),
        "return_to_hand short-circuits while selection pending"
    );
    assert!(
        r.game.pending_selection.is_some(),
        "Decode's WhenWouldBeReturnedToHand replacement should install a selection \
         (regression guard: Task-6 previously only installed the deck-timing effect)"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Decode replacement (hand route)");

    // After resolving accept, the permanent should be in P0's hand.
    assert_eq!(r.battle_area_size(0), 0, "Decode Digimon left battle area");
    assert_eq!(r.game.player(0).hand.len(), 1, "ended up in hand");
}

/// Keyword-derived replacements must be scoped to the specific permanent that
/// carries the keyword — a card in hand (no permanent on the field) should
/// not install a replacement that would fire for a different permanent being
/// deleted.
#[test]
fn printed_keyword_effects_only_apply_to_permanents_on_field() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("BARRIER_CARD", vec![Keyword::Barrier]))
        .add_card(make_test_card("FILLER", "FILLER"))
        .hand(0, &["BARRIER_CARD"])
        .deck(0, &["FILLER"; 5])
        .start();
    // Put a distinct plain permanent on the field. Its deletion must NOT
    // trigger the BARRIER_CARD's replacement (barrier is in hand, not field).
    let plain = r.place_on_field(0, "FILLER", Some(0));

    r.game.delete_permanent_with_effects(plain);

    // No selection should be installed — there is no WhenWouldBeDeleted
    // replacement in scope for FILLER.
    assert!(
        r.game.pending_selection.is_none(),
        "keyword effects must not leak from hand cards"
    );
    assert_eq!(
        r.battle_area_size(0),
        0,
        "FILLER should be deleted normally"
    );
    assert_eq!(
        r.game.player(0).hand.len(),
        1,
        "BARRIER_CARD still in hand"
    );
}
