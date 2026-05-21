//! Phase D — `Keyword::Evade` auto-install behavioral tests.
//!
//! Printed text (per `data/cards.json`): "When this Digimon would be deleted,
//! you may suspend it to prevent that deletion." Mirrors DCGO `Evade.cs:38-49`
//! (suspend cost + `willBeRemoveField = false`).
//!
//! A card declaring ONLY `keywords: vec![Keyword::Evade]` (no hand-rolled
//! `CardEffect`) installs a `WhenWouldBeDeleted` replacement that:
//!   1. Gates at candidate-collection time on `!is_suspended` (DCGO's
//!      `CanActivatePermanentSuspendCostEffect`). An already-suspended
//!      carrier cannot pay the cost — the outer accept dialog is suppressed
//!      and the original deletion proceeds normally.
//!   2. On accept: pays the cost by suspending the carrier (firing
//!      `OnSuspend` observers), then cancels the deletion. Carrier survives
//!      on the field, suspended.
//!   3. On decline (PASS): no state change — the original deletion proceeds.
//!   4. Self-scopes inside the body: a neighbor's deletion never causes the
//!      Evade carrier's own state to change.
//!   5. Is cause-agnostic: both battle-deletion and effect-deletion offer
//!      the replacement (matches DCGO — `Evade.cs` does not filter on
//!      deletion cause).

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

use super::helpers::drain_outer_accept_dialog;

fn evade_card(id: &str) -> CardData {
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
        // Printed-only Evade: the auto-install MUST be the sole source of
        // behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::Evade],
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

// ─── Test 1: accept — suspend + cancel deletion (effect-cause) ──────────────

/// `delete_permanent_with_effects` represents the generic effect-deletion
/// path. Evade should fire and offer the player an accept dialog; on accept,
/// the carrier is suspended and the deletion is cancelled.
#[test]
fn evade_effect_deletion_accept_suspends_and_cancels() {
    let mut r = DebugRunner::builder().add_card(evade_card("EVADE")).start();
    let evade = r.place_on_field(0, "EVADE", None);

    let trash_before = r.game.players[0].trash.len();
    let deck_before = r.game.players[0].deck.len();

    r.game.delete_permanent_with_effects(evade);

    // Outer optional accept dialog parked.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Evade outer accept dialog must be parked");
        assert!(
            pending.is_optional,
            "Evade is optional ('you may suspend it'); outer dialog must be is_optional"
        );
        assert_eq!(pending.selecting_player, 0);
    }

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Evade replacement");

    // Carrier survives on the field, suspended.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Evade cancels the deletion — carrier stays on field"
    );
    let perm = &r.game.players[0].battle_area[0];
    assert!(
        perm.is_suspended,
        "Evade pays its cost by suspending the carrier"
    );
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "EVADE",
        "the surviving carrier is EVADE"
    );

    // Nothing landed in trash or deck.
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before,
        "deletion cancelled — trash unchanged"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_before,
        "Evade is NOT a deck-redirect — deck unchanged"
    );
}

// ─── Test 2: accept — suspend + cancel deletion (battle-cause) ──────────────

/// Battle-cause deletion route via `delete_permanent_with_cause`. Evade
/// auto-install does NOT filter on deletion cause (DCGO's `Evade.cs` is
/// cause-agnostic), so it must fire on battle deletions too.
#[test]
fn evade_battle_deletion_accept_suspends_and_cancels() {
    let mut r = DebugRunner::builder().add_card(evade_card("EVADE")).start();
    let evade = r.place_on_field(0, "EVADE", None);

    r.game
        .delete_permanent_with_cause(evade, ReplacementCause::Battle);

    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Evade should fire on battle deletions too");
        assert!(pending.is_optional);
    }
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Evade on battle deletion");

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Evade cancels battle deletion — carrier survives"
    );
    assert!(r.game.players[0].battle_area[0].is_suspended);
}

// ─── Test 3: decline — original deletion proceeds ───────────────────────────

