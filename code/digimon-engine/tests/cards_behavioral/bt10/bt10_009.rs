//! BT10-009 Shoutmon X4 - Digimon, Lv.4, Red.
//!
//! Acceptance focus for Xros Heart DigiXros substrate:
//! - DigiXros -2 with Shoutmon, Ballistamon, Dorulumon, and Starmons.
//! - Playing with selected materials reduces play cost per material.
//! - Selected materials become digivolution cards beneath Shoutmon X4.
//! - The resolving play records a DigiXros material count.

use digimon_dsl::compiled::CompiledAltPathKind;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;

fn xros_material(card_id: &str, name: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn runner_with_bt10_009() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT10-009")
        .expect("BT10-009 YAML loads")
        .add_card(xros_material("BT10-008", "Shoutmon", 3))
        .add_card(xros_material("BT10-049", "Ballistamon", 4))
        .add_card(xros_material("BT10-034", "Dorulumon", 4))
        .add_card(xros_material("BT10-029", "Starmons", 3))
        .memory(15)
        .hand(0, &["BT10-009"])
        .deck(0, &["BT10-008", "BT10-049"])
        .start()
}

#[test]
fn bt10_009_declares_digixros_x4_recipe() {
    let runner = runner_with_bt10_009();
    let card = runner
        .compiled_card("BT10-009")
        .expect("BT10-009 compiled card present");

    assert_eq!(card.name, "Shoutmon X4");
    assert_eq!(card.level, Some(4));
    assert_eq!(card.cost, Some(9));

    let digixros = card
        .alt_paths
        .iter()
        .find(|path| path.kind == CompiledAltPathKind::DigiXros)
        .expect("BT10-009 must declare a DigiXros alt path");

    for required_name in ["Shoutmon", "Ballistamon", "Dorulumon", "Starmons"] {
        assert!(
            digixros
                .materials
                .iter()
                .any(|mat| mat.filter.name_is.as_deref() == Some(required_name)),
            "DigiXros recipe must include {required_name}"
        );
    }
}

#[test]
fn bt10_009_digixros_two_field_materials_reduce_cost_and_attach_sources() {
    let mut runner = runner_with_bt10_009();
    runner.place_on_field(0, "BT10-008", Some(0));
    runner.place_on_field(0, "BT10-049", Some(0));

    let memory_before = runner.memory();
    let _ = runner.play(0, 0);

    let first_prompt = runner
        .pending_selection()
        .expect("DigiXros play must ask for recipe materials before paying cost");
    assert_eq!(first_prompt.selecting_player, 0);
    assert!(
        first_prompt.is_optional,
        "DigiXros material selection must allow stopping with fewer than all recipe materials"
    );
    assert!(
        first_prompt.valid_action_ids.contains(&0),
        "Shoutmon in battle-area slot 0 must be selectable as DigiXros material"
    );
    assert!(
        first_prompt.valid_action_ids.contains(&1),
        "Ballistamon in battle-area slot 1 must be selectable as DigiXros material"
    );

    runner
        .execute_action(0, 0)
        .expect("select Shoutmon as first DigiXros material");
    let second_prompt = runner
        .pending_selection()
        .expect("DigiXros transaction should continue after first material");
    assert!(
        second_prompt.valid_action_ids.contains(&1),
        "Ballistamon remains selectable as second DigiXros material"
    );
    runner
        .execute_action(0, 1)
        .expect("select Ballistamon as second DigiXros material");
    runner
        .execute_action(0, PASS)
        .expect("stop selecting DigiXros materials");

    assert_eq!(
        memory_before - runner.memory(),
        5,
        "BT10-009 play cost 9 reduced by two DigiXros -2 materials should cost 5"
    );
    assert_eq!(
        runner.battle_area_size(0),
        1,
        "selected field materials should leave battle area and become sources"
    );

    let x4 = &runner.game.players[0].battle_area[0];
    assert_eq!(x4.top_card().card_id(&runner.game.card_data), "BT10-009");
    assert_eq!(
        x4.card_sources.len(),
        3,
        "Shoutmon X4 stack should contain two selected sources plus the top card"
    );
    assert!(
        x4.card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-008"),
        "Shoutmon source should be attached"
    );
    assert!(
        x4.card_sources
            .iter()
            .any(|card| card.card_id(&runner.game.card_data) == "BT10-049"),
        "Ballistamon source should be attached"
    );
}
