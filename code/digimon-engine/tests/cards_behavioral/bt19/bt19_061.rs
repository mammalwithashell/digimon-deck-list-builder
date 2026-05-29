use digimon_engine::action::space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const CARD_ID: &str = "BT19-061";

fn trait_digimon(card_id: &str, name: &str, trait_name: &str) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, 4);
    card.colors = vec![CardColor::Purple];
    card.traits = vec![trait_name.to_string()];
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

#[test]
fn bt19_061_declares_sparrowmon_digixros_alias_without_generic_identity_alias() {
    let runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-061 YAML loads")
        .start();
    let compiled = runner
        .compiled_card(CARD_ID)
        .expect("BT19-061 compiled card should be registered");

    assert_eq!(
        compiled.digixros_aliases,
        vec!["Sparrowmon".to_string()],
        "BT19-061 is treated as Sparrowmon for DigiXros only"
    );
    assert!(
        compiled.identity.is_none(),
        "DigiXros-only treated-as text must not become a generic identity alias"
    );
}

#[test]
fn bt19_061_on_deletion_places_matching_hand_or_trash_card_under_own_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT19-061 YAML loads")
        .add_card(trait_digimon("XH-HAND", "Xros Hand", "Xros Heart"))
        .add_card(trait_digimon("BF-TRASH", "Blue Flare Trash", "Blue Flare"))
        .add_card(tamer("TAMER"))
        .hand(0, &["XH-HAND"])
        .memory(10)
        .start();

    push_to_trash(&mut runner, 0, "BF-TRASH");
    let raptor = runner.place_on_field(0, CARD_ID, Some(0));
    let tamer = runner.place_on_field(0, "TAMER", Some(0));
    let tamer_sources_before = runner.game.players[0].battle_area[tamer.index as usize]
        .card_sources
        .len();

    runner
        .game
        .delete_permanent_with_cause(raptor, ReplacementCause::OpponentEffect);

    for _ in 0..8 {
        let view = runner
            .pending_selection_view()
            .expect("BT19-061 should keep a pending selection until the stash pick");
        if matches!(
            view.kind,
            SelectionKind::UnionZone { .. } | SelectionKind::Hand | SelectionKind::Trash
        ) {
            let hand_action = PLAY_HAND_START + hand_index(&runner, 0, "XH-HAND") as u16;
            let trash_action = TRASH_EFFECT_START + trash_index(&runner, 0, "BF-TRASH") as u16;
            assert!(
                view.valid_action_ids.contains(&hand_action),
                "Xros Heart card in hand should be legal"
            );
            assert!(
                view.valid_action_ids.contains(&trash_action),
                "Blue Flare card in trash should be legal"
            );
            runner
                .execute_action(0, trash_action)
                .expect("select trash card to place under Tamer");
            break;
        }

        let action = view
            .valid_action_ids
            .iter()
            .copied()
            .find(|&action| action != PASS)
            .expect("BT19-061 intermediate prompt should have a non-PASS action");
        runner
            .execute_action(0, action)
            .expect("advance BT19-061 deletion prompt");
    }
    runner
        .auto_resolve()
        .expect("finish BT19-061 deletion flow");

    let tamer_sources = &runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "TAMER")
        .expect("destination Tamer remains on field")
        .card_sources;
    assert_eq!(
        tamer_sources.len(),
        tamer_sources_before + 1,
        "BT19-061 should place one selected card under the chosen Tamer"
    );
    assert!(
        tamer_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BF-TRASH"),
        "the selected Blue Flare trash card should be under the Tamer"
    );
}
