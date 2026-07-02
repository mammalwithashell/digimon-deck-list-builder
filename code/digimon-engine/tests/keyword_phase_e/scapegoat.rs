//! Phase E §E2 — `Keyword::Scapegoat` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Scapegoat]` (no
//! hand-rolled `CardEffect`) must, when self would be deleted by anything
//! other than the controller's own effect, optionally let the controller
//! pick a different own permanent to delete instead.
//!
//! Mirrors DCGO `Scapegoat.cs` — Immediate-type, optional, picks
//! another own permanent.
//!
//! RULES_CONTEXT 16-31. Cause filter: deletion_cause() != OwnEffect.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

use super::helpers::plain_digimon;

fn scapegoat_card(id: &str) -> CardData {
    let mut c = plain_digimon(id);
    c.keywords = vec![Keyword::Scapegoat];
    c
}

// ─── Test 1: happy path — accept → substitute ally for self ─────────────────

/// P0 has SCAP (Scapegoat) and ALLY (plain). Opponent triggers SCAP's
/// deletion via OpponentEffect cause → outer accept dialog parks → on
/// accept, inner own-permanent pick offers ALLY → on ALLY pick, substitute
/// fires → SCAP survives, ALLY dies.
#[test]
fn scapegoat_substitutes_ally_for_self_on_opponent_effect_deletion() {
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);

    // Outer accept dialog: optional ("may" substitute).
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat outer accept dialog must be parked");
        assert!(
            pending.is_optional,
            "Scapegoat is optional ('may'); outer dialog must accept PASS"
        );
        assert_eq!(pending.selecting_player, 0);
    }
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Scapegoat substitute");

    // Inner pick: select an own permanent != self. ALLY is the only valid
    // candidate (SCAP itself is filtered out).
    let inner_action = {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat inner ally pick must be parked");
        // Mandatory once accepted.
        assert!(
            !pending.is_optional,
            "inner pick should be mandatory once accepted"
        );
        pending.valid_action_ids[0]
    };
    r.game
        .resolve_selection(0, inner_action)
        .expect("pick ally");

    // Post-state: SCAP survives, ALLY in trash.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "exactly one permanent should remain after Scapegoat substitute"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "SCAP",
        "SCAP should survive"
    );
    assert_eq!(r.game.players[0].trash.len(), 1, "ALLY should be in trash");
    assert_eq!(
        r.game.players[0].trash[0].card_id(&r.game.card_data),
        "ALLY",
        "ALLY should be in trash"
    );
}

#[test]
fn scapegoat_inner_prompt_clones_faithfully() {
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Scapegoat");
    let inner_action = {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat inner ally pick must be parked");
        assert!(
            r.game.pending_selection_resume.is_some(),
            "Scapegoat inner prompt must be resume-driven before cloning"
        );
        pending.valid_action_ids[0]
    };

    let mut clone = r.game.clone();
    clone
        .resolve_selection(0, inner_action)
        .expect("cloned Scapegoat pick resolves");
    assert!(
        r.game.pending_selection.is_some(),
        "resolving the clone must leave the original Scapegoat prompt intact"
    );
    r.game
        .resolve_selection(0, inner_action)
        .expect("original Scapegoat pick resolves");

    assert_eq!(
        clone.players[0].battle_area.len(),
        r.game.players[0].battle_area.len(),
        "clone and original should replay the same Scapegoat result"
    );
}

// ─── Test 2: cause gate — own-effect deletion does NOT trigger Scapegoat ────

/// Cause gate: own-effect deletion does NOT trigger Scapegoat.
#[test]
fn scapegoat_does_not_fire_on_own_effect_deletion() {
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OwnEffect);

    // No parked selection — Scapegoat's optional dialog should not fire.
    // The optional dialog may park but when resolved, the body's cause
    // filter should reject and SCAP should be deleted.
    if r.game.pending_selection.is_some() {
        // Spurious outer dialog (cause filter runs in body after accept).
        // Accept-and-fall-through — body rejects on OwnEffect cause.
        r.game
            .resolve_selection(0, REPLACEMENT_ACCEPT)
            .expect("spurious outer accept (cause filter rejects in body)");
    }

    // SCAP gone, ALLY survives.
    assert_eq!(
        r.game.players[0].trash.len(),
        1,
        "SCAP should be deleted by own-effect cause"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "ALLY should survive"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ALLY",
        "ALLY should be the surviving permanent"
    );
}

// ─── Test 3: decline the optional dialog — original deletion proceeds ────────

/// Decline the optional dialog — original deletion proceeds.
#[test]
fn scapegoat_decline_proceeds_with_self_deletion() {
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);
    let _ally = r.place_on_field(0, "ALLY", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);

    // Outer optional accept dialog is parked.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Scapegoat outer accept dialog must be parked");
        assert!(pending.is_optional);
    }

    r.game
        .resolve_selection(0, PASS)
        .expect("decline Scapegoat");

    // SCAP gone, ALLY survives.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "ALLY should survive after declining Scapegoat"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ALLY",
        "ALLY is the surviving permanent"
    );
    assert_eq!(r.game.players[0].trash.len(), 1, "SCAP should be in trash");
}

// ─── Test 4: no other own permanents — original deletion proceeds ────────────

/// No other own permanents → no inner pick offered → original deletion
/// proceeds. The outer "may" dialog may still park, but accepting offers
/// no targets → falls back to deletion.
///
/// Note: DCGO's `CanActivateScapegoat` returns false when no targets exist,
/// which would suppress the outer dialog entirely. If the Rust substrate
/// pre-filters (suppresses outer dialog when no candidates), this test
/// adapts by tolerating either behavior — the invariant is SCAP gets
/// deleted.
#[test]
fn scapegoat_no_other_permanents_proceeds_with_self_deletion() {
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .add_card(scapegoat_card("SCAP"))
        .start();

    let scap = r.place_on_field(0, "SCAP", None);

    r.game
        .delete_permanent_with_cause(scap, ReplacementCause::OpponentEffect);

    // Drain any optional dialog that may have parked.
    // Either the outer dialog was suppressed (no candidates), or it parked
    // but PASS/accept both result in SCAP being deleted.
    if r.game.pending_selection.is_some() {
        // Try PASS first — both should lead to SCAP being deleted.
        let _ = r.game.resolve_selection(0, PASS);
    }
    // If still pending after PASS, drain further.
    if r.game.pending_selection.is_some() {
        let pending = r.game.pending_selection.as_ref().unwrap();
        let action = pending.valid_action_ids.first().copied().unwrap_or(PASS);
        let player = pending.selecting_player;
        let _ = r.game.resolve_selection(player, action);
    }

    assert!(
        r.game.players[0].battle_area.is_empty(),
        "SCAP should be deleted when no other permanents are available"
    );
    assert_eq!(r.game.players[0].trash.len(), 1);
}
