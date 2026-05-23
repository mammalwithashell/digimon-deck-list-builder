//! Phase D Task 9 — `Keyword::Partition` auto-install behavioral tests.
//!
//! A card declaring ONLY `keywords: vec![Keyword::Partition]` (no hand-rolled
//! `CardEffect`) installs an `OnDeletion` trigger that mirrors DCGO
//! `Partition.cs:9-23` and `Partition.cs:71-162`:
//!
//!   - Trigger fires when the carrier leaves the battle area, gated on
//!     cause: NOT `Battle` and NOT a same-controller (`OwnerEffect`) effect
//!     (DCGO `Partition.cs:14-19`).
//!   - When fired with `card_sources.len() >= 3` (top + ≥2 sources), the
//!     handler parks an exactly-2 selection over the carrier's digivolution
//!     sources (`CountCappedZone::Material(carrier)`). Color filtering is
//!     out of Phase D scope — the auto-install offers any source.
//!   - On selection, the two picked sources are stashed into
//!     `pending_post_deletion_replays` so they replay from trash via
//!     `play_from_trash_free_unsuspended` AFTER `delete_permanent` moves
//!     the carrier + sources to trash, but BEFORE the global
//!     `OnAnyDeletion` broadcast.
//!   - When `card_sources.len() < 3`, the gate fails and Partition does
//!     not park (deletion proceeds normally with no replay).
//!
//! ## Cause filter rationale
//!
//! DCGO `Partition.cs:14-19`:
//! ```
//! if (CanTriggerWhenPermanentRemoveField(...))
//!     if (!IsByBattle(...))
//!         if (!IsByEffect(..., cardEffect => IsOwnerEffect(cardEffect, card)))
//!             return true;
//! ```
//!
//! So Partition fires when the carrier leaves the field via:
//!   - Opponent's effect (`ReplacementCause::OpponentEffect`)
//!   - `SecurityCheck` (defensive, rare in practice)
//!   - `Cost` (defensive, rare)
//!
//! Partition does NOT fire when the carrier is deleted by:
//!   - Own effect (`ReplacementCause::OwnEffect`)
//!   - Battle (`ReplacementCause::Battle`)
//!
//! ## Color grouping (out of scope)
//!
//! Per-card-text color groupings (`firstSources` / `secondSources`) come
//! from `partitionConditions: List<PartitionCondition>` injected by the
//! card factory in DCGO. The auto-install can't know color groups from
//! `Keyword::Partition` alone — Phase D offers ANY same-stack source for
//! each pick. Per-card overrides apply color grouping via hand-rolled
//! `CardEffect`.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

