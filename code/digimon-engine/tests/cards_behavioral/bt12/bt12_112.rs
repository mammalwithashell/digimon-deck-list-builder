//! BT12-112 Shoutmon X7: Superior Mode - Digimon, Lv.7, White/Red.
//!
//! Acceptance focus for Xros Heart DigiXros substrate:
//! - Its "when you would play this card" hook selects a Shoutmon to pre-attach.
//! - That hook applies a one-shot -1 cost modifier.
//! - The same transaction can then use trash cards as DigiXros materials.

use digimon_engine::action::space::{PASS, TRASH_EFFECT_START};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, PlayerId};

fn xros_material(card_id: &str, name: &str, level: u8) -> CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn push_to_trash(runner: &mut DebugRunner, player: PlayerId, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("push_to_trash: unknown card_id {}", card_id));
    let next_idx = runner.game.next_card_index();
    let card = CardSource::new(data_idx, player, next_idx);
    runner.game.players[player as usize].trash.push(card);
}

fn superior_mode_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT12-112")
        .expect("BT12-112 YAML loads")
        .add_card(xros_material("BT10-008", "Shoutmon", 3))
        .add_card(xros_material("BT10-049", "Ballistamon", 4))
        .memory(20)
        .hand(0, &["BT12-112"])
        .start()
}

#[test]
fn bt12_112_preattaches_shoutmon_reduces_cost_and_unlocks_trash_materials() {
    let mut runner = superior_mode_runner();
    runner.place_on_field(0, "BT10-008", Some(0));
    push_to_trash(&mut runner, 0, "BT10-049");

    let memory_before = runner.memory();
    let _ = runner.play(0, 0);

    let preattach_prompt = runner
        .pending_selection()
        .expect("Superior Mode must ask for a Shoutmon to pre-attach before cost is fixed");
    assert_eq!(preattach_prompt.selecting_player, 0);
    assert!(
        preattach_prompt.is_optional,
        "the printed 'you may' hook must be optional"
    );
    assert!(
        !preattach_prompt.valid_action_ids.is_empty(),
        "Shoutmon in battle area must be selectable for pre-attachment"
    );

    let shoutmon_action = preattach_prompt.valid_action_ids[0];
    runner
        .execute_action(0, shoutmon_action)
        .expect("select Shoutmon for Superior Mode pre-attachment");

    let material_prompt = runner
        .pending_selection()
        .expect("DigiXros transaction should continue with material selection");
    assert!(
        material_prompt
            .valid_action_ids
            .contains(&TRASH_EFFECT_START),
        "accepting the Shoutmon hook must unlock trash as a DigiXros material zone"
    );
    runner
        .execute_action(0, TRASH_EFFECT_START)
        .expect("select Ballistamon from trash as a DigiXros material");
    runner
        .execute_action(0, PASS)
        .expect("stop selecting DigiXros materials");

    assert_eq!(
        memory_before, 10,
        "test setup is clamped to the engine memory ceiling before the play"
    );
    assert_eq!(
        runner.turn_player(),
        1,
        "paying 13 from 10 memory should pass turn with memory on the opponent's side"
    );
    assert_eq!(
        runner.memory(),
        3,
        "BT12-112 cost 15 reduced by -1 Shoutmon and -1 trash material should pay 13, then flip -3 to opponent-side +3"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "pre-attached Shoutmon should leave battle area for the Superior Mode stack"
    );

    let superior = &runner.game.players[0].battle_area[0];
    assert_eq!(
        superior.top_card().card_id(&runner.game.card_data),
        "BT12-112"
    );
    assert!(
        superior
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-008"),
        "selected Shoutmon should be attached as a source"
    );
    assert!(
        superior
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-049"),
        "trash-selected Ballistamon should be attached as a source"
    );
    assert!(
        runner.game.players[0].trash.is_empty(),
        "trash material should be consumed into the DigiXros stack"
    );
}
