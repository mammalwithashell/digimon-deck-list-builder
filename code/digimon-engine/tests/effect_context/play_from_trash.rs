//! Task 3 — `EffectContext::play_from_trash_free_unsuspended` plays a card
//! from the controller's trash into the battle area with zero memory cost,
//! unsuspended (active), and ETB triggers active.
//!
//! ## Audit finding: PATH A (thin alias)
//!
//! `Game::play_from_trash_with_cost(player, trash_index, CostDelta::Free)` already
//! covers the required flag combination:
//!   - `CostDelta::Free` resolves to 0 → `effective_cost = 0` →
//!     `pay_memory(0)` is a no-op (memory unchanged).
//!   - `Permanent::new()` initialises `is_suspended: false` → the card lands
//!     unsuspended (active / "ready state").
//!   - `fire_on_play` + `OnEnterFieldAnyone` already run at the end of the
//!     path → ETB triggers fire.
//!
//! The only gap is the call-site convenience: callers have a `CardHandle` (a
//! stable identity across zone moves), not a `trash_index`. The alias locates
//! the card in the controller's trash by handle and delegates.
//!
//! DCGO parity: `Fortitude.cs:54-63`
//!   `PlayPermanentCards(payCost: false, isTapped: false,
//!    root: SelectCardEffect.Root.Trash, activateETB: true)`
//!
//! Task 8 (Fortitude auto-install) is the primary consumer of this primitive.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::{CardHandle, CardSource};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use std::sync::Arc;

// ─── Card factories ─────────────────────────────────────────────────────────

fn make_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 5, // non-zero cost so we can assert memory is NOT spent
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

/// Helper: push a card into a player's trash (bypasses game logic).
fn push_to_trash(r: &mut DebugRunner, player: u8, card_id: &str) -> CardHandle {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let card_index = r.game.next_card_index();
    let card = CardSource::new(data_idx, player, card_index);
    let handle = card.handle();
    r.game.players[player as usize].trash.push(card);
    handle
}

// ─── ETB test card ────────────────────────────────────────────────────────────

/// A `CardEffect` that fires `OnPlay` and calls `ctx.gain_memory(1)`.
/// Used to verify that ETB (OnPlay) triggers fire after
/// `play_from_trash_free_unsuspended`.
struct OnPlayGainMemory;
impl CardEffect for OnPlayGainMemory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Gain 1 memory")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}

// ─── Test 1: Basic free play from trash ─────────────────────────────────────

/// A card in trash, played via `play_from_trash_free_unsuspended`:
///   - Lands in battle area (field_index 0).
///   - Controller's memory is unchanged (no cost paid — printed cost 5).
///   - The permanent is unsuspended (is_suspended == false).
///   - Trash is empty after the call.
#[test]
fn free_play_from_trash_lands_on_field_unsuspended_zero_cost() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("TEST-FORTITUDE"))
        .start();

    let tp = r.game.turn_player();

    // Pre-populate trash manually.
    let card_handle = push_to_trash(&mut r, tp, "TEST-FORTITUDE");

    // Snapshot memory before the call.
    let memory_before = r.game.memory;

    // Confirm pre-conditions.
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        1,
        "trash has 1 card before"
    );
    assert_eq!(
        r.game.players[tp as usize].battle_area.len(),
        0,
        "battle area empty before"
    );

    // Invoke the primitive via a raw EffectContext (no card effect wrapping
    // needed; we call the method directly as any card script would).
    {
        let mut ctx = EffectContext::new(&mut r.game, card_handle, None, tp);
        ctx.play_from_trash_free_unsuspended(card_handle);
    }

    // Card moved from trash to battle area.
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        0,
        "trash empty after free play"
    );
    assert_eq!(
        r.game.players[tp as usize].battle_area.len(),
        1,
        "battle area has 1 permanent after free play"
    );

    // Verify card identity.
    let perm = &r.game.players[tp as usize].battle_area[0];
    assert_eq!(
        perm.top_card().card_id(&r.game.card_data),
        "TEST-FORTITUDE",
        "the correct card is on field"
    );

    // Memory unchanged — no cost was paid.
    assert_eq!(
        r.game.memory, memory_before,
        "memory unchanged after free play (printed cost 5 was NOT deducted)"
    );

    // Permanent is unsuspended (active / ready state).
    assert!(
        !perm.is_suspended,
        "permanent must be unsuspended after free play"
    );
}

// ─── Test 2: ETB triggers fire ───────────────────────────────────────────────

