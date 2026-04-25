//! Task 2 — `EffectContext::armor_purge_top` trashes the current top Digimon
//! of a permanent and promotes the next-highest digivolution source to become
//! the new visible top.
//!
//! Stack layout reminder (same as place_under_permanent tests):
//!   `card_sources[0]` = bottom card (base)
//!   `card_sources.last()` = top card (visible Digimon / new top after promote)
//!
//! DCGO parity: `ArmorPurge.cs:50-65`
//!   `RemoveFromAllArea(topCard)` → pop from card_sources
//!   `AddTrashCard(topCard)` → push to controller trash
//!   `RemoveDigivolveRootEffect(topCard, _permanent)` → modifier cleanup
//!     (In this engine modifiers are keyed by PermanentHandle, not per-card;
//!      see deviation note in armor_purge_top doc comment.)

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
fn make_digimon(id: &str) -> CardData {
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
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

/// Helper: push a source card onto an existing permanent's stack (makes it the new top).
fn push_source_on_top(
    r: &mut DebugRunner,
    perm_player: u8,
    perm_idx: usize,
    card_id: &str,
) -> digimon_engine::card_source::CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_source_on_top: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, perm_player, card_index);
    let handle = card.handle();
    r.game.players[perm_player as usize].battle_area[perm_idx]
        .card_sources
        .push(card);
    handle
}

// ── Test 1: Happy path — 3-card stack ────────────────────────────────────────

/// Stack: [SRC-BOTTOM (index 0), SRC-MID (index 1), TOP-DIGIMON (index 2 / top)].
/// After armor_purge_top:
///   - TOP-DIGIMON lands in controller's trash.
///   - SRC-MID becomes the new top (visible Digimon).
///   - SRC-BOTTOM remains at index 0.
///   - Stack length = 2.
#[test]
fn armor_purge_trashes_top_and_promotes_next() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("TOP-DIGIMON"))
        .add_card(make_digimon("SRC-MID"))
        .add_card(make_digimon("SRC-BOTTOM"))
        .start();

    let tp = r.game.turn_player();

    // Place SRC-BOTTOM as the base permanent (played first).
    let perm_handle = r.place_on_field(tp, "SRC-BOTTOM", Some(0));
    let perm_idx = perm_handle.index as usize;

    // Push SRC-MID then TOP-DIGIMON on top (simulates 2 digivolutions).
    push_source_on_top(&mut r, tp, perm_idx, "SRC-MID");
    push_source_on_top(&mut r, tp, perm_idx, "TOP-DIGIMON");

    // Pre-condition: stack is [SRC-BOTTOM, SRC-MID, TOP-DIGIMON].
    {
        let perm = &r.game.players[tp as usize].battle_area[perm_idx];
        assert_eq!(perm.card_sources.len(), 3, "stack has 3 cards before");
        assert_eq!(
            perm.top_card().card_id(&r.game.card_data),
            "TOP-DIGIMON",
            "top is TOP-DIGIMON before purge"
        );
    }
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        0,
        "trash empty before purge"
    );

    // Apply armor_purge_top.
    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.armor_purge_top(perm_handle);
    }

    // Post-conditions.
    let perm = &r.game.players[tp as usize].battle_area[perm_idx];
    assert_eq!(perm.card_sources.len(), 2, "stack shrunk to 2 after purge");
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "SRC-MID",
        "SRC-MID promoted to new top"
    );
    assert_eq!(
        perm.card_sources[0].card_id(&r.game.card_data),
        "SRC-BOTTOM",
        "SRC-BOTTOM stays at index 0 (bottom)"
    );

    // TOP-DIGIMON moved to trash.
    let trash = &r.game.players[tp as usize].trash;
    assert_eq!(trash.len(), 1, "trash has 1 card after purge");
    assert_eq!(
        trash.last().unwrap().card_id(&r.game.card_data),
        "TOP-DIGIMON",
        "TOP-DIGIMON is in trash"
    );
}

// ── Test 2: Minimal 2-card stack ─────────────────────────────────────────────

