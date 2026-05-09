//! BT20-081 Fenriloogamon: Takemikazuchi

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{DNA_DIGIVOLVE_START, PLAY_HAND_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};

#[test]
fn bt20_081_counter_blast_dna_uses_fenriloogamon_and_kazuchimon() {
    let mut fenriloogamon = make_test_card_with_level("TEST-FENRILOOGAMON", "Fenriloogamon", 6);
    fenriloogamon.colors = vec![CardColor::Purple];
    fenriloogamon.dp = Some(12000);

    let mut kazuchimon = make_test_card_with_level("TEST-KAZUCHIMON", "Kazuchimon", 6);
    kazuchimon.colors = vec![CardColor::Yellow];
    kazuchimon.dp = Some(12000);

    let mut attacker = make_test_card_with_level("TEST-ATTACKER", "Attacker", 6);
    attacker.colors = vec![CardColor::Red];
    attacker.dp = Some(15000);

    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-081")
        .expect("BT20-081 YAML loads")
        .add_card(fenriloogamon)
        .add_card(kazuchimon)
        .add_card(attacker)
        .hand(1, &["BT20-081", "TEST-KAZUCHIMON"])
        .start();

    let attacking = runner.place_on_field(0, "TEST-ATTACKER", Some(0));
    let fenriloogamon = runner.place_on_field(1, "TEST-FENRILOOGAMON", Some(0));

    let result = runner.attack_digimon(attacking, fenriloogamon, false);
    assert_eq!(result, digimon_engine::combat::AttackResult::InProgress);
    assert_eq!(runner.current_phase(), GamePhase::CounterTiming);

    let counter_prompt = runner
        .pending_selection()
        .expect("Counter window should offer Fenriloogamon Blast DNA");
    assert!(counter_prompt
        .valid_action_ids
        .contains(&DNA_DIGIVOLVE_START));
    let mask = build_action_mask(&runner.game, 1);
    assert_eq!(mask[DNA_DIGIVOLVE_START as usize], 1.0);

    runner
        .execute_action(1, DNA_DIGIVOLVE_START)
        .expect("choose BT20-081 for Counter Blast DNA");
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
        .expect("choose Fenriloogamon as the field material");
    assert_eq!(
        runner
            .pending_selection()
            .expect("hand material prompt")
            .valid_action_ids,
        vec![PLAY_HAND_START + 1]
    );

    runner
        .execute_action(1, PLAY_HAND_START + 1)
        .expect("choose Kazuchimon as the hand material");

    let evolved = &runner.game.players[1].battle_area[0];
    assert_eq!(
        evolved.top_card().card_id(&runner.game.card_data),
        "BT20-081"
    );
    assert!(evolved
        .card_sources
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "TEST-KAZUCHIMON"));
    assert_eq!(runner.hand_size(1), 0);
}
