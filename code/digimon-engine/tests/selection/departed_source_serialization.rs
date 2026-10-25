use digimon_engine::{action::mask::build_action_mask, card_data::CardData,
    runners::HeadlessRunner, search::decision_player, serialization::to_ui_json};

#[test]
fn medusamon_sw_evaluation_departed_source_choice_serializes() {
    let case: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/superpowers/research/2026-10-25-token-choice-regression.json")).unwrap();
    let cards = CardData::load_from_str(&std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json")).unwrap()).unwrap();
    let deck = |key: &str| case[key].as_array().unwrap().iter()
        .map(|c| c.as_str().unwrap().to_owned()).collect();
    let mut runner = HeadlessRunner::new(deck("deck1"), deck("deck2"), &cards,
        false, false, false, Some(case["seed"].as_u64().unwrap())).unwrap();
    for (i, row) in case["prefix"].as_array().unwrap().iter().enumerate() {
        let player = row["player"].as_u64().unwrap() as u8;
        let action = row["action"].as_u64().unwrap() as u16;
        assert_eq!(decision_player(&runner.game), player, "step {i}");
        assert!(build_action_mask(&runner.game, player)[action as usize] > 0.5, "step {i}");
        runner.step(action);
    }
    let pending = runner.game.pending_selection.as_ref().unwrap();
    let choices = pending.effect_choices.as_ref().unwrap();
    assert_eq!(choices.len(), 4);
    for choice in choices {
        assert!(choice.label.starts_with("TOKEN_PETRIFICATION"));
        assert!(runner.game.card_data_for_handle(choice.source_card.unwrap()).is_none());
    }
    let before = build_action_mask(&runner.game, decision_player(&runner.game));
    let json = to_ui_json(&runner.game);
    let serialized = json["pendingSelection"]["effectChoices"].as_array().unwrap();
    assert_eq!(serialized.len(), choices.len());
    for (actual, expected) in serialized.iter().zip(choices) {
        assert_eq!(actual["label"], expected.label);
        assert_eq!(actual["actionId"], expected.action_id);
        assert!(actual["cardId"].is_null());
        assert!(actual["cardName"].is_null());
    }
    assert_eq!(before, build_action_mask(&runner.game, decision_player(&runner.game)));

    // Live sources still expose their card face; an absent source remains a
    // labeled action too. Neither case may filter or renumber the menu.
    let live = runner.game.players[0].hand[0].handle();
    let expected_id = runner.game.card(live).card_id.clone();
    let mut metadata_control = runner.game.clone();
    let control_choices = metadata_control.pending_selection.as_mut().unwrap()
        .effect_choices.as_mut().unwrap();
    control_choices[0].source_card = Some(live);
    control_choices[1].source_card = None;
    let control_json = to_ui_json(&metadata_control);
    assert_eq!(control_json["pendingSelection"]["effectChoices"][0]["cardId"], expected_id);
    assert!(control_json["pendingSelection"]["effectChoices"][1]["cardId"].is_null());
    assert_eq!(control_json["pendingSelection"]["effectChoices"].as_array().unwrap().len(), 4);

    // Cloned search worlds retain the same selectable triggers. Every ordering
    // consumes all four mandatory effects and trashes the available security.
    let actions = pending.valid_action_ids.clone();
    let security_before = runner.game.players[1].security.len();
    for first in actions {
        let mut world = runner.game.clone();
        let mut action = first;
        for _ in 0..4 {
            let player = decision_player(&world);
            assert!(build_action_mask(&world, player)[action as usize] > 0.5);
            world.decode_action(action, player);
            to_ui_json(&world);
            match world.pending_selection.as_ref() {
                Some(selection) if selection.effect_choices.as_ref().is_some_and(|choices|
                    choices.iter().all(|choice| choice.label.starts_with("TOKEN_PETRIFICATION"))) => {
                    action = selection.valid_action_ids[0];
                }
                _ => break,
            }
        }
        assert_eq!(world.players[1].security.len(), security_before.saturating_sub(4));
        assert!(!world.effect_queue.iter().any(|effect| effect.card_id == "TOKEN_PETRIFICATION"));
    }
}
