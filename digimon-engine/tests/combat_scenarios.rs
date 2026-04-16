//! Combat + security scenario tests for Phase 4.
//!
//! Exercises attack flow, DP comparison, ties, security check, and
//! OnDeletion firing on combat kills.

use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn big_digimon(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    digimon_engine::card_data::CardData {
        card_id: id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 6,
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

fn option_card(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    digimon_engine::card_data::CardData {
        card_id: id.to_string(),
        card_name: name.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 2,
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
fn attacker_wins_deletes_defender() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(big_digimon("SMALL", "Small", 3000))
        .start();

    let atk = r.place_on_field(0, "BIG", Some(0)); // turn 0 so not summoning-sick
    let def = r.place_on_field(1, "SMALL", Some(0));

    let result = r.attack_digimon(atk, def);
    assert_eq!(result, AttackResult::AttackerWins);
    assert_eq!(r.battle_area_size(0), 1, "attacker still on field");
    assert_eq!(r.battle_area_size(1), 0, "defender deleted");
    assert_eq!(r.trash_size(1), 1, "defender in trash");
}

#[test]
fn defender_wins_deletes_attacker() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(big_digimon("SMALL", "Small", 3000))
        .start();

    let atk = r.place_on_field(0, "SMALL", Some(0));
    let def = r.place_on_field(1, "BIG", Some(0));

    let result = r.attack_digimon(atk, def);
    assert_eq!(result, AttackResult::DefenderWins);
    assert_eq!(r.battle_area_size(0), 0, "attacker deleted");
    assert_eq!(r.battle_area_size(1), 1, "defender survived");
    assert_eq!(r.trash_size(0), 1);
}

#[test]
fn tie_deletes_both() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("EQUAL", "Equal", 5000))
        .start();

    let atk = r.place_on_field(0, "EQUAL", Some(0));
    let def = r.place_on_field(1, "EQUAL", Some(0));

    let result = r.attack_digimon(atk, def);
    assert_eq!(result, AttackResult::MutualDestruction);
    assert_eq!(r.battle_area_size(0), 0);
    assert_eq!(r.battle_area_size(1), 0);
    assert_eq!(r.trash_size(0), 1);
    assert_eq!(r.trash_size(1), 1);
}

#[test]
fn suspended_attacker_cannot_attack() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(big_digimon("SMALL", "Small", 3000))
        .start();
    let atk = r.place_on_field(0, "BIG", Some(0));
    let def = r.place_on_field(1, "SMALL", Some(0));

    // Manually suspend the attacker.
    r.game.players[atk.player as usize].battle_area[atk.index as usize].is_suspended = true;

    let result = r.attack_digimon(atk, def);
    assert_eq!(result, AttackResult::Invalid);
    assert_eq!(r.battle_area_size(1), 1);
}

#[test]
fn summoning_sick_attacker_cannot_attack() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(big_digimon("SMALL", "Small", 3000))
        .start();
    // Place attacker on CURRENT turn — fresh, summoning-sick.
    let atk = r.place_on_field(0, "BIG", None);
    let def = r.place_on_field(1, "SMALL", Some(0));

    let result = r.attack_digimon(atk, def);
    assert_eq!(result, AttackResult::Invalid);
    assert_eq!(r.battle_area_size(0), 1, "attacker still there");
}

#[test]
fn attacker_gets_suspended_after_attack() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(big_digimon("SMALL", "Small", 3000))
        .start();
    let atk = r.place_on_field(0, "BIG", Some(0));
    let def = r.place_on_field(1, "SMALL", Some(0));

    r.attack_digimon(atk, def);
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "attacker should be suspended after attacking"
    );
}

#[test]
fn attack_player_with_security_survives() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(option_card("OPT", "Option"))
        .security(1, &["OPT", "OPT", "OPT"]) // 3 option cards in security
        .start();

    let atk = r.place_on_field(0, "BIG", Some(0));
    let result = r.attack_player(atk, 1);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(r.security_count(1), 2, "one security consumed");
    assert_eq!(r.trash_size(1), 1, "security card trashed");
    assert!(!r.game_over());
}

#[test]
fn attack_player_with_digimon_security_survives_when_attacker_stronger() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", "Attacker", 8000))
        .add_card(big_digimon("WEAK_SEC", "WeakSec", 2000))
        .security(1, &["WEAK_SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let result = r.attack_player(atk, 1);
    // Security drained to 0 after consuming the Digimon sec, but attacker
    // is still alive. Game doesn't end unless a subsequent attack with no
    // security remaining comes in.
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(r.security_count(1), 0);
    assert!(!r.game_over());
}

#[test]
fn attack_player_with_strong_digimon_security_deletes_attacker() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("WEAK", "Weak", 3000))
        .add_card(big_digimon("STRONG_SEC", "StrongSec", 10_000))
        .security(1, &["STRONG_SEC"])
        .start();

    let atk = r.place_on_field(0, "WEAK", Some(0));
    let result = r.attack_player(atk, 1);
    assert_eq!(result, AttackResult::AttackerDeletedBySecurity);
    assert_eq!(r.battle_area_size(0), 0, "attacker deleted");
    assert_eq!(r.security_count(1), 0, "security trashed too");
    assert!(!r.game_over());
}

