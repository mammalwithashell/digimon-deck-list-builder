//! Regressions from capped self-play games 340 and 850: spent Main activations
//! and interrupted leave continuations must survive selection and search cloning.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{encode_attack, ATTACK_START, PASS, REPLACEMENT_ACCEPT, SECURITY_TARGET},
    },
    card_data::CardData,
    debug_runner::DebugRunner,
    enums::GamePhase,
    game::Game,
    runners::HeadlessRunner,
    selection::SelectionKind,
    serialization::to_ui_json,
};

// A deliberately synthetic immediate effect reacts to the Delay COST, not to
// MarineBullmon's original leave. It preserves genuine nested-resume coverage
// without relying on the old bug that mislabeled attack-effect deletion Battle
// and then manufactured a second OwnEffect deletion for the same Digimon.
const NESTED_COST_WITNESS: &str = r#"
card: NESTED-COST-WITNESS
name: Nested Delay Cost Witness
kind: tamer
color: [blue]
cost: 3
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - kind: option
    process:
      - select_hand:
          of: you
          bind_as: witness_card
          filter: { level_lte: 4 }
          prompt: "Nested cost witness: choose the level 4 card to trash"
      - trash_from_hand_by_index: { of: you, hand_index: witness_card }
"#;

fn masked(game: &mut Game, player: u8, action: u16) {
    assert_eq!(
        build_action_mask(game, player)[action as usize],
        1.0,
        "{action}: {:?}",
        game.pending_selection
    );
    game.decode_action(action, player);
}

fn arrival_boundary(nested_cost: bool) -> Game {
    // The original recording remains immutable. Its 179-action prefix now
    // diverges at decision41 (PASS over mandatory start-main effects). Use the
    // same printed cards and a real masked attack/target choice instead.
    let raw = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../data/cards.json"
    ))
    .unwrap();
    let mut cards = CardData::load_from_str(&raw).unwrap();
    let mut builder = DebugRunner::builder();
    for id in [
        "BT26-016", "EX12-031", "EX12-070", "EX12-036", "EX12-076", "EX12-026", "BT1-019",
    ] {
        builder = builder.add_card(cards.remove(id).unwrap());
        if id != "BT1-019" {
            builder = builder.dsl_card(id).unwrap();
        }
    }
    if nested_cost {
        builder = builder.from_dsl_yaml(NESTED_COST_WITNESS).unwrap();
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
    if nested_cost {
        runner.place_on_field(1, "NESTED-COST-WITNESS", Some(0));
    }
    masked(
        &mut runner.game,
        0,
        encode_attack(attacker.index as u16, SECURITY_TARGET),
    );
    assert!(runner
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .prompt
        .contains("delete 1 of your opponent"));
    masked(&mut runner.game, 0, ATTACK_START + marine.index as u16);
    let order = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(order.previous_phase, GamePhase::Main);
    let choices = order
        .effect_choices
        .as_ref()
        .expect("Arrival and Decode are one original immediate window");
    let mut ids: Vec<_> = choices
        .iter()
        .map(|c| runner.game.card(c.source_card.unwrap()).card_id.as_str())
        .collect();
    ids.sort();
    assert_eq!(ids, ["EX12-031", "EX12-070"]);
    let choose_arrival = choices
        .iter()
        .find(|c| runner.game.card(c.source_card.unwrap()).card_id == "EX12-070")
        .unwrap()
        .action_id;
    masked(&mut runner.game, 1, choose_arrival);
    let outer = runner.game.pending_selection.as_ref().unwrap();
    assert_eq!(outer.kind, SelectionKind::Replacement);
    assert_eq!(runner.game.card(outer.source_card).card_id, "EX12-070");
    masked(&mut runner.game, 1, REPLACEMENT_ACCEPT);
    if nested_cost {
        let inner = runner
            .game
            .pending_selection
            .as_ref()
            .expect("actual nested Delay-cost reaction");
        assert_eq!(inner.kind, SelectionKind::Replacement, "{inner:?}");
        assert_eq!(
            runner.game.card(inner.source_card).card_id,
            "NESTED-COST-WITNESS"
        );
        assert_eq!(inner.previous_phase, GamePhase::Main);
    }
    runner.game
}

