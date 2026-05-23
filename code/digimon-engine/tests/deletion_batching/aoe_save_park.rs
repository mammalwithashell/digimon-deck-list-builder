//! Regression: AoE Option deletes two `<Save>` permanents simultaneously
//! and BOTH OnDeletion handlers park selections. Before the
//! `align-deletion-with-dcgo-model` change this tripped the
//! single-occupancy `debug_assert!` at
//! `commit_permanent_deletion` / `commit_permanent_deletion_no_replace`
//! (`G-DELETION-RESUME-NESTED`). Under the batched flow the parks are
//! handled in sequence via the active-batch state machine — no panic.
//!
//! See `openspec/specs/permanent-deletion-semantics/spec.md` §"Multiple
//! OnDeletion-parking permanents in one batch resolve sequentially".

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

/// Resolve the currently parked selection. If it's a `TriggerOrder`
/// bundle, pick the first listed entry; otherwise apply `default_action`
/// (typically `PASS` for "decline Save" cases or a valid_action_ids[0]
/// for "accept").
fn resolve_with(r: &mut DebugRunner, default_action: u16) -> bool {
    let Some(pending) = r.game.pending_selection.as_ref() else {
        return false;
    };
    let action = match pending.kind {
        SelectionKind::TriggerOrder => pending.valid_action_ids[0],
        _ => default_action,
    };
    r.game
        .resolve_selection(pending.selecting_player, action)
        .expect("resolve_with: action must be valid for the current selection");
    true
}

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
        keywords: vec![Keyword::Save],
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
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
        also_treated_as: Vec::new(),
    }
}

/// Two `<Save>` permanents deleted as a single batch. Both handlers park
/// optional Tamer-pick selections. The active-batch resume mechanism
/// sequences them: park 1 → resolve → drain continues → park 2 → resolve
/// → batch finalizes. NO panic on the `pending_deletion_resume` slot
/// (which has been retired entirely).
#[test]
fn aoe_delete_two_save_permanents_both_park_sequentially() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-1"))
        .add_card(save_card("SAVE-2"))
        .add_card(tamer_card("TAMER"))
        .start();

    let save1 = r.place_on_field(0, "SAVE-1", None);
    let save2 = r.place_on_field(0, "SAVE-2", None);
    let _tamer = r.place_on_field(0, "TAMER", None);

    // Sanity: 3 permanents on field at indices 0, 1, 2.
    assert_eq!(r.game.players[0].battle_area.len(), 3);

    // Delete both Save permanents in a single batch (DCGO board-wipe
    // pattern). This is exactly the call shape that previously panicked.
    r.game
        .delete_permanents_batch(vec![save1, save2], ReplacementCause::OpponentEffect);

    // The drainer parks a `TriggerOrder` bundle first because two
    // OnDeletion entries belong to the same controller. Resolve by
    // picking the first listed entry. This runs Save handler #1, which
    // parks a Tamer-pick.
    assert_eq!(
        r.game.pending_selection.as_ref().map(|s| s.kind),
        Some(SelectionKind::TriggerOrder),
        "TriggerOrder bundle parks first for the dual Save OnDeletion drain"
    );
    let trigger_action = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game
        .resolve_selection(0, trigger_action)
        .expect("pick first Save in TriggerOrder bundle");

    // Park 1: first Save handler installed an optional Tamer-pick.
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("first Save's Tamer-pick parked after TriggerOrder pick");
    assert!(pending.is_optional, "Save's Tamer-pick is optional");
    let action_for_first = pending.valid_action_ids[0];

    // Accept the first Tamer-pick.
    r.game
        .resolve_selection(0, action_for_first)
        .expect("accept first Save's Tamer-pick");

    // Park 2: the second Save handler now runs (no further TriggerOrder
    // prompt — the bundle had 2 entries and we just resolved the first).
    let pending2 = r
        .game
        .pending_selection
        .as_ref()
        .expect("second Save handler parked after first Save resolved");
    assert!(
        pending2.is_optional,
        "second Save's Tamer-pick is also optional"
    );

    // Decline the second one (PASS).
    r.game
        .resolve_selection(0, PASS)
        .expect("decline second Save");

    // No further selections — the batch has finalized.
    assert!(
        r.game.pending_selection.is_none(),
        "no pending selection after both Save resolutions; batch is complete"
    );

    // End state: both SAVE-* permanents are off the field (carriers
    // trashed). The Tamer is still on field with one of the Save cards
    // tucked under it (the accepted one). The other Save card is in
    // trash (the declined one).
    //
    // We don't assert which Save went under the Tamer because trigger
    // order is implementation-defined; the important invariant is "no
    // panic and the batch processes both."
    let field_card_ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        field_card_ids,
        vec!["TAMER".to_string()],
        "only the Tamer remains on field"
    );

    let tamer_perm = &r.game.players[0].battle_area[0];
    assert_eq!(
        tamer_perm.card_sources.len(),
        2,
        "Tamer carries one extra source — the saved Digimon from the accept path"
    );

    // Exactly one SAVE-* card lives under the Tamer; the other is in
    // trash. Verify by counting across both zones.
    let trash_save_count = r.game.players[0]
        .trash
        .iter()
        .filter(|c| {
            let id = c.card_id(&r.game.card_data);
            id == "SAVE-1" || id == "SAVE-2"
        })
        .count();
    let under_tamer_save_count = tamer_perm
        .card_sources
        .iter()
        .filter(|s| {
            let id = s.card_id(&r.game.card_data);
            id == "SAVE-1" || id == "SAVE-2"
        })
        .count();
    assert_eq!(
        trash_save_count + under_tamer_save_count,
        2,
        "both Save cards accounted for (one under Tamer, one in trash)"
    );
    assert_eq!(under_tamer_save_count, 1, "exactly one was saved");
    assert_eq!(trash_save_count, 1, "exactly one was declined → in trash");
}

