use digimon_engine::action::space::{
    HAND_EFFECT_START, PASS, PLAY_HAND_START, SOURCE_SELECT_START,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, PlayerId};
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT11-095";

fn trait_digimon(card_id: &str, name: &str, level: u8, trait_name: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec![trait_name.to_string()];
    card
}

fn filler_digimon(card_id: &str) -> CardData {
    make_test_card(card_id, card_id)
}

fn hand_index(runner: &DebugRunner, player: PlayerId, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|card| card.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

fn hand_contains(runner: &DebugRunner, player: PlayerId, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == card_id)
}

fn resolve_until_hand_pick(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("BT11-095 should keep a pending selection while paying its stash cost");

        if view.kind == SelectionKind::Hand {
            let hand_slot = hand_index(runner, player, card_id) as u16;
            let action = [PLAY_HAND_START + hand_slot, HAND_EFFECT_START + hand_slot]
                .into_iter()
                .find(|action| view.valid_action_ids.contains(action))
                .unwrap_or(HAND_EFFECT_START + hand_slot);
            assert!(
                view.valid_action_ids.contains(&action),
                "{card_id} should be a legal BT11-095 hand-selection action; view={view:?}"
            );
            runner
                .execute_action(player, action)
                .expect("select hand card for BT11-095");
            return;
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .unwrap_or_else(|| panic!("BT11-095 prompt had no non-PASS action: {view:?}"));
        runner
            .execute_action(player, action)
            .expect("advance BT11-095 pending prompt");
    }

    panic!("BT11-095 never installed a hand selection for {card_id}");
}

#[test]
fn bt11_095_start_main_places_xros_or_blue_flare_hand_card_under_self_then_memory_and_draw() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT11-095 YAML loads")
        .add_card(trait_digimon(
            "BLUE-FLARE-HAND",
            "Blue Flare Body",
            4,
            "Blue Flare",
        ))
        .add_card(filler_digimon("NO-TRAIT"))
        .add_card(filler_digimon("DRAW-CARD"))
        .hand(0, &["BLUE-FLARE-HAND", "NO-TRAIT"])
        .deck(0, &["DRAW-CARD"])
        .memory(2)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    let memory_before = runner.memory();
    let deck_before = runner.deck_size(0);
    let stack_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner.game.enter_main_phase();
    resolve_until_hand_pick(&mut runner, 0, "BLUE-FLARE-HAND");
    runner
        .auto_resolve()
        .expect("finish BT11-095 start-main effect");

    let tamer_stack = &runner.game.players[0].battle_area[tamer.index as usize].card_sources;
    assert_eq!(
        tamer_stack.len(),
        stack_before + 1,
        "BT11-095 should place exactly one matching hand card under itself"
    );
    assert!(
        tamer_stack
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BLUE-FLARE-HAND"),
        "the selected Blue Flare Digimon should be under BT11-095"
    );
    assert!(!hand_contains(&runner, 0, "BLUE-FLARE-HAND"));
    assert!(
        hand_contains(&runner, 0, "NO-TRAIT"),
        "non-matching hand cards must remain untouched"
    );
    assert_eq!(
        runner.memory(),
        memory_before + 1,
        "BT11-095 should gain 1 memory after paying the stash cost"
    );
    assert_eq!(
        runner.deck_size(0),
        deck_before - 1,
        "BT11-095 should draw 1 after paying the stash cost"
    );
}

#[test]
fn bt11_095_suspends_to_allow_under_tamer_cards_as_digixros_materials_for_one_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT11-095 YAML loads")
        .dsl_card("BT10-009")
        .expect("BT10-009 YAML loads")
        .add_card(trait_digimon("BT10-008", "Shoutmon", 3, "Xros Heart"))
        .add_card(trait_digimon("BT10-049", "Ballistamon", 4, "Xros Heart"))
        .hand(0, &["BT10-009"])
        .memory(15)
        .start();

    let tamer = runner.place_on_field(0, CARD_ID, Some(0));
    runner.push_source(tamer, "BT10-008");
    runner.place_on_field(0, "BT10-049", Some(0));

    let _ = runner.play(0, 0);

    let modifier_prompt = runner
        .pending_selection()
        .expect("BT11-095 must offer a cast-time DigiXros material-zone prompt");
    assert_eq!(modifier_prompt.selecting_player, 0);
    assert!(
        modifier_prompt.is_optional,
        "BT11-095's under-Tamer DigiXros access is optional"
    );
    let accept = modifier_prompt
        .valid_action_ids
        .first()
        .copied()
        .expect("BT11-095 accept action");
    runner
        .execute_action(0, accept)
        .expect("accept BT11-095 transaction modifier");

    assert!(
        runner.game.players[0].battle_area[tamer.index as usize].is_suspended,
        "accepting BT11-095 must suspend this Tamer as the printed cost"
    );

    let material_prompt = runner
        .pending_selection()
        .expect("DigiXros material selection should follow BT11-095 acceptance");
    let under_tamer_action = material_prompt
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action >= SOURCE_SELECT_START)
        .expect("card under BT11-095 must be selectable as a DigiXros material");
    runner
        .execute_action(0, under_tamer_action)
        .expect("select under-Tamer Shoutmon as DigiXros material");
    if runner
        .pending_selection()
        .is_some_and(|selection| selection.valid_action_ids.contains(&1))
    {
        runner
            .execute_action(0, 1)
            .expect("select Ballistamon from battle area");
    }
    runner
        .execute_action(0, PASS)
        .expect("stop selecting DigiXros materials");

    let x4 = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "BT10-009")
        .expect("Shoutmon X4 should be played");
    assert!(
        x4.card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-008"),
        "Shoutmon from under BT11-095 should become a DigiXros source"
    );
    assert!(
        !runner.game.players[0].battle_area[tamer.index as usize]
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-008"),
        "the chosen under-Tamer source should leave BT11-095's stack"
    );
}
