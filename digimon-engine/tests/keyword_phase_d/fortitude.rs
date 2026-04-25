//! Phase D Task 8 — `Keyword::Fortitude` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Fortitude]` (no hand-rolled
//! `CardEffect`) must, when self is deleted with at least one digivolution
//! source under the top:
//!   1. Fire as a TRIGGER (not a replacement). The carrier's deletion is
//!      NOT cancelled — the carrier moves to trash with its sources, then
//!      Fortitude plays the carrier's top card back from trash, free,
//!      unsuspended, with ETB triggers active.
//!   2. Gate: `card_sources.len() >= 2` at deletion time (top + ≥1 source).
//!      When the gate fails, no replay fires; carrier stays in trash.
//!   3. Self-scope: when a NEIGHBORING permanent is deleted, the Fortitude
//!      carrier's auto-install body MUST NOT fire on that neighbor's
//!      deletion (Fortitude is "when self is deleted", not "when an ally
//!      is deleted").
//!
//! Mirrors DCGO `Fortitude.cs:14-63`. DCGO `CanActivateFortitude` requires:
//!   - `IsExistOnTrash(card)` — the card lives in trash at fire-time.
//!   - The deleted stack contained `card`
//!     (`CardStack.Contains(card)`).
//!   - The deleted stack had `CardSources.Count >= 1` (≥1 digi source under
//!     the top — translates to `card_sources.len() >= 2` in Rust where
//!     index 0 = bottom and last = top).
//!   - `CanPlayAsNewPermanent` — i.e., new-permanent slot available.
//!
//! ## Rust timing
//!
//! `OnDeletion` in Rust fires BEFORE `delete_permanent` (the carrier is
//! still in `battle_area` with its `card_sources` intact). `OnAnyDeletion`
//! fires AFTER `delete_permanent` for surviving permanents only — the
//! just-deleted card is in trash and is NOT scanned by the
//! `PlayerBattleArea` enumeration.
//!
//! Phase D Task 8 substrate: the OnDeletion handler captures the gate-check
//! result + self_card and stashes it in `Game.pending_post_deletion_replays`.
//! `finalize_permanent_deletion` drains the slot AFTER `delete_permanent`
//! moves the card to trash but BEFORE the global `OnAnyDeletion` broadcast,
//! so:
//!   - `play_from_trash_free_unsuspended` finds the card in trash.
//!   - Subsequent `OnAnyDeletion` observers see the replayed card on field.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

fn fortitude_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(5000),
        play_cost: 5,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        // Printed-only Fortitude: the auto-install MUST be the sole source
        // of behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::Fortitude],
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
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
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Append `card_id` on top of a field permanent's digivolution stack so it
/// becomes the new visible top (since `top_card() = card_sources.last()`).
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

// ─── Test 1: happy path — self deleted with 1 source → replays self from trash ──

/// Stack: [SRC-A, FORTITUDE]. Opponent deletes the carrier. Gate passes
/// (`card_sources.len() == 2`). Fortitude must:
///   - Let the carrier deletion proceed normally (move to trash).
///   - Then play FORTITUDE back from trash, free, unsuspended.
///   - The replayed permanent has a fresh stack (no sources under it).
///   - Memory does NOT change (free play).
#[test]
fn fortitude_replays_self_from_trash_when_self_deleted_with_source() {
    let mut r = DebugRunner::builder()
        .add_card(fortitude_card("FORTITUDE"))
        .add_card(plain_digimon("SRC-A"))
        .start();

    // Build the carrier with 1 source under it.
    // Stack: [SRC-A, FORTITUDE]  (index 0 = bottom, last = top)
    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "FORTITUDE");
    {
        let perm = &r.game.players[0].battle_area[carrier.index as usize];
        assert_eq!(perm.card_sources.len(), 2, "preconditions: 2-card stack");
        assert_eq!(
            perm.top_card().card_id(&r.game.card_data),
            "FORTITUDE",
            "FORTITUDE is the visible top"
        );
    }

    let memory_before = r.game.memory;

    r.game.delete_permanent_with_effects(carrier);

    // No selection should be parked — Fortitude is mandatory and
    // synchronous (no player choice).
    assert!(
        r.game.pending_selection.is_none(),
        "Fortitude must not install any pending_selection — got {:?}",
        r.game.pending_selection.as_ref().map(|s| &s.kind),
    );

    // Carrier finalized: SRC-A moved to trash. FORTITUDE was in trash but
    // got replayed from there onto the field.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "FORTITUDE replayed itself onto the field"
    );
    let surviving = &r.game.players[0].battle_area[0];
    assert_eq!(
        surviving.top_card().card_id(&r.game.card_data),
        "FORTITUDE",
        "FORTITUDE is the top of the replayed permanent"
    );
    assert_eq!(
        surviving.card_sources.len(),
        1,
        "replayed permanent has a fresh stack — no sources under top"
    );
    assert!(
        !surviving.is_suspended,
        "Fortitude replay is unsuspended (DCGO Fortitude.cs:54-63 \
         `isTapped: false`)"
    );

    // SRC-A landed in trash via normal deletion; FORTITUDE is NOT in trash
    // (it was retrieved by the Fortitude replay).
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SRC-A"),
        "SRC-A in trash via normal deletion"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "FORTITUDE"),
        "FORTITUDE NOT in trash — Fortitude replay retrieved it"
    );

    // Free play: memory does not change due to the replay.
    assert_eq!(
        r.game.memory, memory_before,
        "Fortitude replay is free — memory must not change"
    );
}

