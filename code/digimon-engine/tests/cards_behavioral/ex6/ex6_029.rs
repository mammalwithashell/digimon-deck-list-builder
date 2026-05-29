//! EX6-029 Mastemon

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{
    encode_attack, DNA_DIGIVOLVE_START, PLAY_HAND_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, EffectTiming, GamePhase};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/ex6/EX6-029.yaml");

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

#[test]
fn ex6_029_on_play_can_play_level_five_angel_from_trash() {
    let mut angel = make_test_card_with_level("TRASH-ANGEL", "Trash Angel", 5);
    angel.colors = vec![CardColor::Yellow];
    angel.traits = vec!["Angel".to_string()];
    angel.dp = Some(6000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-029 YAML loads")
        .add_card(angel)
        .memory(0)
        .start();
    push_to_trash(&mut runner, 0, "TRASH-ANGEL");
    let mastemon = runner.place_on_field(0, "EX6-029", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(mastemon));
    runner.game.drain_effect_queue();

    let pick = runner
        .pending_selection_view()
        .expect("Angel hand/trash prompt");
    assert!(pick.valid_action_ids.contains(&TRASH_EFFECT_START));
    runner
        .execute_action(pick.selecting_player, TRASH_EFFECT_START)
        .expect("play Angel from trash");

    assert!(battle_area_contains(&runner, 0, "TRASH-ANGEL"));
    assert!(!trash_ids(&runner, 0).contains(&"TRASH-ANGEL".to_string()));
}

#[test]
fn ex6_029_dna_origin_places_other_digimon_bottom_and_trashes_opponent_to_four() {
    let mut other = make_test_card("OTHER-DIGIMON", "Other Digimon");
    other.colors = vec![CardColor::Purple];
    other.dp = Some(3000);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("EX6-029 YAML loads")
        .add_card(other)
        .add_card(make_test_card("S1", "S1"))
        .add_card(make_test_card("S2", "S2"))
        .add_card(make_test_card("S3", "S3"))
        .add_card(make_test_card("S4", "S4"))
        .add_card(make_test_card("S5", "S5"))
        .add_card(make_test_card("S6", "S6"))
        .security(1, &["S1", "S2", "S3", "S4", "S5", "S6"])
        .memory(0)
        .start();
    let mastemon = runner.place_on_field(0, "EX6-029", Some(0));
    let target = runner.place_on_field(0, "OTHER-DIGIMON", Some(0));

    fire_when_digivolving(&mut runner, mastemon, true);

    let pick = runner
        .pending_selection_view()
        .expect("other Digimon placement prompt");
    assert!(
        pick.valid_action_ids
            .contains(&encode_attack(target.player as u16, target.index as u16)),
        "the other Digimon must be selectable"
    );
    assert!(
        !pick.valid_action_ids.contains(&encode_attack(
            mastemon.player as u16,
            mastemon.index as u16
        )),
        "the source Mastemon must be excluded by the printed 'other Digimon' text"
    );
    runner
        .execute_action(
            pick.selecting_player,
            encode_attack(target.player as u16, target.index as u16),
        )
        .expect("place other Digimon");

    assert_eq!(runner.security_count(1), 4);
    assert_eq!(runner.security_count(0), 1);
    assert_eq!(
        runner.game.players[0].security[0].card_id(&runner.game.card_data),
        "OTHER-DIGIMON",
        "target is placed at the bottom of its owner's security"
    );
}

fn fire_when_digivolving(runner: &mut DebugRunner, handle: PermanentHandle, dna_origin: bool) {
    let card = runner.game.players[handle.player as usize].battle_area[handle.index as usize]
        .top_card()
        .handle();
    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Digivolved {
            player: handle.player,
            permanent: handle,
            card,
            effect_initiated: false,
            dna_origin,
        },
    );
    runner.game.drain_effect_queue();
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .expect("card exists");
    let card_index = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_idx, player, card_index));
}

fn trash_ids(runner: &DebugRunner, player: u8) -> Vec<String> {
    zone_ids(
        &runner.game.players[player as usize].trash,
        &runner.game.card_data,
    )
}

fn zone_ids(cards: &[CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn battle_area_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}
