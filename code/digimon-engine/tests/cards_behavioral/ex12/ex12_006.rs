use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX12-006";

fn red_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.play_cost = 3;
    card.dp = Some(3000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn hand_index(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn hand_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

fn trash_contains(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

fn select_hand_card(runner: &mut DebugRunner, player: u8, card_id: &str) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("EX12-006 should install a hand-cost selection");
        if view.kind == SelectionKind::Hand {
            let slot = hand_index(runner, player, card_id) as u16;
            let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
                .into_iter()
                .find(|action| view.valid_action_ids.contains(action))
                .unwrap_or_else(|| panic!("{card_id} was not selectable from hand; view={view:?}"));
            runner
                .execute_action(player, action)
                .expect("select EX12-006 hand cost");
            return;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != PASS)
            .unwrap_or_else(|| panic!("unexpected EX12-006 prompt with only PASS: {view:?}"));
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance EX12-006 prompt");
    }
    panic!("EX12-006 never reached the expected hand selection");
}

#[test]
fn ex12_006_start_main_trashes_sw_card_then_draws_and_gains_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-006 YAML loads")
        .add_card(red_digimon("SW-COST", &["SW"]))
        .add_card(red_digimon("PLAIN", &[]))
        .add_card(red_digimon("DRAW", &[]))
        .hand(0, &["SW-COST", "PLAIN"])
        .deck(0, &["DRAW"])
        .memory(2)
        .start();

    runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);

    runner.game.enter_main_phase();
    select_hand_card(&mut runner, 0, "SW-COST");
    runner
        .auto_resolve()
        .expect("finish EX12-006 start-main effect");

    assert!(trash_contains(&runner, 0, "SW-COST"));
    assert!(hand_contains(&runner, 0, "PLAIN"));
    assert!(hand_contains(&runner, 0, "DRAW"));
    assert_eq!(runner.deck_size(0), deck_before - 1, "draw 1");
    assert_eq!(runner.memory(), memory_before + 1, "gain 1 memory");
}

#[test]
fn ex12_006_inherited_dp_aura_applies_only_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-006 YAML loads")
        .add_card(red_digimon("CARRIER", &[]))
        .memory(1)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    assert_eq!(
        runner.effective_dp(carrier),
        Some(5000),
        "EX12-006 inherited +2000 DP applies on your turn"
    );

    runner.end_turn();
    assert_eq!(
        runner.effective_dp(carrier),
        Some(3000),
        "EX12-006 inherited +2000 DP turns off outside your turn"
    );
}
