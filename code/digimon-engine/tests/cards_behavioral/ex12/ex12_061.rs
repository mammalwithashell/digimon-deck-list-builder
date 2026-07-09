use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "EX12-061";

fn purple_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
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
            .expect("EX12-061 should install a hand selection");
        if view.kind == SelectionKind::Hand {
            let slot = hand_index(runner, player, card_id) as u16;
            let action = [PLAY_HAND_START + slot, HAND_EFFECT_START + slot]
                .into_iter()
                .find(|action| view.valid_action_ids.contains(action))
                .unwrap_or_else(|| panic!("{card_id} was not selectable from hand; view={view:?}"));
            runner
                .execute_action(player, action)
                .expect("select EX12-061 hand card");
            return;
        }
        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|action| *action != PASS)
            .unwrap_or_else(|| panic!("unexpected EX12-061 prompt with only PASS: {view:?}"));
        runner
            .execute_action(view.selecting_player, action)
            .expect("advance EX12-061 prompt");
    }
    panic!("EX12-061 never reached the expected hand selection");
}

#[test]
fn ex12_061_on_play_trashes_puppet_or_tb_card_then_draws_two() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-061 YAML loads")
        .add_card(purple_digimon("PUPPET-COST", &["Puppet"]))
        .add_card(purple_digimon("PLAIN", &[]))
        .add_card(purple_digimon("DRAW-A", &[]))
        .add_card(purple_digimon("DRAW-B", &[]))
        .hand(0, &[CARD_ID, "PUPPET-COST", "PLAIN"])
        .deck(0, &["DRAW-A", "DRAW-B"])
        .memory(10)
        .start();
    let deck_before = runner.deck_size(0);

    assert!(runner.play(0, 0).is_some(), "EX12-061 should be playable");
    select_hand_card(&mut runner, 0, "PUPPET-COST");
    runner
        .auto_resolve()
        .expect("finish EX12-061 on-play effect");

    assert!(trash_contains(&runner, 0, "PUPPET-COST"));
    assert!(hand_contains(&runner, 0, "PLAIN"));
    assert!(hand_contains(&runner, 0, "DRAW-A"));
    assert!(hand_contains(&runner, 0, "DRAW-B"));
    assert_eq!(runner.deck_size(0), deck_before - 2);
}

#[test]
fn ex12_061_inherited_when_attacking_draws_then_trashes_one_card() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-061 YAML loads")
        .add_card(purple_digimon("CARRIER", &[]))
        .add_card(purple_digimon("DRAWN", &[]))
        .add_card(purple_digimon("SEC", &[]))
        .deck(0, &["DRAWN"])
        .security(1, &["SEC"])
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let deck_before = runner.deck_size(0);

    runner.attack_player(carrier, 1, false);
    select_hand_card(&mut runner, 0, "DRAWN");
    runner
        .auto_resolve()
        .expect("finish EX12-061 inherited effect");

    assert_eq!(runner.deck_size(0), deck_before - 1, "draw 1 happened");
    assert!(
        trash_contains(&runner, 0, "DRAWN"),
        "drawn card was trashed"
    );
    assert!(!hand_contains(&runner, 0, "DRAWN"));
}
