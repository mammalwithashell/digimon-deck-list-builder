//! `GameEvent::Attack` emission tests. Covers the scenarios in
//! `openspec/changes/add-reward-profiles/specs/engine-event-emission/spec.md`
//! under "Rust engine emits Attack events at attack declaration".

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;

fn fighter(id: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.dp = Some(dp);
    card
}

fn named_fighter(id: &str, name: &str, dp: i32) -> CardData {
    let mut card = make_test_card(id, name);
    card.dp = Some(dp);
    card
}

/// Drain events and return only `Attack` variants for assertion convenience.
fn drained_attacks(r: &mut DebugRunner) -> Vec<GameEvent> {
    r.game
        .drain_events()
        .into_iter()
        .filter(|e| matches!(e, GameEvent::Attack { .. }))
        .collect()
}

#[test]
fn attack_on_digimon_emits_attack_event_with_field_index_target() {
    // Scenario: Attack on a Digimon emits Attack with field-index target.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));

    // Drain any pre-attack events so assertions only see attack-time emissions.
    r.game.drain_events();

    let _result = r.attack_digimon(atk, def, false);

    let attacks = drained_attacks(&mut r);
    assert_eq!(
        attacks.len(),
        1,
        "exactly one Attack event SHALL fire per attack declaration; got {attacks:?}"
    );

    let GameEvent::Attack {
        player,
        attacker_field_index,
        target_field_index,
        target_player,
        ..
    } = attacks[0].clone()
    else {
        unreachable!()
    };

    assert_eq!(player, 0, "attacker is P1 (Rust 0)");
    assert_eq!(
        attacker_field_index, atk.index,
        "attacker_field_index matches handle"
    );
    assert_eq!(
        target_field_index,
        Some(def.index),
        "Digimon target carries Some(field_index)"
    );
    assert_eq!(target_player, Some(1), "target_player is P2 (Rust 1)");
}

#[test]
fn attack_on_security_emits_attack_event_with_none_target_field_index() {
    // Scenario: Attack on security emits Attack with None target_field_index.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.game.drain_events();

    let _result = r.attack_player(atk, 1, false);

    let attacks = drained_attacks(&mut r);
    assert_eq!(
        attacks.len(),
        1,
        "exactly one Attack event SHALL fire per attack declaration"
    );

    let GameEvent::Attack {
        target_field_index,
        target_player,
        ..
    } = attacks[0].clone()
    else {
        unreachable!()
    };

    assert_eq!(
        target_field_index, None,
        "security target carries None field_index"
    );
    assert_eq!(target_player, Some(1), "target_player is P2 (Rust 1)");
}

#[test]
fn attack_on_digimon_carries_attacker_and_target_identity() {
    // Scenario: Attack on a Digimon carries attacker and target identity.
    let mut r = DebugRunner::builder()
        .add_card(named_fighter("ATK", "Greymon", 5000))
        .add_card(named_fighter("DEF", "Tyrannomon", 3000))
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let def = r.place_on_field(1, "DEF", Some(0));
    r.game.drain_events();

    let _ = r.attack_digimon(atk, def, false);

    let attacks = drained_attacks(&mut r);
    let GameEvent::Attack {
        attacker_card_id,
        attacker_card_name,
        target_card_id,
        target_card_name,
        attacker_dp,
        target_dp,
        ..
    } = attacks[0].clone()
    else {
        unreachable!()
    };
    assert_eq!(attacker_card_id, "ATK");
    assert_eq!(attacker_card_name, "Greymon");
    assert_eq!(target_card_id.as_deref(), Some("DEF"));
    assert_eq!(target_card_name.as_deref(), Some("Tyrannomon"));
    assert_eq!(
        attacker_dp,
        Some(5000),
        "attacker effective DP carried for battle"
    );
    assert_eq!(
        target_dp,
        Some(3000),
        "target effective DP carried for battle"
    );
}

#[test]
fn attack_on_security_has_no_target_identity() {
    let mut r = DebugRunner::builder()
        .add_card(named_fighter("ATK", "Greymon", 5000))
        .add_card(make_test_card("SEC", "Security"))
        .security(1, &["SEC", "SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.game.drain_events();

    let _ = r.attack_player(atk, 1, false);

    let attacks = drained_attacks(&mut r);
    let GameEvent::Attack {
        attacker_card_id,
        target_card_id,
        target_card_name,
        target_dp,
        ..
    } = attacks[0].clone()
    else {
        unreachable!()
    };
    assert_eq!(attacker_card_id, "ATK", "attacker identity present");
    assert_eq!(
        target_card_id, None,
        "security attack has no target card id"
    );
    assert_eq!(
        target_card_name, None,
        "security attack has no target card name"
    );
    assert_eq!(target_dp, None, "security attack has no target DP");
}
