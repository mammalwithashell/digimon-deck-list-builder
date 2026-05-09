//! EX6-029 Mastemon

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};

#[test]
fn ex6_029_counter_blast_dna_uses_angewomon_and_ladydevimon() {
    let mut angewomon = make_test_card_with_level("TEST-ANGEWOMON", "Angewomon", 5);
    angewomon.colors = vec![CardColor::Yellow];
    angewomon.dp = Some(8000);

    let mut ladydevimon = make_test_card_with_level("TEST-LADYDEVIMON", "LadyDevimon", 5);
    ladydevimon.colors = vec![CardColor::Purple];
    ladydevimon.dp = Some(8000);

    let mut attacker = make_test_card_with_level("TEST-ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX6-029")
        .expect("EX6-029 YAML loads")
        .add_card(angewomon)
        .add_card(ladydevimon)
        .add_card(attacker)
        .hand(1, &["EX6-029", "TEST-LADYDEVIMON"])
        .start();

    let attacking = runner.place_on_field(0, "TEST-ATTACKER", Some(0));
    let angewomon = runner.place_on_field(1, "TEST-ANGEWOMON", Some(0));

    let result = runner.attack_digimon(attacking, angewomon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    let counter_prompt = runner
        .pending_selection()
        .expect("Counter window should offer Mastemon Blast DNA");
    assert!(counter_prompt
        .valid_action_ids
        .contains(&DNA_DIGIVOLVE_START));
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose EX6-029 for Counter Blast DNA");
    assert_eq!(runner.current_phase(), GamePhase::SelectMaterial);
    assert_eq!(
        runner
            .pending_selection()
            .expect("field material prompt")
            .valid_action_ids,
        vec![0]
    );

    runner
        .execute_action(1, 0)
        .expect("choose Angewomon as the field material");
    assert_eq!(
        runner
            .pending_selection()
            .expect("hand material prompt")
            .valid_action_ids,
        vec![PLAY_HAND_START + 1]
    );

    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose LadyDevimon as the hand material");

    let evolved = &runner.game.players[1].battle_area[0];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "EX6-029"
    );
    assert!(evolved
        .card_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "TEST-LADYDEVIMON"));
    assert_eq!(runner.hand_size(1), 0);
}
