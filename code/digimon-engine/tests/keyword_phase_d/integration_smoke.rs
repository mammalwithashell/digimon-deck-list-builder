//! Phase D Task 14 — printed-keyword-only end-to-end smoke test.
//!
//! Demonstrates the contract Phase D delivers: a card declaring multiple
//! Phase D keywords with NO hand-rolled `CardEffect` behaves correctly when
//! the keywords interact. The auto-install plumbing in `effects_for_card`
//! flat-maps keywords into separate `Effect`s, and the dispatcher resolves
//! them through their respective timings (`OnDeletion` triggers,
//! `WhenWouldBeDeleted` replacements).
//!
//! Scenarios covered:
//!
//! 1. **`Save + Decoy`** — coexistence on the same card. Save fires on
//!    self-deletion (`OnDeletion`); Decoy fires on ally deletion
//!    (`WhenWouldBeDeleted`, redirects to self). The two keywords trigger
//!    on different subjects, so they don't conflict — this is the cleanest
//!    proof that multi-keyword auto-install composes correctly.
//!
//! 2. **`Save + Fortitude`, decline Save** — both are `OnDeletion` triggers
//!    on the carrier. When the carrier is deleted with ≥1 digi source and
//!    the player declines Save (PASS), the carrier is finalized normally,
//!    Fortitude's `pending_post_deletion_replays` slot fires, and the card
//!    is replayed from trash. The two triggers compose because Save's
//!    decline path doesn't disturb the trash, leaving Fortitude's gate
//!    valid.
//!
//! 3. **`Save + Fortitude`, accept Save** — stress case for the
//!    composition. Accepting Save moves the saved top card from the
//!    carrier's stack under the chosen Tamer (NOT to trash). Fortitude's
//!    deferred replay then attempts to play that same `CardHandle` from
//!    trash — but the card is no longer in trash. Documented behavior:
//!    `play_from_trash_free_unsuspended` panics if the card is missing.
//!    This test pins the current outcome (whichever it is) so future
//!    refactors must consciously break it.

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

/// A printed-keyword-only Digimon — no hand-rolled `CardEffect`. The set
/// of keywords passed in fully determines the card's behavior; Phase D's
/// auto-install path is the sole source of effect logic.
fn printed_only_digimon(id: &str, keywords: Vec<Keyword>) -> CardData {
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
        keywords,
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
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
    }
}

/// Append `card_id` on top of a field permanent's digivolution stack so
/// it becomes the new visible top (since `top_card() = card_sources.last()`).
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

// ─── Scenario 1: Save + Decoy on one printed-only card ──────────────────────