/// Stack: [BASE (index 0), TOP (index 1)].
/// After armor_purge_top: stack = [BASE], TOP is in trash.
#[test]
fn armor_purge_with_two_card_stack_promotes_base() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("TOP-ARMOR"))
        .add_card(make_digimon("BASE-DIGI"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "BASE-DIGI", Some(0));
    let perm_idx = perm_handle.index as usize;
    push_source_on_top(&mut r, tp, perm_idx, "TOP-ARMOR");

    {
        let perm = &r.game.players[tp as usize].battle_area[perm_idx];
        assert_eq!(perm.card_sources.len(), 2, "2-card stack before");
    }

    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.armor_purge_top(perm_handle);
    }

    let perm = &r.game.players[tp as usize].battle_area[perm_idx];
    assert_eq!(perm.card_sources.len(), 1, "stack has 1 card left");
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "BASE-DIGI",
        "BASE-DIGI is now the (only) top"
    );
    let trash = &r.game.players[tp as usize].trash;
    assert_eq!(trash.len(), 1, "trash has 1 card");
    assert_eq!(
        trash[0].card_id(&r.game.card_data),
        "TOP-ARMOR",
        "TOP-ARMOR went to trash"
    );
}

// ── Test 3: debug_assert fires when stack has only 1 card ────────────────────

/// A permanent with only a single card (no source beneath the top) must
/// cause a debug_assert panic. Production callers gate before invoking.
///
/// Note: debug_assert only fires in debug builds. This test is only meaningful
/// when compiled without `--release`. In release builds the test will pass
/// trivially (the assert is a no-op and the function will proceed — callers
/// are responsible for gating).
#[test]
#[cfg(debug_assertions)]
fn armor_purge_with_only_top_panics_in_debug() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ONLY-TOP"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "ONLY-TOP", Some(0));
    let perm_idx = perm_handle.index as usize;

    // Stack has exactly 1 card; debug_assert should fire.
    {
        let perm = &r.game.players[tp as usize].battle_area[perm_idx];
        assert_eq!(perm.card_sources.len(), 1, "single-card stack");
    }

    // Clone the handle values before the closure to avoid capturing `r` by ref.
    let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
        .top_card()
        .handle();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.armor_purge_top(perm_handle);
    }));
    assert!(
        result.is_err(),
        "armor_purge_top must panic in debug when stack has only 1 card"
    );
}

// ── Test 4: Modifier cleanup — permanent-level modifiers survive ──────────────

/// `ModifierEntry` in this engine is keyed by `PermanentHandle`, not by
/// individual `CardSource`. Therefore, modifiers attached to the permanent
/// handle (e.g. a +1000 DP granted to the whole stack from an external
/// effect) remain valid after `armor_purge_top` — the permanent handle is
/// unchanged.
///
/// This test verifies that calling `armor_purge_top` does NOT accidentally
/// wipe all modifiers on the permanent (over-clearing). The modifier count
/// must be preserved for entries that belong to the permanent as a whole.
#[test]
fn armor_purge_does_not_clear_permanent_level_modifiers() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ARMOR-TOP"))
        .add_card(make_digimon("SOURCE-CARD"))
        .start();

    let tp = r.game.turn_player();
    let perm_handle = r.place_on_field(tp, "SOURCE-CARD", Some(0));
    let perm_idx = perm_handle.index as usize;
    push_source_on_top(&mut r, tp, perm_idx, "ARMOR-TOP");

    // Grant a +1000 DP modifier to the permanent (simulating an external
    // "your Digimon gets +1000 DP" effect — keyed to the permanent handle).
    r.game.modifiers.add(
        perm_handle,
        ModifierEntry::simple(ModifierType::ChangeDp, 1000, Expiry::EndOfTurn, tp),
    );

    // Confirm modifier is present.
    assert_eq!(
        r.game.modifiers.sum(perm_handle, ModifierType::ChangeDp),
        1000,
        "modifier present before purge"
    );

    // Apply armor_purge_top.
    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.armor_purge_top(perm_handle);
    }

    // Modifier must still be on the permanent after purge.
    assert_eq!(
        r.game.modifiers.sum(perm_handle, ModifierType::ChangeDp),
        1000,
        "permanent-level modifier survives armor_purge_top (handle unchanged)"
    );

    // Stack has been mutated correctly.
    let perm = &r.game.players[tp as usize].battle_area[perm_idx];
    assert_eq!(perm.card_sources.len(), 1, "only base card remains");
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "SOURCE-CARD",
        "SOURCE-CARD promoted to top"
    );
}

