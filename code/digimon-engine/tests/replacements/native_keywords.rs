//! Phase 7 Task 6 — native-keyword auto-install replacement tests.
//!
//! When a card's `CardData::keywords` contains one of the Phase-7 replacement
//! keywords (`Barrier`, `Evade`, `Fragment(N)`), the engine should
//! auto-install a matching `WhenWouldBe*` replacement effect at
//! `Game::effects_for_card` so the card behaves as printed without needing a
//! hand-authored `CardEffect` script.
//!
//! `Decode` is the deliberate exception (task_69f10a66 Family 2): its rule
//! 16-35 behavior is per-card parameterized (play a matching Digimon from the
//! carrier's own digivolution cards on a non-battle leave), so no
//! keyword-generic auto-effect exists — real Decode lives in each card's
//! self-scoped YAML replacement clauses, and a bare `Keyword::Decode` marker
//! installs nothing (pinned below).
//!
//! Partition / ArmorPurge are explicitly **deferred** from this task —
//! see TODO(phase-7-followup) in `cards/keyword_effects.rs`.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;
use digimon_engine::replacement::ReplacementCause;

fn card_with_keywords(id: &str, keywords: Vec<Keyword>) -> digimon_engine::CardData {
    let mut c = make_test_card(id, id);
    c.keywords = keywords;
    c
}

/// Printed `<Barrier>` on a card's `CardData::keywords` should auto-install
/// an optional `WhenWouldBeDeleted` replacement that, on accept, trashes the
/// top of the owner's security and cancels the deletion.
#[test]
fn printed_barrier_keyword_auto_installs_replacement() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("BARRIER_CARD", vec![Keyword::Barrier]))
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC", "SEC"])
        .start();
    let handle = r.place_on_field(0, "BARRIER_CARD", Some(0));

    assert_eq!(r.game.player(0).security.len(), 2);
    assert_eq!(r.game.player(0).trash.len(), 0);
    assert_eq!(r.battle_area_size(0), 1);

    r.game
        .delete_permanent_with_cause(handle, ReplacementCause::Battle);

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
        r.game.player(0).security.len(),
        1,
        "top of security should be trashed"
    );
    assert_eq!(r.game.player(0).trash.len(), 1);
}

/// Declining an optional Barrier replacement allows the deletion to proceed.
#[test]
fn printed_barrier_keyword_decline_allows_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("BARRIER_CARD", vec![Keyword::Barrier]))
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC", "SEC"])
        .start();
    let handle = r.place_on_field(0, "BARRIER_CARD", Some(0));

    r.game
        .delete_permanent_with_cause(handle, ReplacementCause::Battle);
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
        r.game.player(0).security.len(),
        2,
        "security unchanged on decline"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        1,
        "the permanent itself goes to trash"
    );
}

/// Barrier cannot be paid with no security, so the optional replacement should
/// not even install.
#[test]
fn printed_barrier_keyword_without_security_does_not_install() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("BARRIER_CARD", vec![Keyword::Barrier]))
        .start();
    let handle = r.place_on_field(0, "BARRIER_CARD", Some(0));

    r.game
        .delete_permanent_with_cause(handle, ReplacementCause::Battle);

    assert!(
        r.game.pending_selection.is_none(),
        "Barrier should not prompt when its security-trash cost cannot be paid"
    );
    assert_eq!(r.battle_area_size(0), 0, "deletion proceeds normally");
    assert_eq!(r.game.player(0).trash.len(), 1);
}

/// Barrier only covers battle deletion; effect deletion should proceed without
/// offering the replacement.
#[test]
fn printed_barrier_keyword_ignores_non_battle_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("BARRIER_CARD", vec![Keyword::Barrier]))
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC", "SEC"])
        .start();
    let handle = r.place_on_field(0, "BARRIER_CARD", Some(0));

    r.game
        .delete_permanent_with_cause(handle, ReplacementCause::OpponentEffect);

    assert!(
        r.game.pending_selection.is_none(),
        "Barrier should not prompt for non-battle deletion"
    );
    assert_eq!(r.battle_area_size(0), 0, "deletion proceeds normally");
    assert_eq!(r.game.player(0).security.len(), 2);
    assert_eq!(
        r.game.player(0).trash.len(),
        1,
        "only the deleted permanent should be trashed"
    );
}

