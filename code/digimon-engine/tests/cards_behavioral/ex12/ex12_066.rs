use digimon_engine::enums::{CardColor, EffectTiming};
use digimon_engine::selection::{SelectionKind, TriggerSource};

use super::support::{
    field_contains, is_suspended, plain_digimon, select_first_non_pass, select_hand_card,
    top_card_id, vb_digimon, with_evo_cost, DebugRunner,
};

const CARD_ID: &str = "EX12-066";

#[test]
fn ex12_066_start_of_turn_sets_memory_to_three_when_two_or_less() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .memory(2)
        .start();

    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourTurn,
        TriggerSource::Permanent(hiro),
    );
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 3);
}

#[test]
fn ex12_066_attack_trigger_suspends_hiro_to_digivolve_attacking_vb_with_cost_reduced() {
    let evo = with_evo_cost(
        vb_digimon("VB-EVO", CardColor::Red, 6, 12000),
        CardColor::Red,
        4,
        2,
    );

    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Red, 4, 5000))
        .add_card(evo)
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &["VB-EVO"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.set_turn(1);
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    let offer = runner
        .pending_selection_view()
        .expect("Hiro should offer an attack trigger");
    assert!(offer.is_optional, "Hiro's attack trigger is optional");
    select_first_non_pass(&mut runner);

    let mode = runner
        .pending_selection_view()
        .expect("Hiro should choose digivolve or use option");
    assert_eq!(mode.kind, SelectionKind::EffectChoice);
    runner.execute_branch(0).expect("choose digivolve mode");
    select_hand_card(&mut runner, 0, "VB-EVO");
    runner.auto_resolve().expect("resolve Hiro digivolve");

    assert!(is_suspended(&runner, 0, hiro.index as usize));
    assert_eq!(top_card_id(&runner, 0, attacker.index as usize), "VB-EVO");
    assert_eq!(memory_before - runner.memory(), 1);
}

#[test]
fn ex12_066_attack_trigger_can_use_a_vb_option_from_hand_with_cost_reduced() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .dsl_card("EX12-069")
        .expect("EX12-069 YAML loads")
        .add_card(vb_digimon("ATTACKER", CardColor::Red, 4, 5000))
        .add_card(plain_digimon("SEC", CardColor::Purple, 3, 2000))
        .hand(0, &["EX12-069"])
        .security(1, &["SEC"])
        .memory(5)
        .start();
    runner.set_turn(1);
    let hiro = runner.place_on_field(0, CARD_ID, Some(0));
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let memory_before = runner.memory();

    runner.attack_player(attacker, 1, false);
    select_first_non_pass(&mut runner);
    runner.execute_branch(1).expect("choose use-option mode");

    let option_pick = runner
        .pending_selection_view()
        .expect("Hiro should offer the eligible VB Option in hand");
    assert_eq!(option_pick.kind, SelectionKind::Hand);
    select_hand_card(&mut runner, 0, "EX12-069");
    runner.auto_resolve().expect("resolve Hiro option use");

    assert!(is_suspended(&runner, 0, hiro.index as usize));
    assert!(runner.game.players[0]
        .security
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "EX12-069"));
    assert_eq!(memory_before, runner.memory(), "cost 2 reduced by 2 is free");
}

#[test]
fn ex12_066_security_plays_hiro_without_paying_cost() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-066 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve Hiro security effect");

    assert!(field_contains(&runner, 1, CARD_ID));
    assert_eq!(runner.security_count(1), 0);
}