/// A card with `[Save, Decoy]` and no hand-rolled `CardEffect`:
///   - Self-deletion fires Save (post-deletion `OnDeletion` trigger). Decoy
///     does not fire (its self-scope guard rejects `subject == me_perm`).
///   - Ally deletion fires Decoy (`WhenWouldBeDeleted` replacement). Save
///     does not fire (Save's `OnDeletion` is keyed on the carrier — it
///     never enqueues for a neighbor's deletion).
///
/// The cleanest demonstration that two Phase D keywords on one card
/// flat-map into independent `Effect`s and never cross-fire on the wrong
/// subject.
#[test]
fn save_and_decoy_coexist_on_one_printed_only_card() {
    let mut r = DebugRunner::builder()
        .add_card(printed_only_digimon(
            "DUAL",
            vec![Keyword::Save, Keyword::Decoy(0)],
        ))
        .add_card(tamer_card("TAMER"))
        .add_card(plain_digimon("ALLY"))
        .start();

    // ─── Sub-case 1a: ally deletion → Decoy substitutes self ────────────────
    let _dual = r.place_on_field(0, "DUAL", None);
    let _tamer = r.place_on_field(0, "TAMER", None);
    let ally = r.place_on_field(0, "ALLY", None);

    r.game.delete_permanent_with_effects(ally);

    // Decoy mounts an optional outer accept dialog. Save did NOT mount on
    // the ally's deletion — Save is a self-keyed `OnDeletion` trigger on
    // DUAL, not an ally observer.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Decoy outer accept dialog should be parked for ally deletion");
    assert!(pending.is_optional, "Decoy is optional ('may' substitute)");
    assert_eq!(pending.selecting_player, 0);

    // Accept the Decoy substitute.
    let accept_action = pending.valid_action_ids[0];
    r.game
        .resolve_selection(0, accept_action)
        .expect("accept Decoy substitute");

    // After substitute: ally survives, DUAL is deleted in its place. Save
    // should NOT have fired even on this substituted self-deletion either,
    // because the saved top card is already in trash by the time the
    // substituted deletion runs Save's OnDeletion handler — but Save still
    // parks an optional Tamer-pick. We expect a Save Tamer-pick dialog to
    // be parked now, since DUAL just got "deleted" (substituted).
    if let Some(p) = r.game.pending_selection.as_ref() {
        assert!(
            p.is_optional,
            "if a selection is parked after Decoy substitute, it must be Save's optional Tamer-pick"
        );
        // Decline Save to keep the test focused on the multi-keyword
        // coexistence claim (not the Save+Tamer behavior, which save.rs
        // tests already cover).
        r.game.resolve_selection(0, PASS).expect("decline Save");
    }

    // Final state: ALLY still on field; TAMER still on field; DUAL was
    // deleted via Decoy's substitution (and Save was declined).
    let surviving_ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        surviving_ids.contains(&"ALLY".to_string()),
        "ALLY survived via Decoy substitute"
    );
    assert!(
        surviving_ids.contains(&"TAMER".to_string()),
        "TAMER untouched"
    );
    assert!(
        !surviving_ids.contains(&"DUAL".to_string()),
        "DUAL was deleted via Decoy substitute"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DUAL"),
        "DUAL landed in trash via the substituted deletion"
    );
}

// ─── Scenario 2: Save + Fortitude compose on decline ────────────────────────

