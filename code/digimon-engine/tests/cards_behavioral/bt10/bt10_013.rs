//! BT10-013 Shoutmon X5 - Digimon, Lv.5, Red.
//!
//! Acceptance focus for Xros Heart Material Save:
//! - <Material Save 3> is offered when the carrier would be deleted.
//! - The prompt is optional.
//! - Only source cards matching the printed DigiXros requirements are eligible.
//! - Selected sources are placed under a chosen Tamer.

use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;

fn xros_material(card_id: &str, name: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, level);
    card.colors = vec![CardColor::Red];
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn test_tamer(card_id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card.play_cost = 3;
    card.traits = vec!["Xros Heart".to_string()];
    card
}

fn runner_with_x5() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT10-013")
        .expect("BT10-013 YAML loads")
        .add_card(xros_material("BT10-008", "Shoutmon", 3))
        .add_card(xros_material("BT10-049", "Ballistamon", 4))
        .add_card(xros_material("BT10-034", "Dorulumon", 4))
        .add_card(xros_material("BT10-029", "Starmons", 3))
        .add_card(xros_material("BT10-060", "Sparrowmon", 3))
        .add_card(make_test_card_with_level("BT10-UNRELATED", "Unrelated", 3))
        .add_card(test_tamer("XROS-TAMER", "Xros Heart Tamer"))
        .memory(10)
        .start()
}

#[test]
fn bt10_013_material_save_three_is_optional_and_filters_recipe_sources_under_tamer() {
    let mut runner = runner_with_x5();
    let x5 = runner.place_stack(
        0,
        &[
            "BT10-008",
            "BT10-049",
            "BT10-034",
            "BT10-UNRELATED",
            "BT10-013",
        ],
    );
    runner.place_on_field(0, "XROS-TAMER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(x5, ReplacementCause::OpponentEffect);

    let tamer_prompt = runner
        .pending_selection()
        .expect("Material Save must ask which Tamer receives saved sources");
    assert_eq!(tamer_prompt.selecting_player, 0);
    assert!(
        tamer_prompt.is_optional,
        "Material Save uses 'you may' and must be declinable at the Tamer destination prompt"
    );
    assert!(
        !tamer_prompt.valid_action_ids.is_empty(),
        "the own Xros Heart Tamer must be a legal destination"
    );

    let tamer_action = tamer_prompt.valid_action_ids[0];
    runner
        .execute_action(0, tamer_action)
        .expect("choose destination Tamer");

    for _ in 0..3 {
        let source_prompt = runner
            .pending_selection()
            .expect("Material Save should ask for up to 3 eligible recipe sources");
        let action = source_prompt.valid_action_ids[0];
        assert!(
            source_prompt.valid_action_ids.contains(&action),
            "recipe source action {action} should be selectable"
        );
        runner
            .execute_action(0, action)
            .expect("select eligible Material Save source");
    }
    let tamer_perm = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "XROS-TAMER")
        .expect("chosen Tamer remains on the field");
    let tamer_source_ids: Vec<_> = tamer_perm
        .card_sources
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    for expected in ["BT10-008", "BT10-049", "BT10-034"] {
        assert!(
            tamer_source_ids.iter().any(|id| id == expected),
            "{expected} should be saved under the chosen Tamer"
        );
    }
    assert!(
        !tamer_source_ids.iter().any(|id| id == "BT10-UNRELATED"),
        "non-recipe source must not be eligible for Material Save"
    );
}
