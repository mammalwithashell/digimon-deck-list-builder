use digimon_engine::action::space::{HAND_EFFECT_START, PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT21-092";

fn xros_digimon(card_id: &str, name: &str, level: u8, cost: u16) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card.play_cost = cost;
    card
}

fn tamer(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card
}

fn hand_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn drive_until_hand_play(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    for _ in 0..12 {
        let view = runner
            .pending_selection_view()
            .expect("BT21-092 should keep a pending selection until the hand play");
        let hand_index = hand_index(runner, player, card_id) as u16;
        let play_action = PLAY_HAND_START + hand_index;
        let hand_effect_action = HAND_EFFECT_START + hand_index;
        if matches!(view.kind, SelectionKind::Hand)
            || view.valid_action_ids.contains(&play_action)
            || view.valid_action_ids.contains(&hand_effect_action)
        {
            let action = if view.valid_action_ids.contains(&play_action) {
                play_action
            } else {
                hand_effect_action
            };
            assert!(
                view.valid_action_ids.contains(&action),
                "{card_id} should be legal to play after BT21-092 moves sources; view={view:?}"
            );
            runner
                .execute_action(player, action)
                .expect("select BT21-092 reduced hand play");
            return;
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .unwrap_or_else(|| panic!("BT21-092 prompt had no non-PASS action: {view:?}"));
        runner
            .execute_action(player, action)
            .expect("advance BT21-092 prompt");
    }

    panic!("BT21-092 never reached the reduced hand-play prompt");
}

#[test]
fn bt21_092_moves_all_digimon_sources_under_tamer_and_reduces_followup_play_by_moved_count() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-092 YAML loads")
        .add_card(xros_digimon("XH-HOST", "Xros Host", 4, 4))
        .add_card(xros_digimon("SOURCE-A", "Source A", 3, 3))
        .add_card(xros_digimon("SOURCE-B", "Source B", 3, 3))
        .add_card(xros_digimon("SOURCE-C", "Source C", 3, 3))
        .add_card(xros_digimon("XH-HAND", "Xros Hand", 5, 6))
        .add_card(tamer("TAMER"))
        .hand(0, &[CARD_ID, "XH-HAND"])
        .memory(10)
        .start();

    let host = runner.place_on_field(0, "XH-HOST", Some(0));
    runner.push_source(host, "SOURCE-A");
    runner.push_source(host, "SOURCE-B");
    runner.push_source(host, "SOURCE-C");
    let tamer = runner.place_on_field(0, "TAMER", Some(0));
    let host_sources_before = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.game.enter_main_phase();
    let memory_before = runner.memory();
    let _ = runner.game.play_option_from_hand(0, 0);
    drive_until_hand_play(&mut runner, 0, "XH-HAND");
    runner.auto_resolve().expect("finish BT21-092");

    let host_sources_after = runner.game.players[0].battle_area[host.index as usize]
        .card_sources
        .len();
    let tamer_sources_after = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();
    assert_eq!(
        host_sources_before - host_sources_after,
        3,
        "BT21-092 should move all three Digimon source cards off the chosen Xros Heart stack"
    );
    assert_eq!(
        tamer_sources_after - tamer_sources_before,
        3,
        "BT21-092 should place the moved source cards under the chosen Tamer"
    );
    assert_eq!(
        memory_before - runner.memory(),
        5,
        "BT21-092 should pay option cost 2 plus XH-HAND cost 6 reduced by three moved cards"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "XH-HAND"),
        "BT21-092 should play the selected Xros Heart card from hand"
    );
}
