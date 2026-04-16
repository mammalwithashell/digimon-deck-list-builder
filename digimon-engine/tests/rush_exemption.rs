//! Rush summoning-sickness exemption tests (§2.1 in RUST_PYTHON_PARITY.md).
//!
//! A Digimon with the Rush keyword — granted via modifier — can attack on
//! the turn it was played. Without Rush, freshly-played Digimon are
//! summoning-sick and can't attack.
//!
//! Scope note: only *modifier-granted* Rush is checked here. Native Rush from
//! a card's static keyword list isn't wired yet — see §2.1 in the parity doc.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword};

fn fighter(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(dp),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn freshly_played_without_rush_cannot_attack() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    // Place both on field with turn_played = current turn (summoning sick).
    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);

    assert!(
        !r.game.can_attack(attacker),
        "fresh non-Rush digimon must not attack"
    );
    let result = r.attack_digimon(attacker, defender);
    assert_eq!(result, digimon_engine::combat::AttackResult::Invalid);
    assert_eq!(r.battle_area_size(1), 1, "defender survived");
}

#[test]
fn freshly_played_with_rush_can_attack() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);

    // Grant Rush for the turn.
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);

    assert!(
        r.game.can_attack(attacker),
        "Rush-granted digimon must be able to attack on the turn played"
    );
    let result = r.attack_digimon(attacker, defender);
    assert_eq!(
        result,
        digimon_engine::combat::AttackResult::AttackerWins,
        "5000 DP attacker beats 3000 DP defender"
    );
    assert_eq!(r.battle_area_size(1), 0, "defender deleted");
}

#[test]
fn rush_does_not_override_suspended_state() {
    // Rush exempts summoning sickness but not the general "must be unsuspended"
    // requirement — a suspended permanent still cannot attack.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 5000))
        .add_card(fighter("DEF", 3000))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    let defender = r.place_on_field(1, "DEF", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    // Suspend the attacker manually.
    r.game.players[0].battle_area[attacker.index as usize].is_suspended = true;

    assert!(
        !r.game.can_attack(attacker),
        "suspended attacker cannot attack, even with Rush"
    );
    let result = r.attack_digimon(attacker, defender);
    assert_eq!(result, digimon_engine::combat::AttackResult::Invalid);
}
