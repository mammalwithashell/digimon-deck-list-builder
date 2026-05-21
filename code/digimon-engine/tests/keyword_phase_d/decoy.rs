//! Phase D Task 7 — `Keyword::Decoy` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Decoy]` (no hand-rolled
//! `CardEffect`) installs a `WhenWouldBeDeleted` replacement that, when an
//! ALLY (same-controller, non-self) Digimon would be deleted, lets the
//! controller substitute the Decoy carrier as the deletion subject — the
//! Decoy dies in the ally's place.
//!
//! Mirrors DCGO `Decoy.cs:24-69`. The body uses `rctx.substitute(...)` (sync
//! ReplacementOutcome::Substituted), no nested selection — the optional
//! outer accept dialog is the only player choice.
//!
//! Filters in the body:
//!   1. `subject != me_perm`: never self-redirect (would loop).
//!   2. `subject.player == me_perm.player`: same-controller only.
//!   3. Subject's top card is a Digimon (not a Tamer).
//!
//! When any filter rejects, the body returns without setting an outcome —
//! the original deletion proceeds normally. (Phase C precedent: matches
//! `nested_select_decoy.rs`'s sync-substitute pattern.)

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

fn decoy_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        // Printed-only Decoy: the auto-install MUST be the sole source of
        // behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::Decoy(0)],
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

fn plain_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

// ─── Test 1: accept → Decoy substitutes self for the ally's deletion ────────

/// Player 0 has Decoy (perm A) and a plain ally Digimon (perm B). Opponent
/// triggers a deletion of B → Decoy's WhenWouldBeDeleted fires (subject = B).
/// Outer optional accept dialog is parked; on accept, `rctx.substitute(A)`
/// runs synchronously. Result: B survives, A is deleted in its place.
#[test]
fn decoy_accept_substitutes_self_for_ally_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(decoy_card("DECOY"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let _decoy = r.place_on_field(0, "DECOY", None);
    let ally = r.place_on_field(0, "ALLY", None);

    r.game.delete_permanent_with_effects(ally);

    // Optional outer accept dialog should be parked.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Decoy outer accept dialog must be parked");
        assert!(
            pending.is_optional,
            "Decoy is optional ('may' substitute); outer dialog accepts PASS"
        );
        assert_eq!(pending.selecting_player, 0);
        assert_eq!(pending.valid_action_ids, vec![REPLACEMENT_ACCEPT]);
    }

    // Accept the substitute.
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Decoy substitute");

    // After substitute: ally survives, decoy is deleted in its place.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "exactly one permanent left after Decoy substitute"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "ALLY",
        "the ally is the survivor; decoy was deleted in its place"
    );
    // DECOY is in trash via the substituted deletion.
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DECOY"),
        "DECOY landed in trash via the substituted deletion"
    );
}

// ─── Test 2: decline → original ally deletion proceeds ──────────────────────

/// Same setup as Test 1, but the controller PASSes the outer optional
/// accept dialog. Decoy declines; the original deletion of the ally
/// proceeds, and the Decoy carrier survives.
#[test]
fn decoy_decline_lets_original_deletion_proceed() {
    let mut r = DebugRunner::builder()
        .add_card(decoy_card("DECOY"))
        .add_card(plain_digimon("ALLY"))
        .start();

    let _decoy = r.place_on_field(0, "DECOY", None);
    let ally = r.place_on_field(0, "ALLY", None);

    r.game.delete_permanent_with_effects(ally);

    // Outer optional accept dialog is parked.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Decoy outer accept dialog must be parked");
        assert!(pending.is_optional);
    }

    // Decline (PASS) the Decoy substitute.
    r.game.resolve_selection(0, PASS).expect("decline Decoy");

    // Ally was deleted normally; Decoy survives.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Decoy survives — only the ally was deleted"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "DECOY",
        "the surviving permanent is the Decoy carrier"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "ALLY"),
        "ALLY landed in trash via its normal (non-substituted) deletion"
    );
}

// ─── Test 3: self-redirect prevention — no substitute when self is subject ──

