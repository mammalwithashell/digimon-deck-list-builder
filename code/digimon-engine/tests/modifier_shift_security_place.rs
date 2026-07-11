//! Modifier-registry re-keying after battle-area removal via the three
//! permanent-to-security placement paths (2026-07-10).
//!
//! Battle-area handles are index based: removing a permanent shifts every
//! later permanent of the same player down by one, so each removal path must
//! call `ModifierRegistry::shift_after_battle_area_remove` (the pattern used
//! by `trash_single_for_batch` in `combat/deletion.rs`,
//! `soft_remove_if_emptied` in `game/mod.rs`, and
//! `absorb_standing_digimon_as_link` in `game_actions/link.rs`).
//!
//! The three security-placement removal paths in `game_actions/mod.rs` —
//! `place_permanent_on_security_observed`,
//! `place_sourceless_permanent_on_security_bottom`, and
//! `place_permanent_on_security_without_leave_replacement` — removed the slot
//! WITHOUT re-keying, so after an effect placed a lower-indexed permanent
//! into security (e.g. EX8-028's pay cost), DP buffs / keyword grants on
//! higher-indexed permanents silently migrated to the wrong permanent.
//!
//! Board shape per test: player 0 has VICTIM (index 0), BUFFED (index 1,
//! carries ChangeDp +3000), BYSTANDER (index 2). VICTIM is placed into
//! security; the modifier must follow BUFFED (now index 0) and must NOT
//! land on BYSTANDER (now index 1).

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardKind, Expiry, ModifierType, StackPosition};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;

fn digimon(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.play_cost = 4;
    c
}

/// Re-resolve a permanent's handle by top-card id (battle-area indices shift
/// when a lower-indexed permanent leaves the field).
fn find_permanent(runner: &DebugRunner, player: u8, card_id: &str) -> PermanentHandle {
    let index = runner.game.players[player as usize]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} not on player {player}'s battle area"));
    PermanentHandle {
        player,
        index: index as u8,
    }
}

/// VICTIM at index 0, BUFFED (ChangeDp +3000) at index 1, BYSTANDER at
/// index 2 — all on player 0's battle area.
fn setup() -> (DebugRunner, PermanentHandle) {
    let mut r = DebugRunner::builder()
        .add_card(digimon("VICTIM", "Victim"))
        .add_card(digimon("BUFFED", "Buffed"))
        .add_card(digimon("BYSTANDER", "Bystander"))
        .start();

    let victim = r.place_on_field(0, "VICTIM", Some(0));
    let buffed = r.place_on_field(0, "BUFFED", Some(0));
    let bystander = r.place_on_field(0, "BYSTANDER", Some(0));
    assert_eq!(victim.index, 0);
    assert_eq!(buffed.index, 1);
    assert_eq!(bystander.index, 2);

    r.game.modifiers.add(
        buffed,
        ModifierEntry::simple(ModifierType::ChangeDp, 3000, Expiry::Permanent, 0),
    );
    assert_eq!(
        r.game.modifiers.sum(buffed, ModifierType::ChangeDp),
        3000,
        "precondition: the DP buff is keyed to BUFFED"
    );

    (r, victim)
}

/// After VICTIM leaves the battle area for security, the +3000 must still
/// apply to BUFFED (found by identity, not index) and BYSTANDER must stay
/// unbuffed.
fn assert_modifier_followed_buffed(r: &DebugRunner) {
    assert_eq!(
        r.game.players[0].battle_area.len(),
        2,
        "VICTIM left the battle area"
    );
    assert_eq!(
        r.game.players[0].security.len(),
        1,
        "VICTIM's card joined the security stack"
    );

    let buffed = find_permanent(r, 0, "BUFFED");
    let bystander = find_permanent(r, 0, "BYSTANDER");
    assert_eq!(
        r.game.modifiers.sum(buffed, ModifierType::ChangeDp),
        3000,
        "the DP buff must follow BUFFED after the battle-area shift \
         (registry not re-keyed after remove)"
    );
    assert_eq!(
        r.game.modifiers.sum(bystander, ModifierType::ChangeDp),
        0,
        "the DP buff must NOT migrate to BYSTANDER \
         (registry not re-keyed after remove)"
    );
}

/// Site 1: `Game::place_permanent_on_security_observed` via
/// `EffectContext::place_self_at_security` (EX4-060 / EX9-021 shape).
#[test]
fn observed_place_rekeys_modifiers_after_battle_area_shift() {
    let (mut r, victim) = setup();

    let placed = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), Some(victim), 0);
        ctx.place_self_at_security(StackPosition::Bottom, false)
    };
    assert!(placed, "VICTIM must be placed into security");

    assert_modifier_followed_buffed(&r);
}

/// Site 2: `Game::place_sourceless_permanent_on_security_bottom` via
/// `EffectContext::place_sourceless_permanent_bottom_security_and_cancel_current_replacement`
/// (EX8-028 pay-cost shape).
#[test]
fn sourceless_bottom_place_rekeys_modifiers_after_battle_area_shift() {
    let (mut r, victim) = setup();

    let placed = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.place_sourceless_permanent_bottom_security_and_cancel_current_replacement(0, victim)
    };
    assert!(placed, "the sourceless VICTIM must be placed into security");

    assert_modifier_followed_buffed(&r);
}

/// Site 3: `Game::place_permanent_on_security_without_leave_replacement` via
/// `EffectContext::place_permanent_on_security` (Track A move-to-security;
/// no leave replacement registered, so the call falls through to the
/// without-leave-replacement body).
#[test]
fn without_leave_replacement_place_rekeys_modifiers_after_battle_area_shift() {
    let (mut r, victim) = setup();

    let placed = {
        let mut ctx = EffectContext::new(&mut r.game, CardHandle(0), None, 0);
        ctx.place_permanent_on_security(0, victim, StackPosition::Bottom, false)
    };
    assert!(placed, "VICTIM must be placed into security");

    assert_modifier_followed_buffed(&r);
}
