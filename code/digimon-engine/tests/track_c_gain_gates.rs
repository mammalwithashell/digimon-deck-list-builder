//! Track C / D consult-site tests for permanent-scoped gain/protection
//! modifiers (2026-05-08):
//!
//! - `CannotAddMemory` — `EffectContext::gain_memory` short-circuits when
//!   any permanent in the acting player's battle area carries the
//!   modifier. Sibling of player-scoped `CannotGainMemoryByEffect`.
//! - `CannotAddSecurity` — `EffectContext::place_on_security` returns
//!   `false` under the same condition. Sibling of player-scoped
//!   `CannotAddSecurityByEffect`.
//! - `ImmuneFromStackTrashing` — `EffectContext::trash_top_source`
//!   returns `false` and leaves the host's stack untouched. Distinct
//!   from `CannotBeDestroyed`, which protects only the top card.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{
    CardKind, CardSourceRef, EffectSourceKind, Expiry, ModifierType, StackPosition,
};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn lv4_digimon(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

fn runner_with_carrier_on_field() -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("CARRIER"))
        .add_card(lv4_digimon("BYSTANDER"))
        .start();
    let carrier = r.place_on_field(0, "CARRIER", Some(0));
    let _bystander = r.place_on_field(0, "BYSTANDER", Some(0));
    (r, carrier)
}

fn enter_effect_scope<F>(
    r: &mut DebugRunner,
    actor: PermanentHandle,
    body: F,
) where
    F: FnOnce(&mut digimon_engine::effect_context::EffectContext<'_>),
{
    let card = r.game.players[actor.player as usize].battle_area[actor.index as usize]
        .top_card()
        .handle();
    let mut ctx = digimon_engine::effect_context::EffectContext::new(
        &mut r.game,
        card,
        Some(actor),
        actor.player,
    );
    body(&mut ctx);
}

// ── CannotAddMemory ────────────────────────────────────────────────────

#[test]
fn cannot_add_memory_blocks_effect_gain_memory() {
    let (mut r, carrier) = runner_with_carrier_on_field();
    let actor = r.perm_handle(0, 1); // BYSTANDER acts; CARRIER bears modifier
    let baseline = r.game.memory;

    r.game.modifiers.add(
        carrier,
        ModifierEntry::simple(ModifierType::CannotAddMemory, 0, Expiry::Permanent, 0),
    );

    enter_effect_scope(&mut r, actor, |ctx| {
        ctx.gain_memory(2);
    });

    assert_eq!(
        r.game.memory, baseline,
        "CannotAddMemory anywhere on acting player's field must suppress gain_memory"
    );
}

#[test]
fn cannot_add_memory_only_blocks_acting_player() {
    // Modifier on opponent's field permanent must NOT suppress P0's
    // memory gains. Mirrors the player-scoped variant's per-player
    // semantics.
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("CARRIER"))
        .add_card(lv4_digimon("ACTOR"))
        .start();
    let _opp_carrier = r.place_on_field(1, "CARRIER", Some(0));
    let actor = r.place_on_field(0, "ACTOR", Some(0));
    let baseline = r.game.memory;

    r.game.modifiers.add(
        r.perm_handle(1, 0),
        ModifierEntry::simple(ModifierType::CannotAddMemory, 0, Expiry::Permanent, 1),
    );

    enter_effect_scope(&mut r, actor, |ctx| {
        ctx.gain_memory(3);
    });

    assert_ne!(
        r.game.memory, baseline,
        "CannotAddMemory on opponent's field must not affect P0's memory gains"
    );
}

// ── CannotAddSecurity ──────────────────────────────────────────────────

