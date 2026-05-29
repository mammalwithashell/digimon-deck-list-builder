//! ST5 option cards: Laser Eye and Dark Side Attack.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn digimon(id: &str, name: &str, level: u8, play_cost: u16) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.play_cost = play_cost;
    card.dp = Some(3000);
    card
}

fn stack_sources(runner: &DebugRunner, player: usize, field_index: usize) -> usize {
    runner.game.players[player].battle_area[field_index]
        .card_sources
        .len()
}

#[test]
fn st5_15_main_de_digivolves_up_to_two_opponent_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-15")
        .expect("ST5-15 YAML parses")
        .add_card(digimon("ST5-15-A3", "Laser Eye Target A3", 3, 3))
        .add_card(digimon("ST5-15-A4", "Laser Eye Target A4", 4, 5))
        .add_card(digimon("ST5-15-B3", "Laser Eye Target B3", 3, 3))
        .add_card(digimon("ST5-15-B4", "Laser Eye Target B4", 4, 5))
        .hand(0, &["ST5-15"])
        .build();
    runner.place_stack(1, &["ST5-15-A3", "ST5-15-A4"]);
    runner.place_stack(1, &["ST5-15-B3", "ST5-15-B4"]);

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 })
    );
    let first = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, first)
        .expect("select first Laser Eye target");
    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 1 })
    );
    let second = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, second)
        .expect("select second Laser Eye target");
    runner.auto_resolve().expect("finish Laser Eye");

    assert_eq!(stack_sources(&runner, 1, 0), 1);
    assert_eq!(stack_sources(&runner, 1, 1), 1);
}

#[test]
fn st5_15_security_activates_laser_eye_main_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-15")
        .expect("ST5-15 YAML parses")
        .add_card(digimon("ST5-15-SEC3", "Laser Eye Security Target 3", 3, 3))
        .add_card(digimon("ST5-15-SEC4", "Laser Eye Security Target 4", 4, 5))
        .build();
    let option = runner.place_on_field(0, "ST5-15", None);
    runner.place_stack(1, &["ST5-15-SEC3", "ST5-15-SEC4"]);

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(option),
    );
    runner.game.drain_effect_queue();

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::CountCappedMultiSelect { max: 2, picked: 0 })
    );
    let pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, pick)
        .expect("select security Laser Eye target");
    runner.auto_resolve().expect("finish Laser Eye security");

    assert_eq!(stack_sources(&runner, 1, 0), 1);
}

#[test]
fn st5_16_main_deletes_one_opponent_digimon_with_play_cost_seven_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-16")
        .expect("ST5-16 YAML parses")
        .add_card(digimon("ST5-16-LOW", "Dark Side Low", 4, 7))
        .add_card(digimon("ST5-16-HIGH", "Dark Side High", 5, 8))
        .hand(0, &["ST5-16"])
        .build();
    runner.place_on_field(1, "ST5-16-LOW", Some(0));
    runner.place_on_field(1, "ST5-16-HIGH", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert_eq!(
        runner
            .pending_selection_view()
            .unwrap()
            .valid_action_ids
            .len(),
        1,
        "play_cost_lte: 7 excludes the cost-8 Digimon"
    );
    let pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, pick)
        .expect("delete cost-7 Digimon");

    assert_eq!(runner.battle_area_size(1), 1);
}

#[test]
fn st5_16_security_activates_dark_side_attack_main_effect() {
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-16")
        .expect("ST5-16 YAML parses")
        .add_card(digimon("ST5-16-SEC", "Dark Side Security Target", 4, 7))
        .build();
    let option = runner.place_on_field(0, "ST5-16", None);
    runner.place_on_field(1, "ST5-16-SEC", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::SecuritySkill,
        TriggerSource::Permanent(option),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    let pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, pick)
        .expect("delete security target");

    assert_eq!(runner.battle_area_size(1), 0);
}
