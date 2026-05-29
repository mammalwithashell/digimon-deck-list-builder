//! BT7-107 Calling From the Darkness.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledStep, CompiledTiming};
use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::TRASH_EFFECT_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

const YAML: &str = include_str!("../../../cards/bt7/BT7-107.yaml");

#[test]
fn bt7_107_main_deletes_own_digimon_and_returns_up_to_two_purple_digimon() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT7-107 YAML loads")
        .add_card(purple_digimon("PURPLE-1"))
        .add_card(purple_digimon("PURPLE-2"))
        .add_card(purple_digimon("SACRIFICE"))
        .hand(0, &["BT7-107"])
        .memory(10)
        .start();
    push_to_trash(&mut runner, 0, "PURPLE-1");
    push_to_trash(&mut runner, 0, "PURPLE-2");
    runner.place_on_field(0, "SACRIFICE", Some(0));

    runner.play(0, 0);
    let sacrifice_pick = runner
        .pending_selection_view()
        .expect("delete own Digimon")
        .valid_action_ids[0];
    assert_mask_allows(&runner, 0, sacrifice_pick);
    runner
        .execute_action(0, sacrifice_pick)
        .expect("delete own Digimon");
    assert_mask_allows(&runner, 0, TRASH_EFFECT_START);
    runner
        .execute_action(0, TRASH_EFFECT_START)
        .expect("return first purple");
    assert_mask_allows(&runner, 0, TRASH_EFFECT_START);
    runner
        .execute_action(0, TRASH_EFFECT_START)
        .expect("return second purple");

    let hand = zone_ids(&runner.game.players[0].hand, &runner.game.card_data);
    assert!(hand.contains(&"PURPLE-1".to_string()));
    assert!(hand.contains(&"PURPLE-2".to_string()));
    assert!(trash_contains(&runner, 0, "SACRIFICE"));
}

#[test]
fn bt7_107_security_adds_this_option_to_hand() {
    let runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT7-107 YAML loads")
        .start();
    let compiled = runner.compiled_card("BT7-107").expect("compiled card");
    let security = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity) => Some(t),
            _ => None,
        })
        .expect("security clause");

    assert_eq!(security.scope, CompiledScope::Inherited);
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddThisOptionToHand)));
}

fn purple_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card
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
        .push(digimon_engine::card_source::CardSource::new(
            data_idx, player, card_index,
        ));
}

fn trash_contains(runner: &DebugRunner, player: u8, id: &str) -> bool {
    runner.game.players[player as usize]
        .trash
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == id)
}

fn zone_ids(cards: &[digimon_engine::card_source::CardSource], data: &[CardData]) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(data).to_string())
        .collect()
}

fn assert_mask_allows(runner: &DebugRunner, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(&runner.game, player)[action as usize],
        1.0,
        "pending action {action} must be legal in the action mask"
    );
}