/// Printed `<Evade>` auto-installs an optional `WhenWouldBeDeleted`
/// replacement that suspends the carrier and cancels the deletion.
/// Per printed text: "When this Digimon would be deleted, you may suspend it
/// to prevent that deletion." Mirrors DCGO `Evade.cs:38-49` (suspend cost +
/// `willBeRemoveField = false`).
#[test]
fn printed_evade_keyword_suspends_and_cancels_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("EVADE_CARD", vec![Keyword::Evade]))
        .start();
    let handle = r.place_on_field(0, "EVADE_CARD", Some(0));

    let deck_before = r.game.player(0).deck.len();
    let trash_before = r.game.player(0).trash.len();
    r.game.delete_permanent_with_effects(handle);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Evade replacement");

    assert_eq!(
        r.battle_area_size(0),
        1,
        "Evade keeps the carrier on the field — only the deletion is cancelled"
    );
    let perm = &r.game.player(0).battle_area[0];
    assert!(
        perm.is_suspended,
        "Evade pays its cost by suspending the carrier"
    );
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before,
        "Evade does not move the carrier to the deck (printed text says 'suspend it', not 'place at deck bottom')"
    );
    assert_eq!(
        r.game.player(0).trash.len(),
        trash_before,
        "Evade cancels the deletion — nothing lands in trash"
    );
}

/// task_69f10a66 Family 2 — printed `<Decode>` installs NO keyword-generic
/// auto-effect. The retired legacy pair ("would be returned to deck/hand →
/// return to hand instead") was not rule 16-35 Decode: per §16-35-1 Decode
/// triggers on the CARRIER's own non-battle leave and plays a per-card-
/// parameterized Digimon from the carrier's digivolution cards — the
/// parameter is unknowable at the keyword layer, so real Decode is authored
/// per-card as self-scoped YAML replacement clauses (EX12-031 / BT22-015 /
/// P-213). This test replaces the two retired tests that pinned the legacy
/// redirect: a bare `Keyword::Decode` card returned to deck goes to the deck,
/// with no replacement window.
#[test]
fn printed_decode_keyword_installs_no_generic_replacement() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("DECODE_CARD", vec![Keyword::Decode]))
        .start();
    let handle = r.place_on_field(0, "DECODE_CARD", Some(0));

    let deck_before = r.game.player(0).deck.len();

    // Return-to-deck completes synchronously — no fabricated window.
    let ok = r
        .game
        .return_to_deck(handle, digimon_engine::enums::StackPosition::Bottom);
    assert!(ok, "return_to_deck commits with no replacement window");
    assert!(
        r.game.pending_selection.is_none(),
        "bare Keyword::Decode must not park any replacement prompt (16-35 \
         Decode is per-card YAML, not a keyword-generic redirect)"
    );
    assert_eq!(r.battle_area_size(0), 0, "Digimon left the battle area");
    assert_eq!(
        r.game.player(0).deck.len(),
        deck_before + 1,
        "the card went to the DECK — the legacy deck→hand redirect is retired"
    );
    assert_eq!(r.game.player(0).hand.len(), 0, "nothing landed in hand");
}

/// Companion to the test above for the hand route: a bare `Keyword::Decode`
/// card returned to hand goes to hand synchronously — the legacy symmetric
/// `WhenWouldBeReturnedToHand` window (which even fired for OTHER
/// permanents' leaves — the EX12-031#effect#1 exam divergence) is retired.
#[test]
fn printed_decode_keyword_return_to_hand_commits_without_window() {
    let mut r = DebugRunner::builder()
        .add_card(card_with_keywords("DECODE_CARD", vec![Keyword::Decode]))
        .start();
    let handle = r.place_on_field(0, "DECODE_CARD", Some(0));

    let result = r.game.return_to_hand(handle);
    assert!(
        result.is_some(),
        "return_to_hand commits synchronously with no replacement window"
    );
    assert!(
        r.game.pending_selection.is_none(),
        "bare Keyword::Decode must not park any replacement prompt"
    );
    assert_eq!(r.battle_area_size(0), 0, "Digimon left battle area");
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
    assert_eq!(r.game.player(0).hand.len(), 1, "BARRIER_CARD still in hand");
}
