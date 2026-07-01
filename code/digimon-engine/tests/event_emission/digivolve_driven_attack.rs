//! `Game::n_digivolve_driven_attacks` counter tests. Covers the engine-side
//! scenarios in
//! `openspec/changes/add-gameplay-reward-config/specs/engine-event-emission/spec.md`
//! under "Rust engine exposes n_digivolve_driven_attacks counter".
//!
//! The counter bumps once per qualifying attack — defined as: attacker
//! effective level ≥ 5, target is the opponent's security stack
//! (`AttackTarget::Player`), and the security loop actually starts a
//! pop (so Jamming-zeroed attacks don't count). Blocked attacks redirect
//! before reaching the Player arm, so the counter naturally skips them.
//! Per-attack semantics: one increment regardless of Security Attack +N
//! revealing multiple cards.

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

fn fighter_at(card_id: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card_with_level(card_id, card_id, level);
    c.dp = Some(dp);
    c
}

#[test]
fn lv5_attacker_on_security_increments_counter() {
    // Scenario: Lv5+ attacker on security increments.
    let mut r = DebugRunner::builder()
        .add_card(fighter_at("ATK5", 5, 9000))
        .add_card(make_test_card_with_level("SEC", "Sec", 4))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK5", Some(0));
    assert_eq!(r.game.n_digivolve_driven_attacks, [0, 0]);

    let _ = r.attack_player(atk, 1, false);

    assert_eq!(
        r.game.n_digivolve_driven_attacks,
        [1, 0],
        "Lv5 attacker hitting security MUST increment p1's counter by 1"
    );
}

#[test]
fn lv4_attacker_on_security_does_not_increment() {
    // Scenario: Lv4 attacker on security does NOT increment.
    let mut r = DebugRunner::builder()
        .add_card(fighter_at("ATK4", 4, 5000))
        .add_card(make_test_card_with_level("SEC", "Sec", 4))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK4", Some(0));
    let _ = r.attack_player(atk, 1, false);

    assert_eq!(
        r.game.n_digivolve_driven_attacks,
        [0, 0],
        "Lv4 attacker is below the engine's effective_level >= 5 gate"
    );
}

#[test]
fn lv5_attacker_on_digimon_does_not_increment() {
    // Scenario: Lv5+ attacker on digimon does NOT increment.
    let mut r = DebugRunner::builder()
        .add_card(fighter_at("ATK5", 5, 9000))
        .add_card(fighter_at("DEF", 4, 5000))
        .start();

    let atk = r.place_on_field(0, "ATK5", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    let _ = r.attack_digimon(atk, def, false);

    assert_eq!(
        r.game.n_digivolve_driven_attacks,
        [0, 0],
        "Digimon-vs-Digimon battles do NOT count — only Player-target attacks"
    );
}

#[test]
fn blocked_lv5_attack_on_security_does_not_increment() {
    // Scenario: Blocked Lv5+ attack on security does NOT increment.
    //
    // When a Blocker declares a block on a Player-target attack, the
    // target is redirected to the blocker (a Digimon target) BEFORE the
    // attack reaches `resolve_battle_attack_target`'s `Player` arm. So
    // the counter naturally skips it.
    let mut r = DebugRunner::builder()
        .add_card(fighter_at("ATK5", 5, 9000))
        .add_card(fighter_at("BLK", 5, 9000))
        .add_card(make_test_card_with_level("SEC", "Sec", 4))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK5", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_player(atk, 1, false);

    // Accept the block — redirect target to BLK.
    r.game
        .resolve_selection(1, encode_attack(0, blk.index as u16))
        .expect("blocker declares block");

    assert_eq!(
        r.game.n_digivolve_driven_attacks,
        [0, 0],
        "blocked attack never reaches the Player arm; counter MUST stay 0"
    );
}

#[test]
fn declined_block_still_increments_once() {
    // Sanity: if the defender has a Blocker but declines to block,
    // the attack proceeds through the security loop and SHOULD count.
    let mut r = DebugRunner::builder()
        .add_card(fighter_at("ATK5", 5, 9000))
        .add_card(fighter_at("BLK", 5, 9000))
        .add_card(make_test_card_with_level("SEC", "Sec", 4))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK5", Some(0));
    let blk = r.place_on_field(1, "BLK", Some(0));
    r.game
        .modifiers
        .grant_keyword(blk, Keyword::Blocker, Expiry::Permanent, 1);

    r.attack_player(atk, 1, false);
    r.game.resolve_selection(1, PASS).expect("decline block");

    assert_eq!(
        r.game.n_digivolve_driven_attacks,
        [1, 0],
        "declined block routes to security loop — counter MUST increment"
    );
}

#[test]
fn security_attack_plus_one_increments_exactly_once() {
    // Scenario: Security Attack +1 revealing two security cards still
    // increments by exactly 1 (per-attack, not per-card semantics).
    let mut r = DebugRunner::builder()
        .add_card(fighter_at("ATK5", 5, 9000))
        .add_card(make_test_card_with_level("SEC", "Sec", 4))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK5", Some(0));

    // Grant Security Attack +1 via the modifier registry so the
    // recompute loop in resolve_player_security_loop reveals two cards.
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(ModifierType::SecurityAttackChange, 1, Expiry::Permanent, 0),
    );

    let _ = r.attack_player(atk, 1, false);

    assert_eq!(
        r.game.n_digivolve_driven_attacks,
        [1, 0],
        "two security reveals from <Security A. +1> MUST still count as 1 attack"
    );
}

#[test]
fn jamming_zero_strike_does_not_increment() {
    // Belt-and-braces: if the attacker has 0 effective security strike
    // (e.g. Jamming), the security loop early-exits and no card is
    // revealed — so the counter must NOT increment, even at Lv5+.
    //
    // The engine's gate is `initial_strike > 0 BEFORE increment`. We
    // simulate "zero strike" by granting SecurityAttackChange -1
    // (which clamps base 1 - 1 = 0).
    let mut r = DebugRunner::builder()
        .add_card(fighter_at("ATK5", 5, 9000))
        .add_card(make_test_card_with_level("SEC", "Sec", 4))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK5", Some(0));
    r.game.modifiers.add(
        atk,
        ModifierEntry::simple(ModifierType::SecurityAttackChange, -1, Expiry::Permanent, 0),
    );

    let _ = r.attack_player(atk, 1, false);

    assert_eq!(
        r.game.n_digivolve_driven_attacks,
        [0, 0],
        "0-strike attack does not actually reach security — must not count"
    );
}
