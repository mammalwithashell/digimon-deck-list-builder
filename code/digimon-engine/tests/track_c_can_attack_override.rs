//! Track C / D consult-site test for `CanAttackTargetDefendingPermanent`
//! (2026-05-08).
//!
//! The affirmative inverse of `CannotAttackTarget`: when both modifiers
//! are present on a target, the affirmative wins ("this Digimon CAN
//! attack the otherwise-illegal target"). Pinned at every consult site
//! that reads `CannotAttackTarget`:
//!
//! - `combat::raid_retarget_candidates` — unsuspended pass + fallback pass
//! - `action::decode` attack-action validation
//! - `action::mask` standard-attack emission
//! - `action::mask` granted-attack emission
//! - `action::mask` per-attacker granted-target enumeration
//!
//! The test exercises the action-mask + decode path because that's the
//! observable surface for an RL agent: a target with both modifiers must
//! still appear as a legal attack target.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

fn lv4_attacker(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c
}

fn defender_card(id: &str) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(3000);
    c
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .add_card(lv4_attacker("ATK"))
        .add_card(defender_card("DEF"))
        .start()
}

#[test]
fn cannot_attack_target_alone_blocks_mask_and_decode() {
    // Baseline: a target with `CannotAttackTarget` is filtered out of the
    // mask AND rejected by `attack_digimon`. Pins the gate so the
    // override test cleanly attributes its delta to the affirmative.
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    // Suspend the defender so unsuspended-attack rules don't tangle the test.
    r.game.players[1].battle_area[def.index as usize].is_suspended = true;

    r.game.modifiers.add(
        def,
        ModifierEntry::simple(ModifierType::CannotAttackTarget, 0, Expiry::Permanent, 1),
    );

    let mask = build_action_mask(&r.game, 0);
    let bit = encode_attack(atk.index as u16, def.index as u16) as usize;
    assert_eq!(
        mask[bit], 0.0,
        "mask must omit attack against a CannotAttackTarget defender"
    );

    let result = r.attack_digimon(atk, def, false);
    assert_ne!(
        result,
        AttackResult::AttackerWins,
        "attack_digimon must not connect against a CannotAttackTarget defender"
    );
    assert_ne!(
        result,
        AttackResult::DefenderWins,
        "attack_digimon must not resolve a battle against a CannotAttackTarget defender"
    );
}

#[test]
fn can_attack_target_defending_permanent_overrides_cannot_attack_target() {
    // Both modifiers present — the affirmative `CanAttackTargetDefendingPermanent`
    // wins, so the target reappears in the mask AND `attack_digimon` proceeds.
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game.players[1].battle_area[def.index as usize].is_suspended = true;

    r.game.modifiers.add(
        def,
        ModifierEntry::simple(ModifierType::CannotAttackTarget, 0, Expiry::Permanent, 1),
    );
    r.game.modifiers.add(
        def,
        ModifierEntry::simple(
            ModifierType::CanAttackTargetDefendingPermanent,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let mask = build_action_mask(&r.game, 0);
    let bit = encode_attack(atk.index as u16, def.index as u16) as usize;
    assert_eq!(
        mask[bit], 1.0,
        "CanAttackTargetDefendingPermanent must restore the mask bit"
    );

    let result = r.attack_digimon(atk, def, false);
    assert_ne!(
        result,
        AttackResult::Invalid,
        "attack_digimon must accept the target when the affirmative override is set"
    );
}

#[test]
fn can_attack_target_defending_permanent_alone_is_a_no_op() {
    // Sanity: the affirmative on its own without the negative is a no-op
    // (the target was already legal). Pinned so a future implementation
    // mistake that grants attacks unconditionally would surface.
    let mut r = runner();
    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game.players[1].battle_area[def.index as usize].is_suspended = true;

    r.game.modifiers.add(
        def,
        ModifierEntry::simple(
            ModifierType::CanAttackTargetDefendingPermanent,
            0,
            Expiry::Permanent,
            1,
        ),
    );

    let mask = build_action_mask(&r.game, 0);
    let bit = encode_attack(atk.index as u16, def.index as u16) as usize;
    assert_eq!(
        mask[bit], 1.0,
        "target without CannotAttackTarget remains legal regardless of the affirmative"
    );
}
