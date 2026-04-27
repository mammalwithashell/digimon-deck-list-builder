//! Phase D Task 6 — `Keyword::Save` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Save]` (no hand-rolled
//! `CardEffect`) installs an `OnDeletion` trigger that mirrors DCGO
//! `Save.cs:24-65`'s **post-deletion** semantics:
//!
//!   - Save fires AFTER the carrier has been deleted normally; its top card
//!     has already moved to the controller's trash with the rest of the
//!     digi-source stack.
//!   - The handler parks an OPTIONAL Tamer-pick selection
//!     (`is_optional=true`). PASS = "decline Save" — saved card stays in
//!     trash with the digi-sources.
//!   - On Tamer pick, `place_card_under_permanent_bottom` retrieves the
//!     saved card from trash via `remove_card_from_any_zone` and tucks it
//!     under the chosen Tamer.
//!   - With no own Tamers on field, the Tamer-pick has no candidates and
//!     `select_own_permanent` no-ops silently — handler returns; deletion
//!     has already completed naturally.
//!
//! ## Why post-deletion (not `WhenWouldBeDeleted` replacement)
//!
//! DCGO `Save.cs` requires `IsTopCardInTrashOnDeletion` (top has already
//! moved to trash) and **never** sets `willBeRemoveField = false`. Modeling
//! Save as a replacement that calls `cancel_leave()` would suppress
//! `OnDeletion` / `OnAnyDeletion` observers (e.g. Phase D Task 8 Fortitude),
//! which is a hard parity bug. See `keyword_effects.rs::Keyword::Save` for
//! the full rationale.

use digimon_engine::action::space::PASS;
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

// ─── Test 1: pick a Tamer → saved card moves from trash to under Tamer ──────

/// DCGO parity: Save fires as an OnDeletion trigger and parks an optional
/// Tamer-pick. The deletion's post-OnDeletion finalization (linked-card
/// drain, `delete_permanent`, `OnAnyDeletion`) is deferred via the
/// `pending_deletion_resume` substrate hook so the parked selection's
/// `valid_action_ids` (encoded against current field indices) stay valid
/// until the user resolves. After the Tamer-pick resolves:
///   1. The handler moves the saved top card from the carrier's stack
///      under the chosen Tamer.
///   2. The deferred finalizer runs: the carrier (now without its top)
///      has its remaining digi-sources trashed and is removed from the
///      battle area.
///   3. `OnAnyDeletion` observers fire normally — Save did NOT cancel
///      the deletion, so cards like Phase D Task 8 Fortitude will
///      observe the event as expected.
#[test]
fn save_accept_places_card_under_tamer_post_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-D"))
        .add_card(tamer_card("TAMER"))
        .start();

    let saved = r.place_on_field(0, "SAVE-D", None);
    let _t = r.place_on_field(0, "TAMER", None);

    let tamer_field_index = 1usize;
    let tamer_stack_before = r.game.players[0].battle_area[tamer_field_index]
        .card_sources
        .len();

    r.game.delete_permanent_with_effects(saved);

    // Mid-deletion: the Save handler parked the optional Tamer-pick and
    // the rest of `commit_permanent_deletion` was deferred. Both the
    // carrier AND the Tamer are still on field — this is the substrate
    // change that lets the parked selection's frozen `valid_action_ids`
    // map cleanly back to live permanents.
    {
        assert_eq!(
            r.game.players[0].battle_area.len(),
            2,
            "carrier + Tamer both still on field while Tamer-pick is parked \
             (deletion is deferred via pending_deletion_resume)"
        );
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Tamer-pick selection must be parked");
        assert_eq!(pending.selecting_player, 0);
        assert!(
            pending.is_optional,
            "Tamer-pick is optional (PASS = decline Save)"
        );
    }

    // Pick the Tamer (originally and still at index 1 — no shift yet).
    let action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, action).expect("pick Tamer");

    // After resolution: the saved card is under the Tamer, and the
    // carrier was finalized (removed from battle_area). Only the Tamer
    // remains on field.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "carrier finalized (removed) after the parked selection resolved"
    );
    let surviving = &r.game.players[0].battle_area[0];
    assert_eq!(
        surviving.top_card().card_id(&r.game.card_data),
        "TAMER",
        "the surviving battle-area permanent is the Tamer"
    );
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
    // SAVE-D is NOT in trash — it was placed under the Tamer.
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SAVE-D"),
        "SAVE-D is no longer in trash — it was retrieved and placed under the Tamer"
    );
    // The deferred finalizer ran — `pending_deletion_resume` is cleared.
    // (Indirectly verified: a fresh deletion would panic on the
    // single-outstanding `debug_assert!` if the slot were still set.)
}

// ─── Test 2: PASS → saved card stays in trash; deletion already happened ────

/// Decline path: the user passes on the optional Tamer-pick. Save did
/// nothing; the carrier was deleted normally and its top card stays in
/// trash with the digi-sources.
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

    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("optional Tamer-pick must be parked");
    assert!(
        pending.is_optional,
        "the Tamer-pick selection is optional — PASS declines Save"
    );

    r.game.resolve_selection(0, PASS).expect("decline Save");

    // Carrier was deleted normally; only the Tamer remains.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "carrier deleted normally; only the Tamer remains"
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
    // SAVE-D stays in trash (declined Save means the post-deletion retrieval
    // never ran).
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SAVE-D"),
        "SAVE-D stays in trash when Save was declined"
    );
}