#[test]
fn attack_player_with_no_security_wins_game() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .start();

    let atk = r.place_on_field(0, "BIG", Some(0));
    let result = r.attack_player(atk, 1);
    assert_eq!(result, AttackResult::GameWon);
    assert!(r.game_over());
    assert_eq!(r.winner(), Some(0));
}

#[test]
fn test_005_on_deletion_fires_during_combat() {
    // TEST-005: "On Deletion: Lose 1 memory."
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-005", "TestFive")) // 2000 DP default
        .add_card(big_digimon("BIG", "Big", 8000))
        .start();

    // Place TEST-005 on P1's field; P0 attacks with BIG.
    let atk = r.place_on_field(0, "BIG", Some(0));
    let def = r.place_on_field(1, "TEST-005", Some(0));

    let m_before = r.memory();
    let result = r.attack_digimon(atk, def);
    assert_eq!(result, AttackResult::AttackerWins);
    assert_eq!(
        r.memory(),
        m_before - 1,
        "TEST-005 should lose 1 memory on deletion"
    );
    assert_eq!(r.battle_area_size(1), 0);
}

#[test]
fn test_003_buff_persists_until_end_of_turn() {
    // Verify that modifiers are wiped when end_turn fires, and that buffs
    // applied by effects actually change combat math.
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-003", "TestThree")) // 2000 DP + buff-all-allies
        .add_card(big_digimon("DEF", "Def", 2500)) // normally beats ally at 2000
        .hand(0, &["TEST-003"])
        .start();

    let ally = r.place_on_field(0, "TEST-003", Some(0));
    // Fire TEST-003's OnPlay effect manually (place_on_field bypasses play_from_hand).
    r.fire_on_play(0, ally.index as usize);

    let def = r.place_on_field(1, "DEF", Some(0));
    // Ally at 2000 + 1000 buff = 3000 vs Def 2500 = ally wins
    assert_eq!(r.effective_dp(ally), Some(3000));
    let result = r.attack_digimon(ally, def);
    assert_eq!(result, AttackResult::AttackerWins);
}

#[test]
fn end_of_turn_expires_dp_buffs() {
    // Separate test: after end_turn, EndOfTurn buffs are gone.
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-003", "TestThree"))
        .hand(0, &["TEST-003"])
        .start();

    let ally = r.place_on_field(0, "TEST-003", Some(0));
    r.fire_on_play(0, ally.index as usize);
    assert_eq!(r.effective_dp(ally), Some(3000));

    r.end_turn();
    // Back to base.
    assert_eq!(r.effective_dp(ally), Some(2000));
}

#[test]
fn attack_count_this_turn_increments() {
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(big_digimon("SMALL", "Small", 3000))
        .start();
    let atk = r.place_on_field(0, "BIG", Some(0));
    let def = r.place_on_field(1, "SMALL", Some(0));

    assert_eq!(r.game.players[0].battle_area[0].attacks_this_turn, 0);
    r.attack_digimon(atk, def);
    assert_eq!(r.game.players[0].battle_area[0].attacks_this_turn, 1);
}

#[test]
fn non_digimon_target_is_invalid() {
    // Set up a Tamer on defender's field (simulate by using the egg maker to
    // create a non-Digimon). Actually simplest: place an "Option"-kind card
    // on the field. Use the card_data to do this.
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("BIG", "Big", 8000))
        .add_card(option_card("TAMER", "Tamer")) // Option kind
        .start();

    let atk = r.place_on_field(0, "BIG", Some(0));
    let tamer = r.place_on_field(1, "TAMER", Some(0));
    let result = r.attack_digimon(atk, tamer);
    assert_eq!(result, AttackResult::Invalid);
    // Attacker did NOT suspend or consume turn; game state unchanged.
    assert!(!r.game.players[0].battle_area[0].is_suspended);
}

#[test]
fn multiple_security_checks_with_sec_attack_plus() {
    // Attacker with SecurityAttack +1 should get 2 checks against security.
    let mut r = DebugRunner::builder()
        .add_card(big_digimon("ATK", "Attacker", 8000))
        .add_card(option_card("OPT", "Option"))
        .security(1, &["OPT", "OPT", "OPT"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));

    // Manually apply Security Attack +1 modifier.
    r.game.modifiers.add(
        atk,
        digimon_engine::modifiers::ModifierEntry {
            modifier: digimon_engine::enums::ModifierType::SecurityAttackChange,
            value: 1,
            expiry: digimon_engine::enums::Expiry::Permanent,
            source_player: 0,
        },
    );

    r.attack_player(atk, 1);
    assert_eq!(
        r.security_count(1),
        1,
        "2 security cards should have been consumed"
    );
    assert_eq!(r.trash_size(1), 2);
}

#[test]
fn digitama_supports_egg_builder() {
    let r = DebugRunner::builder()
        .add_card(make_test_egg("EGG", "Egg"))
        .digitama(0, &["EGG"])
        .start();
    assert_eq!(r.game.player(0).digitama_deck.len(), 1);
}