/// Declining (PASS) the outer optional accept dialog leaves no state change —
/// the original deletion proceeds; the carrier lands in trash.
#[test]
fn evade_decline_proceeds_with_deletion() {
    let mut r = DebugRunner::builder().add_card(evade_card("EVADE")).start();
    let evade = r.place_on_field(0, "EVADE", None);

    let trash_before = r.game.players[0].trash.len();

    r.game.delete_permanent_with_effects(evade);
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Evade outer accept dialog must be parked");
        assert!(pending.is_optional);
    }

    r.game.resolve_selection(0, PASS).expect("decline Evade");

    // Original deletion proceeded normally.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "Evade declined — carrier deleted normally"
    );
    assert_eq!(
        r.game.players[0].trash.len(),
        trash_before + 1,
        "carrier landed in trash via the normal deletion"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EVADE"),
        "EVADE landed in trash"
    );
}

// ─── Test 4: gate — already-suspended carrier cannot pay the cost ───────────

/// DCGO `CanActivateEvade` → `CanActivatePermanentSuspendCostEffect` requires
/// `!IsSuspended`. The Rust auto-install puts the same gate on
/// `condition`, so an already-suspended carrier never sees the outer accept
/// dialog and the original deletion proceeds normally.
#[test]
fn evade_already_suspended_carrier_does_not_offer() {
    let mut r = DebugRunner::builder().add_card(evade_card("EVADE")).start();
    let evade = r.place_on_field(0, "EVADE", None);

    // Pre-suspend the carrier — Evade cannot pay its cost a second time.
    r.game.players[0].battle_area[evade.index as usize].is_suspended = true;

    r.game.delete_permanent_with_effects(evade);

    // Gate fail → no candidate parked → no outer accept dialog.
    assert!(
        r.game.pending_selection.is_none(),
        "Evade gate failed (already suspended); no accept dialog should be parked"
    );

    // Deletion proceeded normally.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "deletion proceeded normally when Evade gate fails"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "EVADE"),
        "EVADE landed in trash"
    );
}

// ─── Test 5: self-scope — neighbor's deletion does NOT consume the carrier ──

/// A neighbor's deletion candidate-walk may park a spurious outer accept
/// dialog for the Evade carrier (the dispatcher walks all battle-area
/// permanents whose effects match the timing). The body's self-scope guard
/// (`Some(subject) != me_perm`) MUST short-circuit before suspending the
/// carrier or cancelling the neighbor's deletion.
#[test]
fn evade_does_not_fire_on_neighbor_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(evade_card("EVADE"))
        .add_card(plain_digimon("NEIGHBOR"))
        .start();

    let _evade = r.place_on_field(0, "EVADE", None);
    let neighbor = r.place_on_field(0, "NEIGHBOR", None);

    let battle_size_before = r.game.players[0].battle_area.len();
    assert_eq!(battle_size_before, 2);

    r.game.delete_permanent_with_effects(neighbor);

    // Optional candidate-walk MAY install a spurious outer accept dialog for
    // the Evade carrier. The body's self-scope guard returns early without
    // suspending or cancelling. We tolerate the dialog and accept it.
    if r.game.pending_selection.is_some() {
        drain_outer_accept_dialog(&mut r, REPLACEMENT_ACCEPT);
    }

    // Carrier untouched: not suspended, still on field. Neighbor deleted.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "exactly one permanent left (the Evade carrier)"
    );
    let surviving = &r.game.players[0].battle_area[0];
    assert_eq!(
        surviving.top_card().card_id(&r.game.card_data),
        "EVADE",
        "EVADE carrier survived the neighbor's deletion"
    );
    assert!(
        !surviving.is_suspended,
        "Evade self-scope guard MUST NOT suspend the carrier on a neighbor's deletion"
    );
}

// ─── Test 6: empty-deck independence ────────────────────────────────────────

/// Sanity check that Evade is not deck-dependent (since it no longer
/// redirects to the deck). With the deck emptied to zero cards, Evade still
/// pays its cost and cancels the deletion.
#[test]
fn evade_works_with_empty_deck() {
    let mut r = DebugRunner::builder().add_card(evade_card("EVADE")).start();
    let evade = r.place_on_field(0, "EVADE", None);

    // Drain the deck. Evade is independent of deck state — the keyword
    // is "suspend it", not "place at deck bottom".
    r.game.players[0].deck.clear();
    let deck_before = r.game.players[0].deck.len();
    assert_eq!(deck_before, 0);

    r.game.delete_permanent_with_effects(evade);
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("Evade with empty deck");

    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Evade still cancels deletion even with an empty deck"
    );
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "carrier suspended"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        0,
        "Evade does not move anything to the deck"
    );
}
