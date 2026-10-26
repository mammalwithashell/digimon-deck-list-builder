//! Regressions from capped self-play games 340 and 850: spent Main activations
//! and nested leave continuations must survive selection and search cloning.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, ATTACK_START, PASS, REPLACEMENT_ACCEPT, SECURITY_TARGET},
    },
    card_data::CardData,
    debug_runner::DebugRunner,
    enums::GamePhase,
    runners::HeadlessRunner,
    selection::SelectionKind,
    serialization::to_ui_json,
};

#[test]
fn capped_game_850_resumes_after_nested_replacement_and_block() {
    // Preserve game 850's actual card path without replaying 179 historical
    // numeric choices. That prefix now diverges at decision 41: PASS tries to
    // skip mandatory BT26-092/BT26-009 start-main triggers. The historical JSON
    // remains unchanged. Here Chronomon's real attack effect deletes MarineBullmon;
    // Sanmyojin Arrival owns the leave and must preserve its nested Decode
    // choice before offering the Ryugumon reward. Susanoomon then really blocks.
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut cards = CardData::load_from_str(&raw).unwrap();
    let ids = [
        "BT26-016", "EX12-031", "EX12-070", "EX12-036", "EX12-076", "EX12-026", "BT1-019",
    ];
    let mut builder = DebugRunner::builder();
    for id in ids {
        builder = builder.add_card(cards.remove(id).unwrap());
        if id != "BT1-019" {
            builder = builder.dsl_card(id).unwrap();
        }
    }
    let mut runner = builder
        .deck(0, &["BT1-019"; 12])
        .deck(1, &["BT1-019"; 12])
        .hand(0, &["BT1-019", "BT1-019"])
        .hand(1, &["EX12-036", "EX12-026"])
        .security(0, &["BT1-019"; 3])
        .security(1, &["BT1-019"; 3])
        .memory(10)
        .start();
    runner.set_phase(GamePhase::Main);
    let attacker = runner.place_on_field(0, "BT26-016", Some(0));
    let arrival = runner.place_on_field(1, "EX12-070", Some(0));
    runner.game.players[1].battle_area[arrival.index as usize].option_state =
        digimon_engine::permanent::OptionState::Delayed {
            owner: 1,
            trash_on_turn: u16::MAX,
            trigger: digimon_engine::enums::DelayTrigger::ExternallyGated,
            placed_on_turn: 0,
        };
    runner.place_on_field(1, "EX12-076", Some(0));
    let marine = runner.place_on_field(1, "EX12-031", Some(0));
    runner.push_source(marine, "EX12-026");
    let attack_action = encode_attack(attacker.index as u16, SECURITY_TARGET);
    assert_eq!(
        build_action_mask(&runner.game, 0)[attack_action as usize],
        1.0
    );
    runner.game.decode_action(attack_action, 0);
    let delete_prompt = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Chronomon attack deletion choice");
    assert!(
        delete_prompt.prompt.contains("delete 1 of your opponent"),
        "{delete_prompt:?}"
    );
    let delete_marine = ATTACK_START + marine.index as u16;
    assert_eq!(
        build_action_mask(&runner.game, 0)[delete_marine as usize],
        1.0
    );
    runner.game.resolve_selection(0, delete_marine).unwrap();
    let outer = runner
        .game
        .pending_selection
        .as_ref()
        .expect("Arrival replacement before Decode");
    assert_eq!(outer.kind, SelectionKind::Replacement, "{outer:?}");
    assert_eq!(
        runner.game.card(outer.source_card).card_id,
        "EX12-070",
        "{outer:?}"
    );
    assert_eq!(
        build_action_mask(&runner.game, 1)[REPLACEMENT_ACCEPT as usize],
        1.0
    );
    runner
        .game
        .resolve_selection(1, REPLACEMENT_ACCEPT)
        .unwrap();

    // Previously overwritten by the hand reward (whose previous_phase then
    // incorrectly became EffectChoice). The repaired game exposes this choice.
    let nested = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(nested.kind, SelectionKind::Replacement, "{nested:?}");
    assert_eq!(
        runner.game.card(nested.source_card).card_id,
        "EX12-031",
        "{nested:?}"
    );
    assert!(
        nested.prompt.contains("when_would_leave_battle_area"),
        "{nested:?}"
    );
    assert_eq!(nested.previous_phase, GamePhase::Main);
    let player = nested.selecting_player;
    for accept_nested in [false, true] {
        for play_reward in [false, true] {
            let mut game = runner.game.clone();
            game.resolve_selection(
                player,
                if accept_nested {
                    REPLACEMENT_ACCEPT
                } else {
                    PASS
                },
            )
            .unwrap();
            let mut cloned = game.clone();
            let mut saw_reward = false;
            let mut saw_block = false;
            for _ in 0..40 {
                let Some(selection) = game.pending_selection.as_ref() else {
                    break;
                };
                assert_eq!(
                    selection.previous_phase,
                    GamePhase::Main,
                    "{}",
                    selection.prompt
                );
                let pid = selection.selecting_player;
                let mask = build_action_mask(&game, pid);
                assert_eq!(mask, build_action_mask(&cloned, pid));
                let reward = selection.prompt
                    == "Play 1 [Sanmyojin] Digimon from hand without paying the cost";
                saw_reward |= reward;
                let action = if reward && play_reward {
                    selection.valid_action_ids[0]
                } else if game.current_phase == GamePhase::BlockTiming {
                    let blocker = game.players[1]
                        .battle_area
                        .iter()
                        .position(|p| p.top_card().card_id(&game.card_data) == "EX12-076")
                        .expect("Susanoomon remains");
                    let block = ATTACK_START + blocker as u16;
                    assert_eq!(
                        mask[block as usize], 1.0,
                        "Susanoomon must be a legal blocker"
                    );
                    saw_block = true;
                    block
                } else if mask[PASS as usize] > 0.0 {
                    PASS
                } else {
                    selection.valid_action_ids[0]
                };
                assert!(mask[action as usize] > 0.0);
                game.decode_action(action, pid);
                cloned.decode_action(action, pid);
                assert_eq!(to_ui_json(&game), to_ui_json(&cloned));
            }
            assert!(
                saw_reward,
                "the suspended Delay reward must still be offered"
            );
            assert!(
                saw_block,
                "the scenario must include Susanoomon's real block"
            );
            assert!(
                game.pending_selection.is_none(),
                "nested choices must finish"
            );
            assert!(game.pending_attack.is_none(), "combat must finish");
            if game.game_over {
                assert_eq!(game.current_phase, GamePhase::GameOver);
                assert!(
                    game.winner.is_some(),
                    "a loop draw must not count as repaired"
                );
                continue;
            }
            assert_eq!(game.current_phase, GamePhase::Main);
            assert!(
                build_action_mask(&game, game.turn_player())
                    .iter()
                    .filter(|v| **v > 0.0)
                    .count()
                    > 1,
                "real main-phase choices must replace the orphan PASS fallback"
            );
        }
    }
}