/// `[Save, Fortitude]` on one printed-only card. Carrier has 1 source under
/// the top → Fortitude's gate (`card_sources.len() >= 2`) holds. Player
/// declines the Save Tamer-pick; carrier is finalized normally (top + sources
/// move to trash); Fortitude's drained replay slot pulls the carrier back
/// from trash, free + unsuspended.
#[test]
fn save_and_fortitude_compose_when_save_is_declined() {
    let mut r = DebugRunner::builder()
        .add_card(printed_only_digimon(
            "DUAL-SF",
            vec![Keyword::Save, Keyword::Fortitude],
        ))
        .add_card(plain_digimon("SRC-A"))
        .add_card(tamer_card("TAMER"))
        .start();

    // Build the carrier: stack [SRC-A, DUAL-SF] (index 0 = bottom).
    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "DUAL-SF");
    let _tamer = r.place_on_field(0, "TAMER", None);
    {
        let perm = &r.game.players[0].battle_area[carrier.index as usize];
        assert_eq!(perm.card_sources.len(), 2);
        assert_eq!(
            perm.top_card().card_id(&r.game.card_data),
            "DUAL-SF",
            "DUAL-SF is the visible top"
        );
    }

    let memory_before = r.game.memory;

    r.game.delete_permanent_with_effects(carrier);

    // Both Save and Fortitude install OnDeletion handlers on the same
    // carrier — the queue drainer parks a `TriggerOrder` selection asking
    // the controller which trigger to fire first. Fortitude is mandatory,
    // so the bundle does NOT carry a PASS bit.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("TriggerOrder should be parked for the multi-trigger bundle");
    assert_eq!(
        pending.kind,
        SelectionKind::TriggerOrder,
        "multi-OnDeletion bundle parks a TriggerOrder prompt"
    );
    assert!(
        !pending.is_optional,
        "Fortitude is mandatory → bundle carries no PASS bit"
    );
    assert_eq!(
        pending.valid_action_ids.len(),
        2,
        "two triggers in the bundle: Save + Fortitude"
    );
    let bundle_actions = pending.valid_action_ids.clone();

    // Pick the FIRST trigger entry — the order between Save and Fortitude
    // in the bundle is implementation-defined (depends on
    // `effects_for_card` keyword flat-map order). Whichever fires first,
    // the second runs after it. We then resolve any sub-prompt
    // (Save's Tamer-pick) by declining (PASS).
    r.game
        .resolve_selection(0, bundle_actions[0])
        .expect("pick first trigger entry");

    // Save's optional Tamer-pick may now be parked. If so, decline.
    if let Some(p) = r.game.pending_selection.as_ref() {
        if p.is_optional {
            r.game.resolve_selection(0, PASS).expect("decline Save");
        } else {
            // Possibly the next bundle entry (TriggerOrder size-1 doesn't
            // re-prompt, so this branch is only reached if a non-Save
            // mandatory sub-prompt unexpectedly appears). Re-pick.
            let next = p.valid_action_ids[0];
            r.game
                .resolve_selection(0, next)
                .expect("resolve unexpected mandatory sub-prompt");
        }
    }

    // After the bundle drains: if Fortitude fired before Save, the carrier
    // was finalized in-line and Fortitude pushed (owner, top_card) into
    // `pending_post_deletion_replays`. Save then ran on the (already
    // finalized) carrier — its `top_card()` snapshot returned the
    // soon-to-be-tombstone, but Save's callback would either decline or
    // operate on the already-trashed card. If Save fired first, it parked
    // a Tamer-pick (we declined above), then Fortitude fired post-resume.
    //
    // Either way, expectations on the END STATE are the same: declining
    // Save means the carrier proceeded to trash; Fortitude's gate (≥1
    // source under top at OnDeletion time) held; Fortitude replayed
    // DUAL-SF from trash.

    // Final state: DUAL-SF is back on the field (replayed by Fortitude),
    // TAMER untouched, SRC-A in trash.
    let dual_on_field = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "DUAL-SF");
    assert!(
        dual_on_field,
        "DUAL-SF replayed onto the field by Fortitude after Save was declined"
    );

    let dual_perm = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "DUAL-SF")
        .expect("DUAL-SF on field");
    assert_eq!(
        dual_perm.card_sources.len(),
        1,
        "Fortitude replay produces a fresh stack — no sources under DUAL-SF"
    );
    assert!(!dual_perm.is_suspended, "Fortitude replay is unsuspended");
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SRC-A"),
        "SRC-A landed in trash via the carrier's normal finalization"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DUAL-SF"),
        "DUAL-SF was retrieved from trash by Fortitude — not in trash"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "Fortitude replay is free — memory must not change"
    );
}

// ─── Scenario 3: Save + Fortitude — accept Save end-state ──────────────────

