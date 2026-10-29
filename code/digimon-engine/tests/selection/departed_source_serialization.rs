use digimon_engine::{
    action::{mask::build_action_mask, space::PASS},
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    effect_context::EffectContext,
    enums::{CardKind, EffectTiming},
    replacement::ReplacementCause,
    search::decision_player,
    selection::SelectionKind,
    serialization::to_ui_json,
};

#[test]
fn medusamon_sw_evaluation_departed_source_choice_serializes() {
    // The archived October 25 replay is historical evidence; its prefix used
    // older timing semantics and no longer reaches the failing state.
    // Recreate that state's actual cause through the token and batch-deletion
    // APIs, without restoring vanished cards or fabricating pending choices.
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["FILL"])
        .deck(0, &["FILL"; 4])
        .deck(1, &["FILL"; 4])
        .security(1, &["FILL"; 5])
        .memory(0)
        .start();
    let tokens: Vec<_> = (0..4)
        .map(|_| {
            EffectContext::new(&mut runner.game, CardHandle(0), None, 1)
                .play_token(1, "petrification")
                .expect("a real Petrification Token is played")
        })
        .collect();
    let sources: Vec<_> = tokens
        .iter()
        .map(|token| {
            let card = runner.game.players[1].battle_area[token.index as usize].top_card();
            assert_eq!(card.card_kind(&runner.game.card_data), CardKind::Token);
            assert_eq!(card.card_id(&runner.game.card_data), "TOKEN_PETRIFICATION");
            card.handle()
        })
        .collect();
    assert_eq!(
        sources
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    let outcome = runner
        .game
        .delete_permanents_batch(tokens, ReplacementCause::OwnEffect);
    assert_eq!(outcome.completed.len(), 4);
    assert!(outcome.cancelled.is_empty());
    runner.game.drain_effect_queue();
    assert!(runner.game.players[1].battle_area.is_empty());
    assert!(sources
        .iter()
        .all(|source| runner.game.card_data_for_handle(*source).is_none()));
    assert_eq!(
        runner.game.players[1].security.len(),
        5,
        "the mandatory effects await ordering"
    );
    let queued: Vec<_> = runner
        .game
        .effect_queue
        .iter()
        .filter(|effect| effect.card_id == "TOKEN_PETRIFICATION")
        .collect();
    assert_eq!(queued.len(), 4);
    for effect in queued {
        assert_eq!(effect.timing, EffectTiming::OnDeletion);
        assert!(!effect.is_optional);
        assert!(sources.contains(&effect.source_card));
    }
    let pending = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(pending.kind, SelectionKind::TriggerOrder);
    assert_eq!(pending.selecting_player, 1);
    assert!(!pending.is_optional);
    assert_eq!(pending.valid_action_ids.len(), 4);
    let choices = pending.effect_choices.as_ref().unwrap();
    assert_eq!(choices.len(), 4);
    for choice in choices {
        assert!(choice.label.starts_with("TOKEN_PETRIFICATION"));
        assert!(sources.contains(&choice.source_card.unwrap()));
        assert!(runner
            .game
            .card_data_for_handle(choice.source_card.unwrap())
            .is_none());
    }
    let before = build_action_mask(&runner.game, decision_player(&runner.game));
    assert_eq!(
        before[PASS as usize], 0.0,
        "mandatory effects cannot be declined"
    );
    let json = to_ui_json(&runner.game);
    let serialized = json["pendingSelection"]["effectChoices"]
        .as_array()
        .unwrap();
    assert_eq!(serialized.len(), choices.len());
    for (actual, expected) in serialized.iter().zip(choices) {
        assert_eq!(actual["label"], expected.label);
        assert_eq!(actual["actionId"], expected.action_id);
        assert!(actual["cardId"].is_null());
        assert!(actual["cardName"].is_null());
    }
    assert_eq!(
        before,
        build_action_mask(&runner.game, decision_player(&runner.game))
    );

    // Live sources still expose their card face; an absent source remains a
    // labeled action too. Neither case may filter or renumber the menu.
    let live = runner.game.players[0].hand[0].handle();
    let expected_id = runner.game.card(live).card_id.clone();
    let mut metadata_control = runner.game.clone();
    let control_choices = metadata_control
        .pending_selection
        .as_mut()
        .unwrap()
        .effect_choices
        .as_mut()
        .unwrap();
    control_choices[0].source_card = Some(live);
    control_choices[1].source_card = None;
    let control_json = to_ui_json(&metadata_control);
    assert_eq!(
        control_json["pendingSelection"]["effectChoices"][0]["cardId"],
        expected_id
    );
    assert!(control_json["pendingSelection"]["effectChoices"][1]["cardId"].is_null());
    assert_eq!(
        control_json["pendingSelection"]["effectChoices"]
            .as_array()
            .unwrap()
            .len(),
        4
    );

    // Cloned search worlds retain the same selectable triggers. Each possible
    // first choice consumes all four mandatory effects and trashes security.
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
                Some(selection)
                    if selection.effect_choices.as_ref().is_some_and(|choices| {
                        choices
                            .iter()
                            .all(|choice| choice.label.starts_with("TOKEN_PETRIFICATION"))
                    }) =>
                {
                    action = selection.valid_action_ids[0];
                }
                _ => break,
            }
        }
        assert_eq!(
            world.players[1].security.len(),
            security_before.saturating_sub(4)
        );
        assert!(!world
            .effect_queue
            .iter()
            .any(|effect| effect.card_id == "TOKEN_PETRIFICATION"));
    }
}