#[test]
fn capped_game_340_cannot_reopen_manekimon_after_declining() {
    let evidence: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/superpowers/research/2026-10-25-cap-investigation.json",
    ))
    .unwrap();
    let case = evidence["games"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["game"] == 340)
        .unwrap();
    let cards = CardData::load_from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/cards.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let deck = |name: &str| {
        case[name]["cards"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_owned())
            .collect()
    };
    let mut runner = HeadlessRunner::new(
        deck("deck1"),
        deck("deck2"),
        &cards,
        false,
        false,
        false,
        Some(case["seed"].as_u64().unwrap()),
    )
    .unwrap();
    for (i, step) in case["prefix_actions"]
        .as_array()
        .unwrap()
        .iter()
        .take(94)
        .enumerate()
    {
        let player = step["player"].as_u64().unwrap() as u8;
        let action = step["action"].as_u64().unwrap() as u16;
        assert!(
            build_action_mask(&runner.game, player)[action as usize] > 0.0,
            "decision {i}"
        );
        runner.game.decode_action(action, player);
    }
    let pid = runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .selecting_player;
    let hand: Vec<_> = runner.game.players[pid as usize]
        .hand
        .iter()
        .map(|c| c.handle())
        .collect();
    let memory = runner.game.memory;
    let mut cloned = runner.game.clone();
    for game in [&mut runner.game, &mut cloned] {
        game.resolve_selection(pid, PASS).unwrap();
        assert!(game.pending_selection.is_none());
        assert_eq!(game.current_phase, GamePhase::Main);
        assert_eq!(game.memory, memory);
        assert_eq!(
            game.players[pid as usize]
                .hand
                .iter()
                .map(|c| c.handle())
                .collect::<Vec<_>>(),
            hand
        );
        let mask = build_action_mask(game, pid);
        assert_eq!(
            mask[1002], 0.0,
            "declining must not refund Manekimon's activation"
        );
        assert_eq!(mask[PASS as usize], 1.0);
        assert!(
            mask.iter().filter(|v| **v > 0.0).count() > 1,
            "ordinary plays/digivolutions remain available"
        );
    }
    assert_eq!(to_ui_json(&runner.game), to_ui_json(&cloned));
}