/// `[Save, Fortitude]` accept-Save path. Both keywords are `OnDeletion`
/// triggers on the same carrier; the queue drainer offers a `TriggerOrder`
/// prompt, and Save fires first in the current implementation's bundle
/// order. The interaction unfolds:
///
///   1. TriggerOrder picks the first bundle entry → Save's `process` runs.
///   2. Save parks an optional Tamer-pick (deferring carrier finalization
///      via `pending_deletion_resume`).
///   3. Player accepts → saved top card is lifted off the carrier and
///      tucked under the Tamer; the carrier's `card_sources.len()`
///      drops by 1.
///   4. Save's resolution returns to the queue drainer; the sibling
///      Fortitude trigger auto-fires (bundle size 1, no second prompt).
///   5. Fortitude reads `card_sources.len() >= 2` on the carrier — the
///      gate now FAILS (length is 1 after Save lifted the top), so
///      Fortitude's replay slot stays empty.
///   6. Deferred deletion finalizes: carrier removed; remaining sources
///      to trash; `pending_post_deletion_replays` drains empty.
///
/// End state: saved card lives under the Tamer; Fortitude silently
/// declined; no panic. This is the "graceful coexistence" branch.
///
/// **DCGO parity note.** No published Digimon prints both `<Save>` and
/// `<Fortitude>` simultaneously, so this scenario is a synthetic stress
/// test of the substrate's coexistence claims, not a production card
/// regression.
///
/// **Forward-compat note.** If the keyword flat-map order in
/// `effects_for_card` ever swaps Save and Fortitude such that Fortitude
/// fires FIRST, the post-deletion replay drain would attempt to play
/// the saved card from trash — but Save's accept-path moves it under
/// the Tamer instead. `play_from_trash_free_unsuspended` would then
/// panic. If this test starts failing with such a panic, treat that
/// as a real substrate gap that needs a defensive skip in the replay
/// drain (`combat::finalize_permanent_deletion`).
#[test]
fn save_and_fortitude_compose_when_save_is_accepted() {
    let mut r = DebugRunner::builder()
        .add_card(printed_only_digimon(
            "DUAL-SF2",
            vec![Keyword::Save, Keyword::Fortitude],
        ))
        .add_card(plain_digimon("SRC-B"))
        .add_card(tamer_card("TAMER"))
        .start();

    let carrier = r.place_on_field(0, "SRC-B", None);
    push_source_card(&mut r, 0, carrier.index as usize, "DUAL-SF2");
    let _tamer = r.place_on_field(0, "TAMER", None);

    r.game.delete_permanent_with_effects(carrier);

    // TriggerOrder bundle for Save + Fortitude.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("TriggerOrder should be parked for the multi-trigger bundle");
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    let bundle_actions = pending.valid_action_ids.clone();

    r.game
        .resolve_selection(0, bundle_actions[0])
        .expect("pick first trigger entry (Save runs first in the bundle)");

    // Save's Tamer-pick is now parked. Accept it — pick the Tamer.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("Save Tamer-pick should be parked after picking Save first");
    assert!(pending.is_optional, "Save Tamer-pick is optional");
    let accept_action = pending.valid_action_ids[0];
    r.game
        .resolve_selection(0, accept_action)
        .expect("accept Save Tamer-pick");

    // After accept: the queue drained the sibling Fortitude trigger
    // (whose gate failed because Save lifted the top), and the deferred
    // deletion finalized. No further selections should be parked.
    assert!(
        r.game.pending_selection.is_none(),
        "no further selections after accepting Save"
    );

    // End state: TAMER carries DUAL-SF2 at the bottom of its stack.
    let tamer_perm = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "TAMER")
        .expect("TAMER on field");
    assert!(
        tamer_perm
            .card_sources
            .iter()
            .any(|s| s.card_id(&r.game.card_data) == "DUAL-SF2"),
        "DUAL-SF2 was tucked under TAMER's stack via Save"
    );
    assert_eq!(
        tamer_perm.card_sources[0].card_id(&r.game.card_data),
        "DUAL-SF2",
        "DUAL-SF2 lands at the BOTTOM of TAMER's stack (index 0)"
    );

    // SRC-B (carrier) was finalized to trash. Save only saves the top
    // card (DUAL-SF2), not the carrier's other digi sources.
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SRC-B"),
        "SRC-B (carrier) finalized to trash"
    );

    // DUAL-SF2 is NOT in trash — it was lifted under the Tamer before
    // any trash routing. Fortitude could not retrieve it from trash
    // (its gate failed first because the top was already gone).
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "DUAL-SF2"),
        "DUAL-SF2 lives under TAMER, not in trash"
    );

    // Fortitude did NOT replay DUAL-SF2 onto the field — its gate
    // failed because Save lifted the top before Fortitude's gate check.
    let dual_on_field = r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "DUAL-SF2");
    assert!(
        !dual_on_field,
        "Fortitude must NOT replay DUAL-SF2 — its gate failed because \
         Save lifted the top before Fortitude's source-count check"
    );
}