fn partition_card(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(6),
        dp: Some(7000),
        play_cost: 7,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        // Printed-only Partition: the auto-install MUST be the sole source
        // of behavior. No hand-rolled CardEffect is registered.
        keywords: vec![Keyword::Partition],
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

/// Append `card_id` on top of a field permanent's digivolution stack so it
/// becomes the new visible top.
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

// ─── Test 1: opponent-effect deletion → Partition picks 2 sources, plays both ──

/// Stack: [SRC-A, SRC-B, SRC-C, PARTITION] (index 0 = bottom, last = top).
/// Opponent's effect deletes the carrier. Cause = `OpponentEffect` →
/// Partition trigger fires. The handler parks a 2-pick selection over
/// the carrier's three sources. After the player picks 2:
///   - The carrier (PARTITION + all three sources) finalizes to trash.
///   - The 2 picked sources replay back to the field, free, unsuspended,
///     each in its own permanent slot.
#[test]
fn partition_plays_two_picked_sources_on_opponent_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(partition_card("PARTITION"))
        .add_card(plain_digimon("SRC-A"))
        .add_card(plain_digimon("SRC-B"))
        .add_card(plain_digimon("SRC-C"))
        .start();

    // Build the carrier with 3 sources. Bottom-up:
    //   index 0 = SRC-A, 1 = SRC-B, 2 = SRC-C, 3 = PARTITION (top).
    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-B");
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-C");
    push_source_card(&mut r, 0, carrier.index as usize, "PARTITION");
    {
        let perm = &r.game.players[0].battle_area[carrier.index as usize];
        assert_eq!(perm.card_sources.len(), 4, "preconditions: 4-card stack");
        assert_eq!(
            perm.top_card().card_id(&r.game.card_data),
            "PARTITION",
            "PARTITION is the visible top"
        );
    }

    // Simulate "opponent's effect is currently resolving" so that
    // `infer_deletion_cause` returns `OpponentEffect` (not `OwnEffect`).
    // Carrier is owned by player 0; effect_source_player = 1 → opponent.
    r.game.set_effect_source_player_for_test(Some(1));

    let memory_before = r.game.memory;

    r.game.delete_permanent_with_effects(carrier);

    // Mid-deletion under the batched flow (2026-05-23): the carrier is
    // already trashed. Partition reads `snap.digisources_just_before`
    // (now in trash) and parks a 2-pick.
    {
        assert_eq!(
            r.game.players[0].battle_area.len(),
            0,
            "carrier already trashed (batched flow); battle area is empty"
        );
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("Partition source-pick must be parked");
        assert_eq!(pending.selecting_player, 0);
        assert!(
            !pending.is_optional,
            "DCGO `canNoSelect: () => false` — the 2-pick is mandatory once gate passes"
        );
    }

    // First pick: SRC-A (zone index 0).
    let action_1 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, action_1).expect("first pick");

    // After first pick, a second selection should be parked (count_capped
    // chains single-item picks until max=2 is reached).
    {
        let pending = r
            .game
            .pending_selection
            .as_ref()
            .expect("second source-pick must be parked");
        // The previously-picked index must be excluded from the new
        // candidate set.
        assert_eq!(pending.valid_action_ids.len(), 2);
    }
    let action_2 = r.game.pending_selection.as_ref().unwrap().valid_action_ids[0];
    r.game.resolve_selection(0, action_2).expect("second pick");

    // Restore the test seam (cleanup).
    r.game.set_effect_source_player_for_test(None);

    // After both picks resolve and finalize runs:
    //   - Carrier removed from battle_area.
    //   - 2 picked sources replayed onto the field via post-deletion replay.
    //   - The unpicked source remains in trash.
    //   - PARTITION (the top) is in trash (it wasn't picked).
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "the 2 picked sources replayed onto the field as separate permanents"
    );
    let surviving_ids: std::collections::HashSet<String> = r.game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect();
    // Each surviving permanent must be a fresh play with no sources under it.
    for perm in r.game.players[0].battle_area.iter() {
        assert_eq!(
            perm.card_sources.len(),
            1,
            "replayed permanent has a fresh stack (no sources under top)"
        );
        assert!(
            !perm.is_suspended,
            "Partition replay is unsuspended (DCGO `isTapped: false`)"
        );
    }

    // Exactly two of the original three sources are on field; the third
    // remains in trash. PARTITION (the top) is in trash.
    let trash_ids: std::collections::HashSet<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    assert!(
        trash_ids.contains("PARTITION"),
        "PARTITION (the deleted top) is in trash — it was not pickable"
    );
    let on_field_count = surviving_ids.len();
    let unpicked_in_trash_count = ["SRC-A", "SRC-B", "SRC-C"]
        .iter()
        .filter(|id| trash_ids.contains(**id))
        .count();
    assert_eq!(on_field_count, 2, "two picked sources are on field");
    assert_eq!(
        unpicked_in_trash_count, 1,
        "exactly one source was not picked and remained in trash"
    );

    // Free play: memory does not change due to the replays.
    assert_eq!(
        r.game.memory, memory_before,
        "Partition replays are free — memory must not change"
    );
}

// ─── Test 2: own-effect deletion → Partition does NOT fire ────────────────

/// DCGO `Partition.cs:16` `!IsByEffect(..., IsOwnerEffect)` — Partition
/// does NOT fire when the deletion is caused by an effect sourced from a
/// same-owner card.
#[test]
fn partition_does_not_fire_on_own_effect_deletion() {
    let mut r = DebugRunner::builder()
        .add_card(partition_card("PARTITION"))
        .add_card(plain_digimon("SRC-A"))
        .add_card(plain_digimon("SRC-B"))
        .add_card(plain_digimon("SRC-C"))
        .start();

    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-B");
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-C");
    push_source_card(&mut r, 0, carrier.index as usize, "PARTITION");

    // Carrier owned by player 0; source player = 0 → OwnEffect.
    r.game.set_effect_source_player_for_test(Some(0));

    r.game.delete_permanent_with_effects(carrier);

    r.game.set_effect_source_player_for_test(None);

    // No selection parked; Partition gated out by cause filter.
    assert!(
        r.game.pending_selection.is_none(),
        "Partition must NOT park a selection when cause=OwnEffect"
    );

    // Nothing replayed: carrier and all sources are in trash.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "no replay — battle area is empty after own-effect deletion"
    );
    let trash_ids: std::collections::HashSet<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    for id in ["PARTITION", "SRC-A", "SRC-B", "SRC-C"] {
        assert!(
            trash_ids.contains(id),
            "{} should be in trash after own-effect deletion (no Partition replay)",
            id
        );
    }
    // Phase 5 (2026-05-23): the `pending_post_deletion_replays` slot was
    // retired — Partition now plays from trash inline. The "no Partition
    // replay" semantic is covered by the trash/field assertions above.
}