fn finish_arrival_branch(
    initial: &Game,
    accept_decode: bool,
    accept_cost: bool,
    play_reward: bool,
    nested_cost: bool,
) {
    let mut game = initial.clone();
    let mut cloned = game.clone();
    let mut rewards = 0;
    let mut blocks = 0;
    let mut decode_choices = 0;
    let mut cost_choices = 0;
    for _ in 0..60 {
        let Some(s) = game.pending_selection.as_ref() else {
            break;
        };
        assert_eq!(s.previous_phase, GamePhase::Main, "{s:?}");
        let pid = s.selecting_player;
        let mask = build_action_mask(&game, pid);
        assert_eq!(mask, build_action_mask(&cloned, pid));
        let source = game
            .card_data_for_handle(s.source_card)
            .map(|c| c.card_id.as_str());
        let reward = s.prompt == "Play 1 [Sanmyojin] Digimon from hand without paying the cost";
        let action = if s.kind == SelectionKind::Replacement && source == Some("EX12-031") {
            decode_choices += 1;
            if accept_decode {
                REPLACEMENT_ACCEPT
            } else {
                PASS
            }
        } else if s.kind == SelectionKind::Replacement && source == Some("NESTED-COST-WITNESS") {
            cost_choices += 1;
            if accept_cost {
                REPLACEMENT_ACCEPT
            } else {
                PASS
            }
        } else if reward {
            rewards += 1;
            if play_reward {
                s.valid_action_ids[0]
            } else {
                PASS
            }
        } else if game.current_phase == GamePhase::BlockTiming {
            blocks += 1;
            let index = game.players[1]
                .battle_area
                .iter()
                .position(|p| p.top_card().card_id(&game.card_data) == "EX12-076")
                .unwrap();
            ATTACK_START + index as u16
        } else if mask[PASS as usize] > 0.0 {
            PASS
        } else {
            s.valid_action_ids[0]
        };
        masked(&mut game, pid, action);
        masked(&mut cloned, pid, action);
        assert_eq!(to_ui_json(&game), to_ui_json(&cloned));
    }
    assert_eq!(
        rewards, 1,
        "the suspended Delay reward must still be offered exactly once"
    );
    assert_eq!(blocks, 1, "Susanoomon must actually block");
    assert_eq!(
        decode_choices, 1,
        "the original Decode candidate remains available exactly once"
    );
    assert_eq!(cost_choices, usize::from(nested_cost));
    if nested_cost {
        assert_eq!(
            game.players[1]
                .hand
                .iter()
                .filter(|c| c.card_id(&game.card_data) == "EX12-026")
                .count(),
            usize::from(!accept_cost)
        );
    }
    assert!(game.pending_selection.is_none(), "choices must finish");
    assert!(game.pending_attack.is_none(), "combat must finish");
    if game.game_over {
        assert_eq!(game.current_phase, GamePhase::GameOver);
        assert!(game.winner.is_some(), "loop draws do not count as repaired");
    } else {
        assert_eq!(game.current_phase, GamePhase::Main);
        assert!(
            build_action_mask(&game, game.turn_player())
                .iter()
                .filter(|x| **x > 0.0)
                .count()
                > 1,
            "real Main choices replace orphan PASS"
        );
    }
}

#[test]
fn capped_game_850_resumes_after_same_event_replacements_and_block() {
    let game = arrival_boundary(false);
    for accept_decode in [false, true] {
        for play_reward in [false, true] {
            finish_arrival_branch(&game, accept_decode, false, play_reward, false);
        }
    }
}

#[test]
fn nested_delay_cost_replacement_preserves_main_and_search_clone() {
    let game = arrival_boundary(true);
    for accept_cost in [false, true] {
        for play_reward in [false, true] {
            finish_arrival_branch(&game, false, accept_cost, play_reward, true);
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
