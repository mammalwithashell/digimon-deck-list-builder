//! Phase D Task 6 — `Keyword::Save` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Save]` (no hand-rolled
//! `CardEffect`) must, on `WhenWouldBeDeleted`:
//!   1. Park an OUTER optional accept dialog ("you may save"). On ACCEPT, an
//!      INNER selection is parked over the controller's own Tamers; the
//!      controller picks a Tamer; the carrier's top card is moved to the
//!      bottom of that Tamer's stack and the original deletion is cancelled.
//!   2. On DECLINE of the outer dialog: original deletion proceeds normally,
//!      no Tamer mutation.
//!   3. With NO own Tamers on field: the outer accept dialog still installs
//!      (Phase C does not pre-filter on inner-filter emptiness). On ACCEPT,
//!      the inner `select_own_permanent` sees zero candidates and silently
//!      no-ops (no `pending_selection` installed); the user callback never
//!      runs; outcome stays `None`; original deletion proceeds.
//!
//! Mirrors DCGO `Save.cs:24-65`.

use digimon_engine::action::space::{PASS, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

fn save_card(id: &str) -> CardData {
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
        // Printed-only Save: the auto-install MUST be the sole source of
        // behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::Save],
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn tamer_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[allow(dead_code)]
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Append `card_id` on top of a field permanent's digivolution stack so it
/// becomes the new visible top.
#[allow(dead_code)]
fn push_source_card(r: &mut DebugRunner, player: u8, field_index: usize, card_id: &str) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_card: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    r.game.players[player as usize].battle_area[field_index]
        .card_sources
        .push(card);
}

// ─── Test 1: accept + pick → tucked under Tamer, deletion cancelled ─────────

#[test]
fn save_accept_picks_tamer_and_cancels_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer_card("TAMER"))
        .start();

    let saved = r.place_on_field(0, "SAVE-D", None);
    let _t = r.place_on_field(0, "TAMER", None);

    // Snapshot tamer's pre-Save stack length.
    let tamer_stack_before = r.game.players[0].battle_area[1].card_sources.len();

    r.game.delete_permanent_with_effects(saved);

    // Outer optional-accept dialog should be installed by `.optional()`.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Save's outer optional-accept dialog must be installed");
        assert_eq!(pending.selecting_player, 0);
    }

    // Accept the optional-replacement.
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Save");

    // Inner Tamer pick is up; pick the only Tamer.
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("inner Tamer-select must be installed after accept");
        let action = pending.valid_action_ids[0];
        let player = pending.selecting_player;
        r.game.resolve_selection(player, action).expect("pick Tamer");
    }

    // The carrier-permanent ceases to exist as a separate permanent — its
    // top card (SAVE-D) was tucked under the Tamer; only the Tamer remains
    // in battle area.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "carrier removed from battle_area (top card tucked under Tamer); \
         only the Tamer remains"
    );
    let surviving = &r.game.players[0].battle_area[0];
    assert_eq!(
        surviving.top_card().card_id(&r.game.card_data),
        "TAMER",
        "the surviving battle-area permanent is the Tamer (SAVE-D was tucked beneath it)"
    );
    // Tamer gained one card under it: SAVE-D should now be at index 0 (bottom).
    assert_eq!(
        surviving.card_sources.len(),
        tamer_stack_before + 1,
        "Tamer gained exactly one source card (the saved Digimon)"
    );
    assert_eq!(
        surviving.card_sources[0].card_id(&r.game.card_data),
        "SAVE-D",
        "SAVE-D was placed at the BOTTOM of the Tamer's stack (index 0)"
    );

    // SAVE-D should NOT be in trash (cancel_leave fired).
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SAVE-D"),
        "SAVE-D must not have been trashed when Save cancelled the deletion"
    );
}

// ─── Test 2: outer decline → original deletion proceeds ─────────────────────

#[test]
fn save_decline_proceeds_with_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer_card("TAMER"))
        .start();

    let saved = r.place_on_field(0, "SAVE-D", None);
    let _t = r.place_on_field(0, "TAMER", None);

    let tamer_stack_before = r.game.players[0].battle_area[1].card_sources.len();

    r.game.delete_permanent_with_effects(saved);

    assert!(
        r.game.pending_selection.is_some(),
        "outer optional-accept dialog must be installed"
    );

    // Decline the optional-replacement (PASS).
    r.game.resolve_selection(0, PASS).expect("decline Save");

    // Original deletion proceeded → SAVE-D is gone, only the Tamer remains.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "decline → original deletion → SAVE-D is gone, only the Tamer remains"
    );
    assert_eq!(
        r.game.players[0].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "TAMER",
    );
    // Tamer's stack must be unchanged.
    assert_eq!(
        r.game.players[0].battle_area[0].card_sources.len(),
        tamer_stack_before,
        "Tamer stack must NOT mutate when Save was declined"
    );
    // SAVE-D should be in trash (normal deletion).
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SAVE-D"),
        "SAVE-D went to trash via normal deletion"
    );
}

// ─── Test 3: no own Tamers → original deletion proceeds ─────────────────────

#[test]
fn save_with_no_tamers_does_not_protect() {
    // No Tamer on field → the candidate filter for the inner select is empty.
    // The outer optional-accept still installs (Phase C does not pre-filter
    // on inner-filter emptiness — that's a Phase D auto-install authoring
    // concern). On accept, the inner `select_own_permanent` sees zero
    // candidates and silently no-ops; the user callback never runs; outcome
    // stays None; original deletion proceeds.
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .start();

    let saved = r.place_on_field(0, "SAVE-D", None);

    r.game.delete_permanent_with_effects(saved);

    // Outer accept dialog still installs.
    assert!(
        r.game.pending_selection.is_some(),
        "outer optional-accept dialog must still be installed"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept (but no Tamers exist)");

    // Inner select had no candidates → no PendingSelection installed; user
    // callback never ran; outcome stayed None → original deletion proceeded.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "empty Tamer filter → outcome=None → original deletion proceeds"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SAVE-D"),
        "SAVE-D went to trash via normal deletion"
    );
}

// ─── Test 4: opponent-controlled Tamers must NOT be selectable ─────────────

#[test]
fn save_does_not_offer_opponent_tamers() {
    // Player 0 has SAVE-D. Player 1 has a Tamer. Save must NOT offer the
    // opponent's Tamer as a candidate — DCGO `Save.cs` filters
    // `customMessageArrayTemplate(CanSelectDigimon: false, CanSelectTamer: true)`
    // against own permanents.
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer_card("OPP-TAMER"))
        .start();

    let saved = r.place_on_field(0, "SAVE-D", None);
    let _opp_tamer = r.place_on_field(1, "OPP-TAMER", None);

    let opp_tamer_stack_before = r.game.players[1].battle_area[0].card_sources.len();

    r.game.delete_permanent_with_effects(saved);

    // Outer accept dialog installs.
    assert!(
        r.game.pending_selection.is_some(),
        "outer optional-accept dialog must install"
    );

    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Save");

    // Inner select is over OWN Tamers only — controller has zero own Tamers,
    // so the inner select has no candidates. Same outcome as test 3: original
    // deletion proceeds.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "no own Tamers → outcome=None → original deletion proceeds"
    );
    // Opponent's Tamer stack must be unchanged.
    assert_eq!(
        r.game.players[1].battle_area[0].card_sources.len(),
        opp_tamer_stack_before,
        "opponent's Tamer must NOT receive the saved card",
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SAVE-D"),
        "SAVE-D went to trash via normal deletion"
    );
}