// ─── Test 3: battle deletion → Partition does NOT fire ────────────────────

/// DCGO `Partition.cs:14` `!IsByBattle(...)` — Partition does NOT fire on
/// battle-driven deletions.
#[test]
fn partition_does_not_fire_on_battle_deletion() {
    use digimon_engine::replacement::ReplacementCause;

    let mut r = DebugRunner::builder()
        .add_card(partition_card("PARTITION"))
        .add_card(plain_digimon("SRC-A"))
        .add_card(plain_digimon("SRC-B"))
        .add_card(plain_digimon("SRC-C"))
        .start();

    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-B");
    push_source_card(&mut r, 0, carrier.index as usize, "SRC-C");
    push_source_card(&mut r, 0, carrier.index as usize, "PARTITION");

    // Drive the deletion with cause=Battle directly via the cause-aware
    // entry point (the higher-level battle resolution path is too heavy
    // for a unit test; we exercise the same fire-site as combat would).
    r.game
        .delete_permanent_with_cause(carrier, ReplacementCause::Battle);

    assert!(
        r.game.pending_selection.is_none(),
        "Partition must NOT park a selection when cause=Battle"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "no replay — battle area is empty after battle deletion"
    );
    let trash_ids: std::collections::HashSet<String> = r.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect();
    for id in ["PARTITION", "SRC-A", "SRC-B", "SRC-C"] {
        assert!(
            trash_ids.contains(id),
            "{} should be in trash after battle deletion (no Partition replay)",
            id
        );
    }
    // Phase 5 (2026-05-23): the `pending_post_deletion_replays` slot was
    // retired — Partition now plays from trash inline. The "no Partition
    // replay" semantic is covered by the trash/field assertions above.
}

// ─── Test 4: insufficient sources → Partition gate fails, no replay ───────

/// DCGO `Partition.cs:32` `DigivolutionCards.Count >= 2` (translated to
/// `card_sources.len() >= 3` in Rust = top + ≥2 sources). With only 1
/// source under the top, Partition's selection gate fails — no selection
/// is parked, deletion proceeds normally, no replay.
#[test]
fn partition_does_not_fire_when_fewer_than_two_sources() {
    let mut r = DebugRunner::builder()
        .add_card(partition_card("PARTITION"))
        .add_card(plain_digimon("SRC-A"))
        .start();

    // Stack: [SRC-A, PARTITION] (just 1 source under the top).
    let carrier = r.place_on_field(0, "SRC-A", None);
    push_source_card(&mut r, 0, carrier.index as usize, "PARTITION");
    {
        let perm = &r.game.players[0].battle_area[carrier.index as usize];
        assert_eq!(
            perm.card_sources.len(),
            2,
            "preconditions: only 1 source under top"
        );
    }

    // Cause = OpponentEffect (so the cause filter is NOT what gates this
    // test out — only the source-count gate).
    r.game.set_effect_source_player_for_test(Some(1));

    r.game.delete_permanent_with_effects(carrier);

    r.game.set_effect_source_player_for_test(None);

    assert!(
        r.game.pending_selection.is_none(),
        "Partition must NOT park a selection with <2 selectable sources"
    );
    assert_eq!(
        r.game.players[0].battle_area.len(),
        0,
        "no replay — battle area is empty after deletion with insufficient sources"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "PARTITION"),
        "PARTITION in trash via normal deletion"
    );
    assert!(
        r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "SRC-A"),
        "SRC-A in trash via normal deletion"
    );
    // Phase 5 (2026-05-23): the `pending_post_deletion_replays` slot was
    // retired — Partition now plays from trash inline. The "no Partition
    // replay" semantic is covered by the trash/field assertions above.
}
