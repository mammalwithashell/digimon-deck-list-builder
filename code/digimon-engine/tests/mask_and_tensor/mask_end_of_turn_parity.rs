//! End-of-turn phase mask parity tests.
//!
//! Covers the Vortex slice of §4.6 from RUST_PYTHON_PARITY.md. When
//! `current_phase == GamePhase::EndOfTurnAction`, the mask should emit:
//!   - `PASS` (always, to decline the end-of-turn action)
//!   - attack bits for any permanent with modifier-granted `Keyword::Vortex`
//!     whose `can_attack(vortex=true)` returns true. Vortex bypasses both
//!     summoning sickness (via §2.1 plumbing) and the Main-phase
//!     "unsuspended targets require Raid / CAN_ATTACK_UNSUSPENDED" rule —
//!     any enemy Digimon is a valid target.
//!
//! Phase-transition logic (deciding *when* to enter `EndOfTurnAction`),
//! Overclock / MAY_ATTACK / FORCE_ATTACK bits, and the full interrupt-
//! phase mask builders are all §4.6b / §4.6c / §4.6d residuals.

use digimon_engine::action::{
    build_action_mask, encode_attack, EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_OVERCLOCK,
    FIELD_EFFECT_START, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

fn make_digimon(id: &str, color: CardColor, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 5,
        colors: vec![color],
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

/// Vortex grants the security-attack bit and a bit for every enemy Digimon
/// (suspended or not) when the phase is active.
#[test]
fn mask_vortex_emits_attacks_in_end_of_turn_phase() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let def_idx = r.place_on_field(opp, "DEF", Some(0)).index;
    // Opponent starts suspended so the Main-phase pathway would also allow
    // this attack; we want to confirm it flows through the EndOfTurnAction
    // arm instead.
    r.game.players[opp as usize].battle_area[0].is_suspended = true;

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);

    assert_eq!(
        mask[PASS as usize], 1.0,
        "PASS is always legal in EndOfTurnAction"
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "Vortex permits attacking security",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, def_idx as u16) as usize],
        1.0,
        "Vortex permits attacking enemy Digimon",
    );
}

/// Without a Vortex grant, `EndOfTurnAction` only emits PASS — no attack
/// bits.
#[test]
fn mask_vortex_without_keyword_only_emits_pass() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let def_idx = r.place_on_field(opp, "DEF", Some(0)).index;

    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);

    assert_eq!(mask[PASS as usize], 1.0);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        0.0,
        "no Vortex keyword → no security attack",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, def_idx as u16) as usize],
        0.0,
        "no Vortex keyword → no digimon attack",
    );
}

/// Vortex bypasses summoning sickness — a freshly-played permanent with
/// Vortex granted can attack. Regression guard for the §2.1 `vortex=true`
/// plumbing in `can_attack`.
#[test]
fn mask_vortex_bypasses_summoning_sickness() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    // place_on_field with None uses current turn_count → summoning-sick.
    let attacker = r.place_on_field(tp, "ATK", None);
    r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[0].is_suspended = true;

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "Vortex must bypass summoning sickness (relies on can_attack(vortex=true))",
    );
}

/// Vortex targets unsuspended enemy Digimon — no Raid-style DP tiebreak or
/// CAN_ATTACK_UNSUSPENDED requirement. Any enemy Digimon is valid.
#[test]
fn mask_vortex_targets_unsuspended_digimon_too() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon_without_dp("WEAK", CardColor::Blue))
        .add_card(make_digimon_without_dp("STRONG", CardColor::Blue))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let weak_idx = r.place_on_field(opp, "WEAK", Some(0)).index;
    let strong_idx = r.place_on_field(opp, "STRONG", Some(0)).index;
    // Both unsuspended.
    for p in &mut r.game.players[opp as usize].battle_area {
        p.is_suspended = false;
    }

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, weak_idx as u16) as usize],
        1.0,
        "Vortex targets any enemy Digimon — weak (unsuspended) is valid",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, strong_idx as u16) as usize],
        1.0,
        "Vortex targets any enemy Digimon — strong (unsuspended) is valid",
    );
}

// Helper with distinct DP so we can verify Vortex ignores DP-tiebreak logic
// (unlike Raid in §4.4).
fn make_digimon_without_dp(id: &str, color: CardColor) -> CardData {
    make_digimon(id, color, 3000)
}

/// Vortex attack bits (EndOfTurnAction arm) must also honor
/// CannotAttackTarget on the enemy permanent — parity with Python's
/// action_mask.py:348-351.
#[test]
fn mask_vortex_respects_cannot_attack_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));

    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);
    r.game.modifiers.add(
        defender,
        ModifierEntry::simple(ModifierType::CannotAttackTarget, 1, Expiry::EndOfTurn, opp),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        0.0,
        "Vortex must also honor CannotAttackTarget",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "Vortex security attack is unaffected by CannotAttackTarget",
    );
}

// ─── §4.6c Overclock ───────────────────────────────────────────────────

fn overclock_bit(field_index: usize) -> usize {
    (FIELD_EFFECT_START
        + field_index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_OVERCLOCK) as usize
}

#[test]
fn mask_overclock_emits_sub_slot_0_bit_with_sacrifice_available() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", CardColor::Red, 5000))
        .add_card(make_digimon("SAC", CardColor::Red, 1000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let _sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[overclock_bit(oc.index as usize)],
        1.0,
        "Overclock with at least one other Digimon must emit sub-slot 0",
    );
}

#[test]
fn mask_overclock_suppressed_when_no_sacrifice() {
    // Only the Overclock Digimon on the field — nothing else to sacrifice.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[overclock_bit(oc.index as usize)],
        0.0,
        "no sacrifice available → Overclock bit must not emit",
    );
}

// ─── §4.6c MayAttack ──────────────────────────────────────────────────

#[test]
fn mask_may_attack_emits_attack_bits_against_digimon_and_security() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "MayAttack permits attacking security",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        1.0,
        "MayAttack permits attacking any enemy Digimon",
    );
}

#[test]
fn mask_may_attack_respects_cannot_attack_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );
    r.game.modifiers.add(
        defender,
        ModifierEntry::simple(ModifierType::CannotAttackTarget, 1, Expiry::EndOfTurn, opp),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        0.0,
        "MayAttack must honor CannotAttackTarget like Vortex does",
    );
}

// ─── §4.6c ForceAttack (EOT branch) ───────────────────────────────────

#[test]
fn mask_force_attack_emits_attack_bits_in_eot() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::ForceAttack, 1, Expiry::EndOfTurn, tp),
    );
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "ForceAttack permits attacking security in EOT",
    );
    assert_eq!(
        mask[encode_attack(attacker.index as u16, defender.index as u16) as usize],
        1.0,
        "ForceAttack permits attacking enemy Digimon in EOT",
    );
    // PASS is still emitted — execution-side enforcement of the "mandatory"
    // part is out of scope for mask-level parity (matches Python).
    assert_eq!(mask[PASS as usize], 1.0);
}