#[test]
fn cannot_add_security_blocks_recover_from_deck() {
    let (mut r, carrier) = runner_with_carrier_on_field();
    let actor = r.perm_handle(0, 1);
    // Pad the deck so a recover from deck has cards to draw from.
    for i in 0..3 {
        let card = CardSource::new(0, 0, 1000 + i);
        r.game.players[0].deck.push(card);
    }
    let security_baseline = r.game.players[0].security.len();

    r.game.modifiers.add(
        carrier,
        ModifierEntry::simple(ModifierType::CannotAddSecurity, 0, Expiry::Permanent, 0),
    );

    enter_effect_scope(&mut r, actor, |ctx| {
        ctx.recover_from_deck(0, 2);
    });

    assert_eq!(
        r.game.players[0].security.len(),
        security_baseline,
        "CannotAddSecurity on acting player's field must suppress place_on_security"
    );
}

#[test]
fn cannot_add_security_only_blocks_acting_player() {
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("CARRIER"))
        .add_card(lv4_digimon("ACTOR"))
        .start();
    let _opp_carrier = r.place_on_field(1, "CARRIER", Some(0));
    let actor = r.place_on_field(0, "ACTOR", Some(0));
    for i in 0..3 {
        let card = CardSource::new(0, 0, 2000 + i);
        r.game.players[0].deck.push(card);
    }
    let baseline = r.game.players[0].security.len();

    r.game.modifiers.add(
        r.perm_handle(1, 0),
        ModifierEntry::simple(ModifierType::CannotAddSecurity, 0, Expiry::Permanent, 1),
    );

    enter_effect_scope(&mut r, actor, |ctx| {
        ctx.recover_from_deck(0, 2);
    });

    assert!(
        r.game.players[0].security.len() > baseline,
        "CannotAddSecurity on opponent's field must not affect P0's recover"
    );
    let _ = StackPosition::Top;
    let _ = CardSourceRef::DeckTop(0);
    let _ = EffectSourceKind::Digimon;
}

// ── ImmuneFromStackTrashing ────────────────────────────────────────────

#[test]
fn immune_from_stack_trashing_blocks_trash_top_source() {
    // A 2-source stack with the modifier on the host: trashing the top
    // source returns false and the stack stays at depth 2.
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("BASE"))
        .add_card(lv4_digimon("TOP"))
        .add_card(lv4_digimon("ACTOR"))
        .start();
    let host = r.place_stack(0, &["BASE", "TOP"]);
    let actor = r.place_on_field(0, "ACTOR", Some(0));

    r.game.modifiers.add(
        host,
        ModifierEntry::simple(
            ModifierType::ImmuneFromStackTrashing,
            0,
            Expiry::Permanent,
            0,
        ),
    );

    let initial_stack_depth = r.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();

    let trashed = {
        let mut result = true;
        enter_effect_scope(&mut r, actor, |ctx| {
            result = ctx.trash_top_source(host);
        });
        result
    };

    assert!(
        !trashed,
        "trash_top_source must return false on a host with ImmuneFromStackTrashing"
    );
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        initial_stack_depth,
        "stack must not be peeled when host has ImmuneFromStackTrashing"
    );
}

#[test]
fn immune_from_stack_trashing_unset_lets_trash_proceed() {
    // Sanity baseline — without the modifier, trash_top_source still
    // peels normally so test 1's regression is attributable to the gate.
    let mut r = DebugRunner::builder()
        .add_card(lv4_digimon("BASE"))
        .add_card(lv4_digimon("TOP"))
        .add_card(lv4_digimon("ACTOR"))
        .start();
    let host = r.place_stack(0, &["BASE", "TOP"]);
    let actor = r.place_on_field(0, "ACTOR", Some(0));
    let initial_depth = r.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();

    let trashed = {
        let mut result = false;
        enter_effect_scope(&mut r, actor, |ctx| {
            result = ctx.trash_top_source(host);
        });
        result
    };

    assert!(trashed);
    assert_eq!(
        r.game.players[0].battle_area[host.index as usize]
            .card_sources
            .len(),
        initial_depth - 1,
        "without the modifier, trash_top_source must peel one source"
    );
}
