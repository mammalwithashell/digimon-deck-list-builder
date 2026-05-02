//! §4.7d FORCE_ATTACK Main-phase mask replacement.
//!
//! If any friendly Digimon carries `ModifierType::ForceAttack` and has at
//! least one legal attack, the Main-phase mask is replaced with a mask
//! containing only that Digimon's attack bits — every other action (play,
//! digivolve, hatch, PASS, field effects, …) is zeroed. Mirrors Python
//! `action_mask.py:227-280`.
//!
//! If no forced Digimon can actually attack (all suspended / all targets
//! blocked), the replacement falls through and the normal Main-phase mask
//! is returned.

use digimon_engine::action::{
    build_action_mask, encode_attack, ACTION_SPACE_SIZE, HATCH, PASS, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType};
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

fn grant_force_attack(
    r: &mut DebugRunner,
    target: digimon_engine::permanent::PermanentHandle,
    source: u8,
) {
    r.game.modifiers.add(
        target,
        ModifierEntry::simple(ModifierType::ForceAttack, 1, Expiry::EndOfTurn, source),
    );
}

#[test]
fn force_attack_zeroes_non_attack_bits_when_active() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FORCED", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let forced = r.place_on_field(tp, "FORCED", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    // Give the turn player a hand card so a normal Main mask would otherwise
    // light up PLAY_HAND bit 0 — we want to prove ForceAttack zeroes it.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FORCED")
        .unwrap();
    let next = r.game.next_card_index();
    let hand_card = digimon_engine::card_source::CardSource::new(data_idx, tp, next);
    r.game.players[tp as usize].hand.push(hand_card);

    grant_force_attack(&mut r, forced, tp);
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);

    // Forced attacker's attack bits present.
    assert_eq!(
        mask[encode_attack(forced.index as u16, defender.index as u16) as usize],
        1.0,
        "forced attacker must be able to attack the suspended defender",
    );
    assert_eq!(
        mask[encode_attack(forced.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "forced attacker must be able to attack security",
    );

    // Every non-attack bit must be zeroed by the replacement.
    assert_eq!(
        mask[0], 0.0,
        "PLAY_HAND bit 0 must be zeroed during ForceAttack"
    );
    assert_eq!(mask[HATCH as usize], 0.0, "HATCH must be zeroed");
    assert_eq!(mask[PASS as usize], 0.0, "PASS must be zeroed");

    // Sanity: the replacement is the same size as the normal mask.
    assert_eq!(mask.len(), ACTION_SPACE_SIZE);
}

#[test]
fn force_attack_emits_attacks_for_all_forced_digimon() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("A", CardColor::Red, 5000))
        .add_card(make_digimon("B", CardColor::Red, 4000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let a = r.place_on_field(tp, "A", Some(0));
    let b = r.place_on_field(tp, "B", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    grant_force_attack(&mut r, a, tp);
    grant_force_attack(&mut r, b, tp);
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);

    assert_eq!(
        mask[encode_attack(a.index as u16, defender.index as u16) as usize],
        1.0,
        "attacker A must retain its forced attack bit",
    );
    assert_eq!(
        mask[encode_attack(b.index as u16, defender.index as u16) as usize],
        1.0,
        "attacker B must retain its forced attack bit",
    );
}

#[test]
fn force_attack_falls_through_when_forced_cannot_attack() {
    // Forced Digimon is suspended → has no legal attack → replacement falls
    // through and the normal Main mask stands.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("SUSP", CardColor::Red, 5000))
        .start();

    let tp = r.game.turn_player();
    let forced = r.place_on_field(tp, "SUSP", Some(0));
    r.game.players[tp as usize].battle_area[forced.index as usize].is_suspended = true;
    grant_force_attack(&mut r, forced, tp);
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[PASS as usize], 1.0,
        "no legal forced attack → fall through to normal Main mask; PASS present",
    );
}

#[test]
fn force_attack_respects_cannot_attack_target() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("FORCED", CardColor::Red, 5000))
        .add_card(make_digimon("DEF", CardColor::Blue, 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let forced = r.place_on_field(tp, "FORCED", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    grant_force_attack(&mut r, forced, tp);
    r.game.modifiers.add(
        defender,
        ModifierEntry::simple(ModifierType::CannotAttackTarget, 1, Expiry::EndOfTurn, opp),
    );
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(forced.index as u16, defender.index as u16) as usize],
        0.0,
        "ForceAttack must skip CannotAttackTarget defenders",
    );
    // Security attack remains legal, so replacement still triggers.
    assert_eq!(
        mask[encode_attack(forced.index as u16, SECURITY_TARGET) as usize],
        1.0,
        "security attack still available under ForceAttack",
    );
    assert_eq!(mask[PASS as usize], 0.0, "PASS must still be zeroed");
}

#[test]
fn force_attack_respects_raid_on_unsuspended_enemies() {
    // Forced attacker with Raid can target unsuspended enemies tied for
    // highest DP. Same semantics as normal Main-phase Raid at §4.4, carried
    // through the replacement.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("RAIDER", CardColor::Red, 6000))
        .add_card(make_digimon("HIGH", CardColor::Blue, 5000))
        .add_card(make_digimon("LOW", CardColor::Blue, 2000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let raider = r.place_on_field(tp, "RAIDER", Some(0));
    let high = r.place_on_field(opp, "HIGH", Some(0));
    let low = r.place_on_field(opp, "LOW", Some(0));

    grant_force_attack(&mut r, raider, tp);
    r.game
        .modifiers
        .grant_keyword(raider, Keyword::Raid, Expiry::EndOfTurn, tp);
    r.game.enter_main_phase();

    let mask = build_action_mask(&r.game, tp);
    assert_eq!(
        mask[encode_attack(raider.index as u16, high.index as u16) as usize],
        1.0,
        "Raid targets tied-for-highest unsuspended DP enemy",
    );
    assert_eq!(
        mask[encode_attack(raider.index as u16, low.index as u16) as usize],
        0.0,
        "Raid must not include the lower-DP unsuspended enemy",
    );
}