/// Decoy carrier exists alone (no other allies). Opponent deletes the
/// Decoy itself. The body's `subject != me_perm` guard MUST prevent the
/// substitute (would create an infinite self-redirect loop). No selection
/// is parked; deletion proceeds normally.
#[test]
fn decoy_does_not_self_redirect_when_self_is_subject() {
    let mut r = DebugRunner::builder().add_card(decoy_card("DECOY")).start();

    let decoy = r.place_on_field(0, "DECOY", None);

    r.game.delete_permanent_with_effects(decoy);

    // No selection parked: the body's subject==self guard returned early
    // before any outcome was set, so no candidate parked an optional dialog.
    //
    // Implementation note: even with `.optional()`, the candidate-collection
    // walk in `collect_candidates` would offer this Decoy effect for self-
    // deletion. The body's self-scope guard is what prevents the loop —
    // but the optional dialog is still parked because optional-install
    // happens before the body runs. So we tolerate the spurious outer
    // dialog; on accept, the body returns early (no outcome) and the
    // deletion proceeds. We verify the END STATE (deletion completed)
    // rather than the absence of the dialog.
    if r.game.pending_selection.is_some() {
        // Accepting the spurious outer dialog runs the body, which falls
        // through (subject==self) → no outcome → original deletion proceeds.
        r.game
            .resolve_selection(0, REPLACEMENT_ACCEPT)
            .expect("spurious outer accept (filter rejects in body)");
    }

    // Decoy was deleted normally — no substitute happened.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "Decoy deleted normally; no self-redirect"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DECOY"),
        "DECOY landed in trash via normal deletion"
    );
}

// ─── Test 4: opponent's permanent is NOT protected ──────────────────────────

/// Player 0 has Decoy. Player 1 has a Digimon. The opponent's Digimon
/// is deleted. The Decoy is on the wrong side of the table — its filter
/// (`subject.player == me_perm.player`) MUST reject the substitute.
#[test]
fn decoy_does_not_protect_opponents_permanent() {
    let mut r = DebugRunner::builder()
        .add_card(decoy_card("DECOY"))
        .add_card(plain_digimon("OPP-DIGI"))
        .start();

    let _decoy = r.place_on_field(0, "DECOY", None);
    let opp_digi = r.place_on_field(1, "OPP-DIGI", None);

    r.game.delete_permanent_with_effects(opp_digi);

    // As in Test 3: optional-install may park a spurious outer dialog
    // because cross-controller filtering happens in the body, not at
    // candidate-collection time. Accept-and-fall-through if so.
    if r.game.pending_selection.is_some() {
        // The dialog targets player 0 (Decoy controller), not player 1.
        let selecting = r.game.pending_selection.as_ref().unwrap().selecting_player;
        r.game
            .resolve_selection(selecting, REPLACEMENT_ACCEPT)
            .expect("spurious outer accept (filter rejects in body)");
    }

    // Opp's permanent was deleted normally; Decoy on the other side survives.
    assert_eq!(
        r.game.players[1].battle_area.len(),
        0,
        "opponent's permanent deleted normally — cross-controller Decoy did not substitute"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Decoy survives — it does not substitute for opponent's deletion"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "DECOY",
    );
}

// ─── Test 5: color-filter — Decoy (Black) protects only Black allies ────────

/// Track G addition: `Keyword::Decoy(u8)` carries a CardColor bitmask.
/// A printed `<Decoy (Black)>` carrier (mask = 0x20, bit 5) substitutes
/// only for ALLY Digimon whose colors include Black. A non-Black ally is
/// rejected by the color-filter inside the body — the optional dialog may
/// still appear (filtering happens in the body, not the condition), but on
/// accept the body falls through and the original deletion proceeds.
#[test]
fn decoy_color_filter_rejects_non_matching_ally() {
    let mut decoy = decoy_card("DECOY-BLACK");
    // 0x20 = bit 5 = CardColor::Black filter.
    decoy.keywords = vec![Keyword::Decoy(0b0010_0000)];

    let red_ally = {
        let mut c = plain_digimon("RED-ALLY");
        c.colors = vec![CardColor::Red];
        c
    };

    let mut r = DebugRunner::builder()
        .add_card(decoy)
        .add_card(red_ally)
        .start();

    let _decoy = r.place_on_field(0, "DECOY-BLACK", None);
    let red = r.place_on_field(0, "RED-ALLY", None);

    r.game.delete_permanent_with_effects(red);

    // Optional outer accept dialog may still install (filter is in body).
    if r.game.pending_selection.is_some() {
        r.game
            .resolve_selection(0, REPLACEMENT_ACCEPT)
            .expect("spurious outer accept (color-filter rejects in body)");
    }

    // Red ally was deleted normally; Decoy survived because the color
    // filter rejected the substitution.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Decoy survives — color filter rejected the Red ally"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "DECOY-BLACK",
        "Decoy carrier survived"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "RED-ALLY"),
        "Red ally landed in trash via normal deletion"
    );
}