// ─── Test 2: gate fail — no source under top → no replay → stays in trash ──

/// Stack: [FORTITUDE] (just the top, no source under it). When deleted, the
/// gate `card_sources.len() >= 2` fails. Fortitude must NOT replay; the
/// carrier stays in trash.
#[test]
fn fortitude_does_not_fire_when_no_source_was_under_self() {
    let mut r = DebugRunner::builder()
        .add_card(fortitude_card("FORTITUDE"))
        .start();

    // Single-card stack: just FORTITUDE on top, no source under.
    let carrier = r.place_on_field(0, "FORTITUDE", None);
    {
        let perm = &r.game.players[0].battle_area[carrier.index as usize];
        assert_eq!(perm.card_sources.len(), 1, "preconditions: only top, no source");
    }

    r.game.delete_permanent_with_effects(carrier);

    // No replay: gate failed (need ≥1 source under top). Carrier stays in
    // trash; battle_area is empty.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "FORTITUDE deleted normally — no replay when gate fails"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "FORTITUDE"),
        "FORTITUDE stays in trash via normal deletion"
    );
}

// ─── Test 3: self-scope — Fortitude does NOT fire on neighbor's deletion ───

/// Two permanents: FORTITUDE on field with 1 source, plus a NEIGHBOR with
/// no Fortitude. Deleting the NEIGHBOR must NOT trigger FORTITUDE's
/// auto-install — Fortitude is a self-trigger, not an ally observer.
#[test]
fn fortitude_does_not_fire_on_neighbor_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(fortitude_card("FORTITUDE"))
        .add_card(plain_digimon("FORT-SRC"))
        .add_card(plain_digimon("NEIGHBOR"))
        .add_card(plain_digimon("NEIGH-SRC"))
        .start();

    // Build FORTITUDE with 1 source.
    let fortitude = r.place_on_field(0, "FORT-SRC", None);
    push_source_card(&mut r, 0, fortitude.index as usize, "FORTITUDE");
    // Build NEIGHBOR with 1 source (so neighbor's own deletion isn't gate-failed
    // for some other reason).
    let neighbor = r.place_on_field(0, "NEIGH-SRC", None);
    push_source_card(&mut r, 0, neighbor.index as usize, "NEIGHBOR");
    {
        let f = &r.game.players[0].battle_area[fortitude.index as usize];
        let n = &r.game.players[0].battle_area[neighbor.index as usize];
        assert_eq!(f.top_card().card_id(&r.game.card_data), "FORTITUDE");
        assert_eq!(n.top_card().card_id(&r.game.card_data), "NEIGHBOR");
    }

    let fortitude_in_trash_before = r.game.players[0]
        .trash
        .iter()
        .any(|c| c.card_id(&r.game.card_data) == "FORTITUDE");
    assert!(
        !fortitude_in_trash_before,
        "preconditions: FORTITUDE is on field, not in trash"
    );

    r.game.delete_permanent_with_effects(neighbor);

    // NEIGHBOR was deleted normally; FORTITUDE is untouched on field.
    let surviving_ids: Vec<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    assert_eq!(
        surviving_ids,
        vec!["FORTITUDE".to_string()],
        "only FORTITUDE remains; NEIGHBOR is gone — Fortitude must not have replayed anything"
    );
    // FORTITUDE on the surviving permanent must STILL have its original
    // source under it (i.e., the original stack). If Fortitude had wrongly
    // fired on the neighbor's deletion, it would have replayed FORTITUDE
    // from somewhere (but FORTITUDE wasn't in trash, so it couldn't —
    // however we still validate stack invariants).
    let f = &r.game.players[0].battle_area[0];
    assert_eq!(
        f.card_sources.len(),
        2,
        "FORTITUDE's stack is undisturbed by neighbor deletion"
    );
    assert!(
        !r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "FORTITUDE"),
        "FORTITUDE never went to trash"
    );
    // NEIGHBOR + NEIGH-SRC went to trash via normal deletion.
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "NEIGHBOR"),
        "NEIGHBOR landed in trash via normal deletion"
    );
}
