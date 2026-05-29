use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{
    EFFECTS_PER_PERMANENT, FIELD_EFFECT_SLOT_FOR_MAIN, FIELD_EFFECT_START, PASS, PLAY_HAND_START,
    SOURCE_SELECT_START, TRASH_EFFECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, PlayerId};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "P-224";

fn trait_digimon(card_id: &str, name: &str, level: u8, trait_name: &str, cost: u16) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Purple];
    card.traits = vec![trait_name.to_string()];
    card.play_cost = cost;
    card
}

fn filler_digimon(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

fn push_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_index = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("unknown card id {card_id}"));
    let instance_id = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(CardSource::new(data_index, player, instance_id));
}

fn hand_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn trash_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .trash
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s trash"))
}

fn trash_contains(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

fn resolve_until_hand_or_trash_pick(
    runner: &mut DebugRunner,
    player: PlayerId,
    hand_card: &str,
    trash_card: &str,
) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("P-224 should keep a pending selection while paying its stash cost");

        match view.kind {
            SelectionKind::UnionZone { .. } | SelectionKind::Hand | SelectionKind::Trash => {
                let hand_action = PLAY_HAND_START + hand_index(runner, player, hand_card) as u16;
                let trash_action =
                    TRASH_EFFECT_START + trash_index(runner, player, trash_card) as u16;
                assert!(
                    view.valid_action_ids.contains(&hand_action),
                    "{hand_card} in hand should be legal for P-224; view={view:?}"
                );
                assert!(
                    view.valid_action_ids.contains(&trash_action),
                    "{trash_card} in trash should be legal for P-224; view={view:?}"
                );
                runner
                    .execute_action(player, trash_action)
                    .expect("select trash card for P-224");
                return;
            }
            _ => {}
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .unwrap_or_else(|| panic!("P-224 prompt had no non-PASS action: {view:?}"));
        runner
            .execute_action(player, action)
            .expect("advance P-224 pending prompt");
    }

    panic!("P-224 never installed a hand-or-trash selection");
}

#[test]
fn p_224_on_play_places_xros_or_twilight_from_hand_or_trash_under_self_and_draws_if_hand_small() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-224 YAML loads")
        .add_card(trait_digimon("XH-HAND", "Xros Hand", 3, "Xros Heart", 3))
        .add_card(trait_digimon(
            "TWILIGHT-TRASH",
            "Twilight Trash",
            4,
            "Twilight",
            4,
        ))
        .add_card(filler_digimon("DRAW-CARD"))
        .hand(0, &["XH-HAND"])
        .deck(0, &["DRAW-CARD"])
        .memory(3)
        .start();

    push_to_trash(&mut runner, 0, "TWILIGHT-TRASH");
    let kotone = runner.place_on_field(0, CARD_ID, Some(0));
    let deck_before = runner.deck_size(0);
    let stack_before = runner.game.players[0].battle_area[kotone.index as usize]
        .card_sources
        .len();

    runner.fire_on_play(0, kotone.index as usize);
    resolve_until_hand_or_trash_pick(&mut runner, 0, "XH-HAND", "TWILIGHT-TRASH");
    runner.auto_resolve().expect("finish P-224 on-play stash");

    let kotone_stack = &runner.game.players[0].battle_area[kotone.index as usize].card_sources;
    assert_eq!(
        kotone_stack.len(),
        stack_before + 1,
        "P-224 should place exactly one selected card under itself"
    );
    assert!(
        kotone_stack
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "TWILIGHT-TRASH"),
        "the selected Twilight Digimon from trash should be under P-224"
    );
    assert!(
        !trash_contains(&runner, 0, "TWILIGHT-TRASH"),
        "the selected trash card must leave trash"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "P-224 should draw 1 when its controller has 7 or fewer cards in hand"
    );
}

#[test]
fn p_224_main_suspends_to_play_level5_or_higher_xros_heart_from_under_tamers_at_cost_minus_one() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("P-224 YAML loads")
        .add_card(trait_digimon(
            "XH-LV5",
            "Xros Heart Lv5",
            5,
            "Xros Heart",
            6,
        ))
        .add_card(trait_digimon(
            "XH-LV4",
            "Xros Heart Lv4",
            4,
            "Xros Heart",
            3,
        ))
        .memory(10)
        .start();

    let kotone = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(kotone, "XH-LV5");
    runner.push_source(kotone, "XH-LV4");

    runner.game.enter_main_phase();
    let activation = FIELD_EFFECT_START
        + kotone.index as u16 * EFFECTS_PER_PERMANENT
        + FIELD_EFFECT_SLOT_FOR_MAIN;
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[activation as usize], 1.0,
        "P-224 [Main] should be a visible field-effect activation"
    );
    let memory_before = runner.memory();

    runner.game.decode_action(activation, 0);

    let source_prompt = runner
        .pending_selection_view()
        .expect("P-224 should ask which under-Tamer card to play");
    let lv5_action = source_prompt
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action >= SOURCE_SELECT_START)
        .expect("eligible level 5+ Xros Heart source should be selectable");
    runner
        .execute_action(0, lv5_action)
        .expect("select level 5+ Xros Heart under a Tamer");
    runner
        .auto_resolve()
        .expect("finish P-224 under-Tamer play");

    assert!(
        runner.game.players[0].battle_area[kotone.index as usize].is_suspended,
        "P-224 must suspend itself as the printed [Main] cost"
    );
    assert_eq!(
        memory_before - runner.memory(),
        5,
        "P-224 should play the selected level 5+ Xros Heart Digimon with cost reduced by 1"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "XH-LV5"),
        "the selected level 5+ card from under a Tamer should be played"
    );
    assert!(
        !runner.game.players[0].battle_area[kotone.index as usize]
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "XH-LV5"),
        "the played card should leave the Tamer stack"
    );
}
