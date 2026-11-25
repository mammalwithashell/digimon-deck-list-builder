//! Gameplay checks for printed keywords in the BT27 Alphamon / X Antibody slice.

use super::common::*;
use digimon_engine::action::space::{encode_attack, PASS, SECURITY_TARGET};
use digimon_engine::enums::Keyword;
use digimon_engine::selection::SelectionKind;

#[test]
fn reboot_readies_the_digimon_to_block_on_the_opponents_turn() {
    for card in ["BT27-015", "BT27-068", "BT27-070", "BT27-072"] {
        let mut ally = digimon("MAME", 5, 7000, false);
        ally.card_name = "Mamemon".into();
        let mut r = start(builder(card).add_card(ally));
        let carrier = r.place_on_field(0, card, Some(0));
        let blocker = if card == "BT27-072" {
            r.place_on_field(0, "MAME", Some(0))
        } else {
            carrier
        };
        let attacker = r.place_on_field(1, "TEXT-3", Some(0));
        r.game.players[0].battle_area[blocker.index as usize].is_suspended = true;
        assert!(r.game.players[0].battle_area[blocker.index as usize].is_suspended);
        next_turn(&mut r);
        assert!(
            !r.game.players[0].battle_area[blocker.index as usize].is_suspended,
            "{card}"
        );
        action(
            &mut r,
            1,
            encode_attack(attacker.index.into(), SECURITY_TARGET),
        );
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        r.choose_prompt_card(if card == "BT27-072" { "MAME" } else { card }, None)
            .unwrap();
        if card == "BT27-070" {
            r.assert_prompt(SelectionKind::EffectChoice, false, 0);
            r.execute_branch(1).unwrap();
        }
        assert_eq!(r.security_count(0), 5, "{card}");
        assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3"], "{card}");
    }
}

#[test]
fn piercing_checks_security_after_winning_a_battle() {
    for card in ["BT27-015", "BT27-016", "BT27-034", "BT27-084", "BT27-103"] {
        let mut r = start(builder(card));
        let attacker = r.place_on_field(0, card, Some(0));
        let defender = r.place_on_field(1, "TEXT-3", Some(0));
        r.game.suspend(defender);
        action(
            &mut r,
            0,
            encode_attack(attacker.index.into(), defender.index.into()),
        );
        if card == "BT27-103" {
            r.assert_prompt(SelectionKind::Replacement, true, 0);
            r.decline_prompt().unwrap();
        }
        assert!(
            r.pending_selection().is_none(),
            "{card}: {:?}",
            r.pending_selection_view()
        );
        assert_eq!(
            r.security_count(1),
            if card == "BT27-034" { 3 } else { 4 },
            "{card}"
        );
        assert!(r.game.players[1].battle_area.is_empty(), "{card}");
    }
}

#[test]
fn raid_redirects_to_the_highest_dp_unsuspended_digimon_then_pierces() {
    for card in ["BT27-034", "BT27-103"] {
        let mut r = start(builder(card));
        let attacker = r.place_on_field(0, card, Some(0));
        r.place_on_field(1, "TEXT-3", Some(0));
        r.place_on_field(1, "TEXT-4", Some(0));
        action(
            &mut r,
            0,
            encode_attack(attacker.index.into(), SECURITY_TARGET),
        );
        r.assert_prompt(SelectionKind::TriggerOrder, card == "BT27-103", 0);
        let raid = r
            .pending_selection_view()
            .unwrap()
            .effect_choices
            .unwrap()
            .into_iter()
            .find(|choice| choice.keyword == Some(Keyword::Raid))
            .unwrap();
        action(&mut r, 0, raid.action_id);
        r.assert_prompt(SelectionKind::OppField, true, 0);
        assert_eq!(candidates(&r), ["TEXT-4"]);
        r.choose_prompt_card("TEXT-4", None).unwrap();
        r.decline_prompt().unwrap();
        assert_eq!(r.battle_area_size(1), 1);
        assert_eq!(
            ids(&r, &r.game.players[1].battle_area[0].card_sources),
            ["TEXT-3"]
        );
        assert_eq!(
            r.security_count(1),
            if card == "BT27-034" { 3 } else { 4 },
            "{card}"
        );
    }
}

#[test]
fn collision_forces_an_opposing_digimon_without_blocker_to_intercept() {
    for card in ["BT27-068", "BT27-084"] {
        let mut r = start(builder(card));
        let attacker = r.place_on_field(0, card, Some(0));
        let target = r.place_on_field(1, "TEXT-3", Some(0));
        r.game.suspend(target);
        r.place_on_field(1, "TEXT-4", Some(0));
        action(
            &mut r,
            0,
            encode_attack(attacker.index.into(), target.index.into()),
        );
        r.assert_prompt(SelectionKind::OwnField, false, 1);
        assert_eq!(candidates(&r), ["TEXT-4"]);
        assert_eq!(
            digimon_engine::build_action_mask(&r.game, 1)[PASS as usize],
            0.0
        );
        r.choose_prompt_card("TEXT-4", None).unwrap();
        assert_eq!(
            ids(&r, &r.game.players[1].battle_area[0].card_sources),
            ["TEXT-3"]
        );
        assert_eq!(r.security_count(1), if card == "BT27-084" { 4 } else { 5 });
    }
}

