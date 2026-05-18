//! ST19-03 Shoemon.
//! Printed text: [On Play] Reveal the top 3 cards of your deck. Add 1 card with
//! the [Puppet] trait and 1 card with the [LIBERATOR] trait among them to the
//! hand. Return the rest to the bottom of the deck.
//! Inherited: [Your Turn] All of your opponent's security Digimon get -3000 DP.

use digimon_engine::action::space::{PASS, SEL_REVEAL_START};
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

#[test]
fn st19_03_on_play_adds_puppet_and_liberator_without_double_picking_dual_match() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-03")
        .expect("ST19-03 YAML loads")
        .add_card(make_trait_card("PUPPET-ONLY", &["Puppet"]))
        .add_card(make_trait_card("LIBERATOR-ONLY", &["LIBERATOR"]))
        .add_card(make_trait_card("DUAL", &["Puppet", "LIBERATOR"]))
        .deck(0, &["DUAL", "LIBERATOR-ONLY", "PUPPET-ONLY"])
        .hand(0, &["ST19-03"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("play Shoemon");
    assert_required_pending_pick(&runner, "Puppet bucket");
    pick_revealed_by_id(&mut runner, "DUAL", "pick dual Puppet");

    assert_required_pending_pick(&runner, "LIBERATOR bucket");
    let second_view = runner.pending_selection_view().expect("LIBERATOR bucket");
    let dual_action = revealed_action_for_id(&runner, "DUAL");
    assert!(
        !second_view.valid_action_ids.contains(&dual_action),
        "the same revealed card cannot satisfy both printed buckets"
    );
    pick_revealed_by_id(&mut runner, "LIBERATOR-ONLY", "pick LIBERATOR");
    runner.auto_resolve().expect("bottom remainder");

    let hand_ids = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand_ids.contains(&"DUAL".to_string()));
    assert!(hand_ids.contains(&"LIBERATOR-ONLY".to_string()));
    assert!(!hand_ids.contains(&"PUPPET-ONLY".to_string()));
}

fn assert_required_pending_pick(runner: &DebugRunner, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    assert!(
        !runner.pending_is_optional(),
        "{label}: bucket pick is required"
    );
    assert!(
        !view.valid_action_ids.contains(&PASS),
        "{label}: PASS is not legal"
    );
}

fn make_trait_card(id: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.traits = traits.iter().map(|s| s.to_string()).collect();
    card
}

fn pick_revealed_by_id(runner: &mut DebugRunner, id: &str, label: &str) {
    let action = revealed_action_for_id(runner, id);
    let view = runner.pending_selection_view().expect(label);
    assert!(view.valid_action_ids.contains(&action), "{label}");
    runner.execute_action(0, action).expect(label);
}

fn revealed_action_for_id(runner: &DebugRunner, id: &str) -> u16 {
    runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(idx, card)| {
            (card.card_id(&runner.game.card_data) == id).then_some(SEL_REVEAL_START + idx as u16)
        })
        .expect("revealed card exists")
}

fn zone_ids(
    cards: &[digimon_engine::card_source::CardSource],
    data: &[digimon_engine::card_data::CardData],
) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn attacker_lv4(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card.play_cost = 5;
    card
}

fn digimon_security_card(id: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card
}

#[test]
fn st19_03_inherited_security_dp_debuff_lets_8000_attacker_survive_9000_security_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-03")
        .expect("ST19-03 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));
    {
        let game = runner.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "ST19-03")
            .expect("ST19-03 registered");
        let next = game.next_card_index();
        let perm = &mut game.players[atk.player as usize].battle_area[atk.index as usize];
        let mut src = CardSource::new(data_idx, atk.player, next);
        src.card_index = next;
        perm.card_sources.insert(0, src);
    }

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "8000 DP attacker survives after Shoemon's -3000 drops 9000 security Digimon to 6000"
    );
    assert_eq!(runner.battle_area_size(0), 1);
}

#[test]
fn st19_03_inherited_security_dp_debuff_without_shoemon_in_stack_loses_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-03")
        .expect("ST19-03 YAML loads")
        .add_card(attacker_lv4("ATK", 8000))
        .add_card(digimon_security_card("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = runner.place_on_field(0, "ATK", Some(0));

    let result = runner.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::AttackerDeletedBySecurity,
        "without ST19-03 in stack, 8000 attacker loses to 9000 security Digimon"
    );
    assert_eq!(runner.battle_area_size(0), 0);
}