// ─── Test 3: no own Tamers → handler returns silently; deletion ran ─────────

#[test]
fn save_with_no_tamers_does_not_protect() {
    // No Tamer on field → the candidate filter for `select_own_permanent`
    // yields zero matches; the helper returns without parking any prompt.
    // Save's handler returns immediately. Carrier was deleted normally.
    let mut r = DebugRunner::builder().add_card(save_card("SAVE-D")).start();

    let saved = r.place_on_field(0, "SAVE-D", None);

    r.game.delete_permanent_with_effects(saved);

    // No selection was parked — Save had no Tamer candidates.
    assert!(
        r.game.pending_selection.is_none(),
        "no Tamer candidates → no selection parked"
    );

    // Carrier was deleted normally.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "carrier deleted; no Tamer to save it under"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SAVE-D"),
        "SAVE-D is in trash via normal deletion"
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

    // Player 0 has zero own Tamers, so `select_own_permanent`'s filter
    // yields no candidates and no prompt is parked. Carrier was deleted
    // normally.
    assert!(
        r.game.pending_selection.is_none(),
        "no own-Tamer candidates → no selection parked"
    );

    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "carrier deleted normally — opponent's Tamer is not eligible"
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
        "SAVE-D is in trash via normal deletion"
    );
}

// ─── Test 5: no-replace path defers Save's parked Tamer-pick correctly ─────

/// Regression for the I-2 follow-up at `replacement.rs::commit_permanent_
/// deletion_no_replace`: a card hosting BOTH an optional `WhenWouldBeDeleted`
/// replacement (`<Decoy>`) AND `<Save>` on the same carrier exercises the
/// deferred-decline path. Without the `pending_selection.is_some()` guard,
/// `Player::delete_permanent` would shift the Tamer's index synchronously
/// while Save's parked Tamer-pick still references the old index, corrupting
/// the selection's `valid_action_ids`.
///
/// Sequence:
///   1. Carrier (Decoy + Save) and Tamer on field; carrier is deleted.
///   2. The optional Decoy dialog is parked at candidate-collection time.
///   3. The user PASSes — decline routes through `commit_deferred_outcome`
///      `(Zone::Trash, ReplacementOutcome::None)` → `commit_permanent_
///      deletion_no_replace`.
///   4. OnDeletion drains; Save parks an optional Tamer-pick. The defer
///      guard MUST stash `pending_deletion_resume = Some(carrier)` and
///      return without calling `delete_permanent`.
///   5. The user accepts the Tamer-pick; the Save callback places SAVE-D
///      under the Tamer; the resume hook then runs
///      `finalize_permanent_deletion` to remove the now-empty carrier.
#[test]
fn save_under_decoy_decline_defers_via_no_replace_path() {
    fn decoy_save_carrier(id: &str) -> CardData {
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
            keywords: vec![Keyword::Decoy, Keyword::Save],
            effect_class_name: id.replace('-', "_"),
            index: 0,
            norm_id: 0.0,
        }
    }

    let mut r = DebugRunner::builder()
        .add_card(decoy_save_carrier("DECOY-SAVE"))
        .add_card(tamer_card("TAMER"))
        .start();

    let carrier = r.place_on_field(0, "DECOY-SAVE", None);
    let _tamer = r.place_on_field(0, "TAMER", None);

    // Sanity: both permanents are on field at indices 0 and 1.
    assert_eq!(carrier.index, 0);
    assert_eq!(r.game.players[0].battle_area.len(), 2);

    r.game.delete_permanent_with_effects(carrier);

    // (2) The Decoy optional dialog is parked. PASS to decline.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Decoy optional dialog must be parked");
    assert!(pending.is_optional, "Decoy is optional");
    r.game.resolve_selection(0, PASS).expect("decline Decoy");

    // (4) After declining Decoy, the no-replace path ran: OnDeletion fired
    // and Save parked a Tamer-pick. The deferred guard MUST have stashed
    // the carrier handle and returned without calling delete_permanent —
    // both the carrier and the Tamer are still on field at their original
    // indices (otherwise Save's `valid_action_ids`, encoded against the
    // original layout, would now point at the wrong slot).
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "carrier + Tamer both still on field while Save's Tamer-pick is parked"
    );
    assert_eq!(
        r.game.players[0].battle_area[1]
            .top_card()
            .card_id(&r.game.card_data),
        "TAMER",
        "Tamer is still at the same index — no synchronous mid-stream delete"
    );
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Save's Tamer-pick selection must be parked");
    assert!(pending.is_optional, "Save's Tamer-pick is optional");

    // (5) Accept the Tamer-pick.
    let action = pending.valid_action_ids[0];
    r.game.resolve_selection(0, action).expect("pick Tamer");

    // Resume hook ran finalize_permanent_deletion — carrier is gone, Tamer
    // remains with SAVE-D tucked under it.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "carrier finalized after the parked Save selection resolved"
    );
    let surviving = &r.game.players[0].battle_area[0];
    assert_eq!(surviving.top_card().card_id(&r.game.card_data), "TAMER",);
    assert_eq!(
        surviving.card_sources[0].card_id(&r.game.card_data),
        "DECOY-SAVE",
        "DECOY-SAVE was placed at the bottom of the Tamer's stack"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DECOY-SAVE"),
        "DECOY-SAVE was retrieved from trash by Save's post-deletion handler"
    );
}
