use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::{CardData, EvoCost};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX12-002";

fn red_sw_digimon(id: &str, level: u8, play_cost: u16) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.traits = vec!["SW".to_string()];
    card.play_cost = play_cost;
    card.dp = Some(if level <= 3 { 3000 } else { 5000 });
    card
}

fn red_sw_evo(id: &str) -> CardData {
    let mut card = red_sw_digimon(id, 4, 4);
    card.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: 3,
    }];
    card
}

fn plain_red_digimon(id: &str) -> CardData {
    let mut card = red_sw_digimon(id, 3, 3);
    card.traits.clear();
    card
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn select_hand_card(runner: &mut DebugRunner, player: u8, card_id: &str) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("EX12-002 should install a hand selection");
        if view.kind == SelectionKind::Hand {
            let slot = hand_index(runner, player, card_id) as u16;
            let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
                .into_iter()
                .find(|action| view.valid_action_ids.contains(action))
                .unwrap_or_else(|| panic!("{card_id} was not selectable from hand; view={view:?}"));
            runner
                .execute_action(player, action)
                .expect("select EX12-002 hand digivolve target");
            return;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != PASS)
            .unwrap_or_else(|| panic!("unexpected EX12-002 prompt with only PASS: {view:?}"));
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance EX12-002 prompt");
    }
    panic!("EX12-002 never reached the expected hand selection");
}

fn top_card_id(runner: &DebugRunner, player: u8, field_index: usize) -> String {
    runner.game.players[player as usize].battle_area[field_index]
        .top_card()
        .card_id(&runner.game.card_data)
        .to_string()
}

#[test]
fn ex12_002_inherited_when_other_sw_played_may_digivolve_source_with_cost_reduced_by_2() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-002 YAML loads")
        .add_card(red_sw_digimon("CARRIER", 3, 3))
        .add_card(red_sw_digimon("SW-ALLY", 3, 3))
        .add_card(red_sw_evo("SW-EVO"))
        .add_card(plain_red_digimon("DRAW"))
        .hand(0, &["SW-ALLY", "SW-EVO"])
        .deck(0, &["DRAW"])
        .memory(10)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let memory_before = runner.memory();
    let ally_slot = hand_index(&runner, 0, "SW-ALLY");

    assert!(
        runner.play(0, ally_slot).is_some(),
        "SW ally should be playable"
    );
    select_hand_card(&mut runner, 0, "SW-EVO");
    runner.auto_resolve().expect("finish EX12-002 effect");

    assert_eq!(
        top_card_id(&runner, 0, carrier.index as usize),
        "SW-EVO",
        "EX12-002's inherited effect should digivolve the source carrier"
    );
    assert_eq!(
        memory_before - runner.memory(),
        4,
        "play cost 3 plus digivolve cost 3 reduced by 2 should spend 4 memory"
    );
}

#[test]
fn ex12_002_inherited_does_not_trigger_for_non_sw_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-002 YAML loads")
        .add_card(red_sw_digimon("CARRIER", 3, 3))
        .add_card(plain_red_digimon("PLAIN-ALLY"))
        .add_card(red_sw_evo("SW-EVO"))
        .hand(0, &["PLAIN-ALLY", "SW-EVO"])
        .memory(10)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let ally_slot = hand_index(&runner, 0, "PLAIN-ALLY");
    assert!(
        runner.play(0, ally_slot).is_some(),
        "plain ally should be playable"
    );
    let _ = runner.auto_resolve();

    assert_eq!(
        top_card_id(&runner, 0, carrier.index as usize),
        "CARRIER",
        "non-SW plays must not trigger EX12-002"
    );
    assert!(
        runner.pending_selection().is_none(),
        "non-SW play should not leave an EX12-002 selection pending"
    );
}