/// A card with an `OnPlay` ("gain 1 memory") ETB effect, placed in trash and
/// replayed via `play_from_trash_free_unsuspended`. The ETB must fire, which
/// increases memory by +1. Combined with the free-cost assertion: memory
/// transitions from `before` to `before + 1` (not `before + 1 - printed_cost`).
#[test]
fn etb_triggers_fire_on_free_play_from_trash() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ETB-DIGIMON"))
        .start();

    let tp = r.game.turn_player();

    // Register the OnPlay effect for ETB-DIGIMON.
    r.register_effect("ETB-DIGIMON", Arc::new(OnPlayGainMemory));

    let card_handle = push_to_trash(&mut r, tp, "ETB-DIGIMON");
    let memory_before = r.game.memory;

    {
        let mut ctx = EffectContext::new(&mut r.game, card_handle, None, tp);
        ctx.play_from_trash_free_unsuspended(card_handle);
    }

    // ETB fired: memory increased by 1. Free play: printed cost (5) NOT paid.
    assert_eq!(
        r.game.memory,
        memory_before + 1,
        "ETB (gain 1 memory) fired; printed cost (5) was NOT paid"
    );
    assert_eq!(
        r.game.players[tp as usize].battle_area.len(),
        1,
        "card is on field"
    );
    assert_eq!(r.game.players[tp as usize].trash.len(), 0, "trash empty");
}

// ─── Test 3: Card removed from trash after play ──────────────────────────────

/// Confirm the card is removed from the correct position in a multi-card trash.
/// Two cards in trash; only the second is replayed. The first must remain.
#[test]
fn card_removed_from_trash_post_play() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("BYSTANDER"))
        .add_card(make_digimon("FORTITUDE-CARD"))
        .start();

    let tp = r.game.turn_player();

    // Push both cards to trash.
    push_to_trash(&mut r, tp, "BYSTANDER");
    let fortitude_handle = push_to_trash(&mut r, tp, "FORTITUDE-CARD");

    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        2,
        "2 cards in trash before"
    );

    {
        let mut ctx = EffectContext::new(&mut r.game, fortitude_handle, None, tp);
        ctx.play_from_trash_free_unsuspended(fortitude_handle);
    }

    // Only BYSTANDER remains in trash.
    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        1,
        "1 card remains in trash after play"
    );
    assert_eq!(
        r.game.players[tp as usize].trash[0].card_id(&r.game.card_data),
        "BYSTANDER",
        "BYSTANDER untouched in trash"
    );

    // FORTITUDE-CARD is on the field.
    assert_eq!(
        r.game.players[tp as usize].battle_area.len(),
        1,
        "1 permanent on field"
    );
    assert_eq!(
        r.game.players[tp as usize].battle_area[0]
            .top_card()
            .card_id(&r.game.card_data),
        "FORTITUDE-CARD",
        "FORTITUDE-CARD landed on field"
    );
}

// ─── Test 4: Defensive None-return when card is no longer in trash ──────────

/// Regression for the `<Save>` + `<Fortitude>` interaction surfaced by Phase D
/// Task 14 integration smoke. If a queued Fortitude replay fires after another
/// effect (e.g. Save) has already moved the card out of trash and under a
/// Tamer's stack, the call must NOT panic — it must return `None` and leave
/// game state untouched. Callers (specifically the drain hook in
/// `combat::finalize_permanent_deletion`) bind the result to `let _ = ...` and
/// silently absorb the no-op.
#[test]
fn returns_none_when_card_not_in_trash() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("RELOCATED-CARD"))
        .start();

    let tp = r.game.turn_player();

    // Push the card to trash, then immediately drain it (simulating another
    // effect relocating the card before the replay fires).
    let handle = push_to_trash(&mut r, tp, "RELOCATED-CARD");
    r.game.players[tp as usize].trash.clear();

    assert_eq!(
        r.game.players[tp as usize].trash.len(),
        0,
        "trash empty before defensive call"
    );
    assert_eq!(
        r.game.players[tp as usize].battle_area.len(),
        0,
        "battle area empty before defensive call"
    );

    let memory_before = r.game.memory;

    let result = {
        let mut ctx = EffectContext::new(&mut r.game, handle, None, tp);
        ctx.play_from_trash_free_unsuspended(handle)
    };

    // None returned, no permanent created, memory unchanged.
    assert!(result.is_none(), "expected None when card is not in trash");
    assert_eq!(
        r.game.players[tp as usize].battle_area.len(),
        0,
        "no permanent created on missing-card path"
    );
    assert_eq!(
        r.game.memory, memory_before,
        "memory untouched on missing-card path"
    );
}