/// Both Save handlers decline. Both carriers end up in trash; Tamer is
/// untouched. No panic, no leftover selections.
#[test]
fn aoe_delete_two_save_permanents_both_declined() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-1"))
        .add_card(save_card("SAVE-2"))
        .add_card(tamer_card("TAMER"))
        .start();

    let save1 = r.place_on_field(0, "SAVE-1", None);
    let save2 = r.place_on_field(0, "SAVE-2", None);
    let _tamer = r.place_on_field(0, "TAMER", None);

    r.game
        .delete_permanents_batch(vec![save1, save2], ReplacementCause::OpponentEffect);

    // Resolve all selections in sequence with PASS where allowed. The
    // helper auto-resolves TriggerOrder bundles by picking entry 0.
    // Loop is bounded by the expected number of prompts (4: TriggerOrder,
    // park 1, park 2 — and any extras would indicate a regression).
    let mut prompts_seen = 0;
    while resolve_with(&mut r, PASS) {
        prompts_seen += 1;
        assert!(
            prompts_seen <= 6,
            "selection-resolve loop ran more than 6 iterations; \
             this indicates the batch isn't progressing"
        );
    }

    // Batch finalized.
    assert!(r.game.pending_selection.is_none(), "batch complete");
    assert!(prompts_seen >= 2, "at least 2 prompts were processed");

    // Both Save cards in trash; Tamer's stack unchanged.
    let trash_save_count = r.game.players[0]
        .trash
        .iter()
        .filter(|c| {
            let id = c.card_id(&r.game.card_data);
            id == "SAVE-1" || id == "SAVE-2"
        })
        .count();
    assert_eq!(
        trash_save_count, 2,
        "both declined Save cards landed in trash"
    );
    let tamer_perm = &r.game.players[0].battle_area[0];
    assert_eq!(
        tamer_perm.card_sources.len(),
        1,
        "Tamer stack untouched (no Save accepted)"
    );
}

/// Three `<Save>` permanents deleted in one batch — all park, all decline,
/// no panic. Validates that the active-batch resume handles N>2 parking
/// permanents (the stack-depth scaling case).
#[test]
fn aoe_delete_three_save_permanents_all_park_in_sequence() {
    let mut r = DebugRunner::builder()
        .add_card(save_card("SAVE-1"))
        .add_card(save_card("SAVE-2"))
        .add_card(save_card("SAVE-3"))
        .add_card(tamer_card("TAMER"))
        .start();

    let s1 = r.place_on_field(0, "SAVE-1", None);
    let s2 = r.place_on_field(0, "SAVE-2", None);
    let s3 = r.place_on_field(0, "SAVE-3", None);
    let _t = r.place_on_field(0, "TAMER", None);

    r.game
        .delete_permanents_batch(vec![s1, s2, s3], ReplacementCause::OpponentEffect);

    // Use the auto-resolve helper: it handles TriggerOrder by picking
    // entry 0 and declines optional Save prompts via PASS. Loop terminates
    // when no further selections are parked.
    let mut prompts_seen = 0;
    while resolve_with(&mut r, PASS) {
        prompts_seen += 1;
        assert!(
            prompts_seen <= 8,
            "selection-resolve loop ran more than 8 iterations; \
             this indicates the batch isn't progressing"
        );
    }

    assert!(
        r.game.pending_selection.is_none(),
        "batch complete after all resolves"
    );
    assert!(prompts_seen >= 3, "at least 3 prompts were processed");

    let trash_save_count = r.game.players[0]
        .trash
        .iter()
        .filter(|c| {
            let id = c.card_id(&r.game.card_data);
            id == "SAVE-1" || id == "SAVE-2" || id == "SAVE-3"
        })
        .count();
    assert_eq!(trash_save_count, 3, "all three Save cards in trash");
}