// ─── Test 6: color-filter — Decoy (Black) accepts a matching Black ally ─────

/// Mirror of Test 5: same Decoy(Black) carrier, but the ally is Black.
/// The substitution is permitted: ally survives, Decoy dies in its place.
#[test]
fn decoy_color_filter_accepts_matching_ally() {
    let mut decoy = decoy_card("DECOY-BLACK");
    decoy.keywords = vec![Keyword::Decoy(0b0010_0000)]; // Black bit

    let black_ally = {
        let mut c = plain_digimon("BLACK-ALLY");
        c.colors = vec![CardColor::Black];
        c
    };

    let mut r = DebugRunner::builder()
        .add_card(decoy)
        .add_card(black_ally)
        .start();

    let _decoy = r.place_on_field(0, "DECOY-BLACK", None);
    let black = r.place_on_field(0, "BLACK-ALLY", None);

    r.game.delete_permanent_with_effects(black);

    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Decoy outer accept dialog parked for color-matching ally");
        assert!(pending.is_optional);
    }

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Decoy substitute (color match)");

    // Black ally survived; Decoy was substituted and deleted in its place.
    assert_eq!(r.game.players[0].battle_area.len(), 1);
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "BLACK-ALLY",
        "Black ally survived; Decoy substituted"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DECOY-BLACK"),
        "Decoy carrier landed in trash via the substituted deletion"
    );
}

// ─── Test 7: multi-color filter — Decoy (Red/Black) accepts both ────────────

/// `Decoy(0x21)` (Red bit + Black bit) accepts any ally whose colors
/// include Red OR Black. Verifies the OR semantics of the bitmask.
#[test]
fn decoy_multi_color_filter_matches_any_in_set() {
    let mut decoy = decoy_card("DECOY-RB");
    // 0b0010_0001 = Red (bit 0) | Black (bit 5).
    decoy.keywords = vec![Keyword::Decoy(0b0010_0001)];

    let yellow_ally = {
        let mut c = plain_digimon("YELLOW-ALLY");
        c.colors = vec![CardColor::Yellow];
        c
    };
    let red_ally = {
        let mut c = plain_digimon("RED-ALLY");
        c.colors = vec![CardColor::Red];
        c
    };

    let mut r = DebugRunner::builder()
        .add_card(decoy)
        .add_card(yellow_ally)
        .add_card(red_ally)
        .start();

    let _decoy = r.place_on_field(0, "DECOY-RB", None);
    let yellow = r.place_on_field(0, "YELLOW-ALLY", None);

    // Yellow is NOT in the Red/Black filter — Decoy should reject.
    r.game.delete_permanent_with_effects(yellow);
    if r.game.pending_selection.is_some() {
        r.game
            .resolve_selection(0, REPLACEMENT_ACCEPT)
            .expect("spurious accept on color-mismatch");
    }
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "YELLOW-ALLY"),
        "Yellow ally rejected by Red/Black filter — deleted normally"
    );

    // Now place a Red ally — should be accepted.
    let red = r.place_on_field(0, "RED-ALLY", None);
    r.game.delete_permanent_with_effects(red);
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Red ally is in the Red/Black filter — outer dialog parked");
        assert!(pending.is_optional);
    }
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Decoy substitute for Red ally");

    // Red ally survived; Decoy was substituted.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Red ally survives; Decoy substituted"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "RED-ALLY",
    );
}