#[test]
fn alliance_suspends_an_ally_to_win_security_battle_and_check_again() {
    for card in ["BT27-072", "BT27-084"] {
        let mut mamemon = digimon("MAME", 5, 7000, false);
        mamemon.card_name = "Mamemon".into();
        let mut r = start(
            builder(card)
                .add_card(mamemon)
                .add_card(digimon("SECURITY", 7, 14000, false))
                .security(
                    1,
                    &[
                        "TEXT-FILLER",
                        "TEXT-FILLER",
                        "TEXT-FILLER",
                        "TEXT-FILLER",
                        "SECURITY",
                    ],
                ),
        );
        let boss = r.place_on_field(0, card, Some(0));
        let attacker = if card == "BT27-072" {
            r.place_on_field(0, "MAME", Some(0))
        } else {
            boss
        };
        let ally = r.place_on_field(0, "TEXT-6", Some(0));
        let printed_dp = r.effective_dp(attacker).unwrap();
        assert!(printed_dp < 14000);
        action(
            &mut r,
            0,
            encode_attack(attacker.index.into(), SECURITY_TARGET),
        );
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        r.choose_prompt_card("TEXT-6", None).unwrap();
        assert!(r.pending_selection().is_none());
        assert!(r.game.players[0].battle_area[ally.index as usize].is_suspended);
        assert_eq!(r.security_count(1), 3);
        assert_eq!(r.effective_dp(attacker), Some(printed_dp));
    }
}

#[test]
fn engage_attacks_at_end_of_turn_and_progress_protects_only_while_attacking() {
    let mut r = start(
        builder("BT27-016")
            .from_dsl_yaml(
                r#"
card: ATTACK-PUNISHER
name: Attack punisher
kind: digimon
level: 3
dp: 2000
cost: 3
color: [black]
effects:
  - when: on_opponent_attack
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: Choose a Digimon to delete
      - delete_permanent: { target: victim }
"#,
            )
            .unwrap(),
    );
    let jesmon = r.place_on_field(0, "BT27-016", Some(0));
    r.place_on_field(0, "TEXT-4", Some(0));
    r.place_on_field(1, "ATTACK-PUNISHER", Some(0));
    action(&mut r, 0, PASS);
    r.assert_prompt(SelectionKind::Target, true, 0);
    action(
        &mut r,
        0,
        encode_attack(jesmon.index.into(), SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::EffectChoice, false, 0);
    r.execute_branch(1).unwrap();
    r.assert_prompt(SelectionKind::OppField, false, 1);
    assert_eq!(candidates(&r), ["BT27-016", "TEXT-4"]);
    assert!(!r.game.effect_can_affect(
        jesmon,
        1,
        digimon_engine::enums::EffectSourceKind::Digimon
    ));
    r.choose_prompt_card("BT27-016", None).unwrap();
    assert_eq!(r.security_count(1), 4);
    assert_eq!(r.game.turn_player(), 1);
    assert_eq!(
        ids(&r, &r.game.players[0].battle_area[0].card_sources),
        ["BT27-016"]
    );
    assert!(r
        .game
        .effect_can_affect(jesmon, 1, digimon_engine::enums::EffectSourceKind::Digimon));
}

#[test]
fn examon_evades_battle_deletion_by_suspending() {
    let mut raider = digimon("RAIDER", 7, 20000, false);
    raider.keywords = vec![Keyword::Raid];
    let mut r = start(builder("BT27-034").add_card(raider));
    r.place_on_field(0, "BT27-034", Some(0));
    let attacker = r.place_on_field(1, "RAIDER", Some(0));
    next_turn(&mut r);
    action(
        &mut r,
        1,
        encode_attack(attacker.index.into(), SECURITY_TARGET),
    );
    r.assert_prompt(SelectionKind::OppField, true, 1);
    assert_eq!(candidates(&r), ["BT27-034"]);
    r.choose_prompt_card("BT27-034", None).unwrap();
    r.assert_prompt(SelectionKind::Hand, true, 0);
    r.decline_prompt().unwrap();
    r.assert_prompt(SelectionKind::Replacement, true, 0);
    r.accept_optional_trigger().unwrap();
    assert_eq!(r.battle_area_size(0), 1);
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert!(r.game.players[0].trash.is_empty());
}

#[test]
fn printed_blocker_intercepts_security_attacks() {
    for card in ["BT27-034", "BT27-084", "BT27-103"] {
        let mut r = start(builder(card));
        r.place_on_field(0, card, Some(0));
        let attacker = r.place_on_field(1, "TEXT-3", Some(0));
        next_turn(&mut r);
        action(
            &mut r,
            1,
            encode_attack(attacker.index.into(), SECURITY_TARGET),
        );
        if card == "BT27-034" {
            r.assert_prompt(SelectionKind::Hand, true, 0);
            r.decline_prompt().unwrap();
        }
        r.assert_prompt(SelectionKind::OwnField, true, 0);
        assert_eq!(candidates(&r), [card]);
        r.choose_prompt_card(card, None).unwrap();
        assert_eq!(r.security_count(0), 5);
        assert_eq!(ids(&r, &r.game.players[1].trash), ["TEXT-3"]);
    }
}
