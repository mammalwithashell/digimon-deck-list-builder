//! BT10-087 Taiki Kudo - Tamer, Red.
//!
//! Acceptance focus for Xros Heart DigiXros substrate:
//! - [Your Turn] when playing a Digimon with DigiXros requirements, suspending
//!   Taiki allows cards under Tamers to be used as DigiXros materials for that
//!   play transaction.

use digimon_engine::action::space::{PASS, SOURCE_SELECT_START};
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;

fn xros_material(card_id: &str, name: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn runner_with_taiki_and_x4() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT10-087")
        .expect("BT10-087 YAML loads")
        .dsl_card("BT10-009")
        .expect("BT10-009 YAML loads")
        .add_card(xros_material("BT10-008", "Shoutmon", 3))
        .add_card(xros_material("BT10-049", "Ballistamon", 4))
        .memory(15)
        .hand(0, &["BT10-009"])
        .deck(0, &["BT10-008", "BT10-049"])
        .start()
}

#[test]
fn bt10_087_suspends_to_allow_under_tamer_digixros_materials_for_one_play() {
    let mut runner = runner_with_taiki_and_x4();
    let taiki = runner.place_on_field(0, "BT10-087", Some(0));
    runner.push_source(taiki, "BT10-008");
    runner.place_on_field(0, "BT10-049", Some(0));

    let _ = runner.play(0, 0);

    let modifier_prompt = runner
        .pending_selection()
        .expect("Taiki must offer a cast-time DigiXros modifier prompt");
    assert_eq!(modifier_prompt.selecting_player, 0);
    assert!(
        modifier_prompt.is_optional,
        "Taiki's 'you may place cards from under your Tamers' effect is optional"
    );
    assert!(
        !modifier_prompt.valid_action_ids.is_empty(),
        "Taiki prompt must expose an accept action through pending selection"
    );

    let accept = modifier_prompt
        .valid_action_ids
        .first()
        .copied()
        .expect("accept action");
    runner
        .execute_action(0, accept)
        .expect("accept Taiki transaction modifier");

    assert!(
        runner.game.players[0].battle_area[taiki.index as usize].is_suspended,
        "accepting Taiki must suspend this Tamer as the printed cost"
    );

    let material_prompt = runner
        .pending_selection()
        .expect("DigiXros transaction should ask for materials after accepting Taiki");
    assert!(
        !material_prompt.valid_action_ids.is_empty(),
        "the card under Taiki must be selectable as a DigiXros material after the modifier"
    );

    let under_tamer_action = material_prompt
        .valid_action_ids
        .iter()
        .copied()
        .find(|action| *action >= SOURCE_SELECT_START)
        .expect("under-Tamer source action");
    runner
        .execute_action(0, under_tamer_action)
        .expect("select Shoutmon from under Taiki as DigiXros material");
    if runner
        .pending_selection()
        .is_some_and(|sel| sel.valid_action_ids.contains(&1))
    {
        runner
            .execute_action(0, 1)
            .expect("select Ballistamon from battle area as a second material");
    }
    runner
        .execute_action(0, PASS)
        .expect("stop selecting DigiXros materials");

    let x4 = runner.game.players[0]
        .battle_area
        .iter()
        .find(|perm| perm.top_card().card_id(&runner.game.card_data) == "BT10-009")
        .expect("Shoutmon X4 should be played");
    assert!(
        x4.card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-008"),
        "Shoutmon from under Taiki should be attached to the DigiXros stack"
    );
    assert!(
        !runner.game.players[0].battle_area[taiki.index as usize]
            .card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-008"),
        "the chosen under-Tamer source should leave Taiki's source stack"
    );
}
