//! BT20-076 Imperialdramon: Dragon Mode

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};

#[test]
fn bt20_076_counter_blast_dna_uses_dinobeemon_and_paildramon() {
    // Real printed name "Dinobeemon" (BT3-055 .. BT20-074; DCGO
    // EqualsCardName("Dinobeemon")) — the material gate is an exact
    // case-sensitive `name_is`, so the synthetic must use the pool spelling.
    let mut dinobeemon = make_test_card_with_level("TEST-DINOBEEMON", "Dinobeemon", 5);
    dinobeemon.colors = vec![CardColor::Purple];
    dinobeemon.dp = Some(8000);

    let mut paildramon = make_test_card_with_level("TEST-PAILDRAMON", "Paildramon", 5);
    paildramon.colors = vec![CardColor::Red];
    paildramon.dp = Some(8000);

    let mut attacker = make_test_card_with_level("TEST-ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(7000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-076")
        .expect("BT20-076 YAML loads")
        .add_card(dinobeemon)
        .add_card(paildramon)
        .add_card(attacker)
        .hand(1, &["BT20-076", "TEST-PAILDRAMON"])
        .start();

    let attacking = runner.place_on_field(0, "TEST-ATTACKER", Some(0));
    let dinobeemon = runner.place_on_field(1, "TEST-DINOBEEMON", Some(0));

    let result = runner.attack_digimon(attacking, dinobeemon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    let counter_prompt = runner
        .pending_selection()
        .expect("Counter window should offer Imperialdramon Blast DNA");
    assert!(counter_prompt
        .valid_action_ids
        .contains(&DNA_DIGIVOLVE_START));
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT20-076 for Counter Blast DNA");
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
        .expect("choose DinoBeemon as the field material");
    assert_eq!(
        runner
            .pending_selection()
            .expect("hand material prompt")
            .valid_action_ids,
        vec![PLAY_HAND_START + 1]
    );

    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose Paildramon as the hand material");

    let evolved = &runner.game.players[1].battle_area[0];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "BT20-076"
    );
    assert!(evolved
        .card_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "TEST-PAILDRAMON"));
    assert_eq!(runner.hand_size(1), 0);
}