// ── Test 5: OnDigivolutionCardTrashed observer fires for the trashed top ─────

/// Phase D Task 5 (Concern S1) — `armor_purge_top` MUST fire
/// `OnDigivolutionCardTrashed` for the card it trashes from the top so that
/// observers (e.g. Rocks-archetype source-trash listeners) see the event.
/// Mirrors DCGO `ArmorPurge.cs:65-78`:
/// `StackSkillInfos(hashtable, EffectTiming.WhenTopCardTrashed)`.
///
/// Strategy: register an OBSERVER `CardEffect` whose
/// `OnDigivolutionCardTrashed` body adds a +100 ChangeDp modifier to itself.
/// Place OBSERVER on the field, run `armor_purge_top` on a separate permanent,
/// and assert the observer's DP grew by 100 — proof the event fired exactly
/// once.
#[test]
fn armor_purge_top_fires_on_digivolution_card_trashed() {
    use digimon_engine::cards::CardEffectRegistry;
    use digimon_engine::effect::{CardEffect, Effect};
    use digimon_engine::enums::ModifierType;
    use std::sync::Arc;

    struct ObserverEffect;
    impl CardEffect for ObserverEffect {
        fn effects(
            &self,
            card: digimon_engine::card_source::CardHandle,
        ) -> Vec<Effect> {
            vec![Effect::on_digivolution_card_trashed(card)
                .name("count source-trash fires")
                .process(|ctx| {
                    if let Some(perm) = ctx.source_permanent {
                        ctx.game.modifiers.add(
                            perm,
                            ModifierEntry::simple(
                                ModifierType::ChangeDp,
                                100,
                                Expiry::Permanent,
                                ctx.player,
                            ),
                        );
                    }
                })
                .build()]
        }
    }

    let mut registry = CardEffectRegistry::default();
    registry.insert("OBSERVER", Arc::new(ObserverEffect));

    let mut r = DebugRunner::builder()
        .add_card(make_digimon("TOP-DIGIMON"))
        .add_card(make_digimon("SRC-MID"))
        .add_card(make_digimon("OBSERVER"))
        .with_registry(registry)
        .start();

    let tp = r.game.turn_player();

    // Build a 2-source stack on player tp's field.
    let perm_handle = r.place_on_field(tp, "SRC-MID", Some(0));
    let perm_idx = perm_handle.index as usize;
    push_source_on_top(&mut r, tp, perm_idx, "TOP-DIGIMON");

    // Place OBSERVER on the same side.
    let observer = r.place_on_field(tp, "OBSERVER", None);

    // Baseline: OBSERVER has no DP delta yet.
    let dp_before = r.game.modifiers.sum(observer, ModifierType::ChangeDp);
    assert_eq!(dp_before, 0);

    // Apply armor_purge_top → top is trashed → OnDigivolutionCardTrashed fires.
    {
        let dummy_handle = r.game.players[tp as usize].battle_area[perm_idx]
            .top_card()
            .handle();
        let mut ctx = EffectContext::new(&mut r.game, dummy_handle, Some(perm_handle), tp);
        ctx.armor_purge_top(perm_handle);
    }

    // OBSERVER must have observed exactly 1 source-trash event.
    let dp_after = r.game.modifiers.sum(observer, ModifierType::ChangeDp);
    assert_eq!(
        dp_after,
        100,
        "OnDigivolutionCardTrashed must fire exactly once for the trashed top \
         (observer DP delta = {})",
        dp_after - dp_before
    );
}
