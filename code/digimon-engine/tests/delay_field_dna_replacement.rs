//! Paid Delay with a result-dependent field pair. Synthetic reusable primitive tests.
use digimon_engine::card_data::{CardData, DnaCost, DnaRequirement};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;
use digimon_engine::replacement::ReplacementCause;

const YAML: &str = r#"
card: DELAY-DNA
name: Paid field DNA
kind: option
color: [red]
cost: 2
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - kind: digimon
        - none_of: [{replacement_cause: battle}]
    process:
      - delete_permanent: {target: source}
      - select_hand:
          of: you
          bind_as: result
          optional: true
          filter: {kind: digimon, name_is: Examon}
          prompt: Result
      - select_dna_pair:
          left_filter: {kind: digimon, of: you}
          right_filter: {kind: digimon, of: you}
          bind_left_as: first
          bind_right_as: second
          optional: true
          prompt: Material
      - effect_initiated_dna_digivolve:
          target_a: first
          target_b: second
          from_hand: result
          cost: printed
          ignore_requirements: false
"#;

fn material(id: &str, color: CardColor) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.colors = vec![color];
    c
}

fn setup(same_turn: bool) -> DebugRunner {
    setup_yaml(same_turn, YAML)
}

fn setup_yaml(same_turn: bool, yaml: &str) -> DebugRunner {
    let mut result = material("RESULT", CardColor::Blue);
    result.card_name = "Examon".into();
    result.level = Some(7);
    let req = |color| DnaRequirement {
        level: 6,
        card_colors: vec![color],
        name_contains: String::new(),
        text_contains: String::new(),
    };
    result.dna_costs = vec![DnaCost {
        requirement1: req(CardColor::Blue),
        requirement2: req(CardColor::Green),
        memory_cost: 2,
    }];
    let mut wrong = result.clone();
    wrong.card_id = "WRONG".into();
    wrong.card_name = "Examon X Antibody".into();
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(result)
        .add_card(wrong)
        .add_card(material("BLUE", CardColor::Blue))
        .add_card(material("GREEN", CardColor::Green))
        .add_card(material("RED", CardColor::Red))
        .add_card(material("FILL", CardColor::Red))
        .memory(10)
        .hand(0, &["RESULT", "WRONG"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    runner.game.turn_count = 3;
    let option = runner.place_on_field(0, "DELAY-DNA", None);
    runner.game.players[0].battle_area[option.index as usize].option_state = OptionState::Delayed {
        owner: 0,
        placed_on_turn: if same_turn { 3 } else { 2 },
        trash_on_turn: 100,
        trigger: DelayTrigger::EndOfYourNextTurn,
    };
    runner.place_on_field(0, "BLUE", None);
    runner.place_on_field(0, "GREEN", None);
    runner.place_on_field(0, "RED", None);
    runner
}

fn trigger(runner: &mut DebugRunner, index: u8) {
    runner.game.delete_permanent_with_cause(
        digimon_engine::permanent::PermanentHandle { player: 0, index },
        ReplacementCause::OpponentEffect,
    );
}
fn pick(game: &mut digimon_engine::game::Game, label: Option<&str>) {
    let sel = game.pending_selection.as_ref().expect("selection");
    let action = label
        .map(|label| {
            sel.effect_choices
                .as_ref()
                .unwrap()
                .iter()
                .find(|c| c.label == label)
                .expect(label)
                .action_id
        })
        .unwrap_or(sel.valid_action_ids[0]);
    let player = sel.selecting_player;
    game.resolve_selection(player, action).unwrap();
}
fn top_ids(game: &digimon_engine::game::Game) -> Vec<String> {
    game.players[0]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&game.card_data).to_string())
        .collect()
}

#[test]
fn delay_field_dna_same_turn_not_offered() {
    let mut runner = setup(true);
    trigger(&mut runner, 1);
    assert!(
        runner.game.pending_selection.is_none(),
        "Delay must be unavailable on placing turn"
    );
    assert!(top_ids(&runner.game).contains(&"DELAY-DNA".into()));
}

#[test]
fn delay_field_dna_paid_legal_pair_cost_and_clone_each_prompt() {
    for threatened in [1, 2, 3] {
        let mut runner = setup(false);
        trigger(&mut runner, threatened);
        for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
            let mut cloned = runner.game.clone();
            pick(&mut cloned, choice);
            pick(&mut runner.game, choice);
            assert_eq!(top_ids(&cloned), top_ids(&runner.game));
            assert_eq!(cloned.memory, runner.game.memory);
            assert_eq!(
                cloned.players[0]
                    .trash
                    .iter()
                    .map(|c| c.handle())
                    .collect::<Vec<_>>(),
                runner.game.players[0]
                    .trash
                    .iter()
                    .map(|c| c.handle())
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                cloned
                    .pending_selection
                    .as_ref()
                    .map(|s| &s.valid_action_ids),
                runner
                    .game
                    .pending_selection
                    .as_ref()
                    .map(|s| &s.valid_action_ids)
            );
            if let Some(selection) = runner.game.pending_selection.as_ref() {
                let labels: Vec<_> = selection
                    .effect_choices
                    .as_ref()
                    .unwrap()
                    .iter()
                    .map(|c| c.label.as_str())
                    .collect();
                assert!(!labels.contains(&"Examon X Antibody"));
                assert!(!labels.contains(&"RED"));
            }
        }
        assert!(runner.game.pending_selection.is_none());
        assert!(
            top_ids(&runner.game).contains(&"RESULT".into()),
            "merged Digimon must survive"
        );
        assert_eq!(runner.game.memory, 8, "printed DNA cost must be paid");
        let trash: Vec<_> = runner.game.players[0]
            .trash
            .iter()
            .map(|c| c.card_id(&runner.game.card_data))
            .collect();
        assert!(trash.contains(&"DELAY-DNA"));
        assert_eq!(
            trash.contains(&"RED"),
            threatened == 3,
            "excluded threatened subject still leaves after field compaction"
        );
        assert!(!trash.contains(&"BLUE") && !trash.contains(&"GREEN"));
    }
}

#[test]
fn delay_field_dna_decline_after_payment_preserves_original_subject() {
    let mut runner = setup(false);
    trigger(&mut runner, 3);
    pick(&mut runner.game, None);
    runner
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert_eq!(top_ids(&runner.game), vec!["BLUE", "GREEN"]);
    let trash: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(trash.contains(&"DELAY-DNA") && trash.contains(&"RED"));
}

#[test]
fn delay_field_dna_material_selection_order_does_not_change_recipe_stack_order() {
    for order in [["BLUE", "GREEN"], ["GREEN", "BLUE"]] {
        let mut runner = setup(false);
        trigger(&mut runner, 1);
        for choice in [None, Some("Examon"), Some(order[0]), Some(order[1])] {
            pick(&mut runner.game, choice);
        }
        let merged = runner.game.players[0]
            .battle_area
            .iter()
            .find(|p| p.top_card().card_id(&runner.game.card_data) == "RESULT")
            .expect("DNA result survives the replaced departure");
        let stack: Vec<_> = merged
            .card_sources
            .iter()
            .map(|card| card.card_id(&runner.game.card_data))
            .collect();
        // Synthetic recipe is blue (left) + green (right). Rule8-2-2-2
        // puts left above right; card_sources stores bottom-to-top.
        assert_eq!(
            stack,
            ["GREEN", "BLUE", "RESULT"],
            "printed left material must be above right regardless of pick order {order:?}"
        );
    }
}

#[test]
fn delay_field_dna_opponent_turn_pays_for_controller() {
    let mut runner = setup(false);
    runner.game.turn_player_idx = 1;
    runner.game.memory = 3;
    trigger(&mut runner, 1);
    for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
        pick(&mut runner.game, choice);
    }
    assert_eq!(runner.game.memory, 5, "P0 pays toward P1 on P1's turn");
    assert!(top_ids(&runner.game).contains(&"RESULT".into()));
}

#[test]
fn delay_field_dna_excluded_subject_keeps_hand_or_deck_destination() {
    for to_deck in [false, true] {
        let mut runner = setup(false);
        let threatened = digimon_engine::permanent::PermanentHandle {
            player: 0,
            index: 3,
        };
        if to_deck {
            runner
                .game
                .return_to_deck(threatened, digimon_engine::enums::StackPosition::Bottom);
        } else {
            runner.game.return_to_hand(threatened);
        }
        for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
            pick(&mut runner.game, choice);
        }
        assert_eq!(top_ids(&runner.game), vec!["RESULT"]);
        let player = &runner.game.players[0];
        let zone = if to_deck { &player.deck } else { &player.hand };
        assert!(zone
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "RED"));
        assert!(!player
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "RED"));
    }
}

#[test]
fn delay_field_dna_nested_payment_can_be_prevented_or_completed_on_clone() {
    let yaml = format!("{YAML}\n  - kind: replacement\n    trigger: when_would_be_deleted\n    optional: true\n    active_when: {{replacement_cause: cost}}\n    process:\n      - cancel_replacement: {{}}\n");
    let mut runner = setup_yaml(false, &yaml);
    trigger(&mut runner, 3);
    pick(&mut runner.game, None);
    assert!(
        runner.game.pending_selection.is_some(),
        "cost prevention prompt"
    );
    let mut prevented = runner.game.clone();
    pick(&mut prevented, None);
    assert!(
        prevented.pending_selection.is_none(),
        "unpaid Delay cannot offer DNA"
    );
    assert_eq!(top_ids(&prevented), vec!["DELAY-DNA", "BLUE", "GREEN"]);
    runner
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS)
        .unwrap();
    for choice in [Some("Examon"), Some("BLUE"), Some("GREEN")] {
        pick(&mut runner.game, choice);
    }
    assert_eq!(top_ids(&runner.game), vec!["RESULT"]);
}

#[test]
fn delay_field_dna_committed_merge_can_delete_itself_without_removing_bystander() {
    struct DeleteSelf;
    impl digimon_engine::effect::CardEffect for DeleteSelf {
        fn effects(
            &self,
            card: digimon_engine::card_source::CardHandle,
        ) -> Vec<digimon_engine::effect::Effect> {
            vec![digimon_engine::effect::Effect::when_digivolving(card)
                .process(|ctx| {
                    if let Some(source) = ctx.source_permanent {
                        ctx.delete_permanent(source);
                    }
                })
                .build()]
        }
    }
    let mut runner = setup(false);
    runner.register_effect("RESULT", std::sync::Arc::new(DeleteSelf));
    trigger(&mut runner, 1);
    for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
        pick(&mut runner.game, choice);
    }
    assert_eq!(top_ids(&runner.game), vec!["RED"]);
    assert_eq!(runner.game.memory, 8);
    let trash: Vec<_> = runner.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&runner.game.card_data))
        .collect();
    assert!(trash.contains(&"RESULT") && trash.contains(&"BLUE") && trash.contains(&"GREEN"));
}

#[test]
fn delay_field_dna_decline_before_payment_and_pay_without_reward() {
    let mut declined = setup(false);
    trigger(&mut declined, 3);
    declined
        .game
        .resolve_selection(0, digimon_engine::action::space::PASS)
        .unwrap();
    assert_eq!(top_ids(&declined.game), vec!["DELAY-DNA", "BLUE", "GREEN"]);
    let mut empty = setup(false);
    empty.game.players[0].hand.clear();
    trigger(&mut empty, 3);
    pick(&mut empty.game, None);
    assert!(empty.game.pending_selection.is_none());
    assert_eq!(top_ids(&empty.game), vec!["BLUE", "GREEN"]);
    let trash: Vec<_> = empty.game.players[0]
        .trash
        .iter()
        .map(|c| c.card_id(&empty.game.card_data))
        .collect();
    assert!(trash.contains(&"DELAY-DNA") && trash.contains(&"RED"));
}

fn assert_delay_observer_sequence(stable_slot: bool, remove_observer: bool, replay: bool) {
    struct RecoverOption;
    impl digimon_engine::effect::CardEffect for RecoverOption {
        fn effects(
            &self,
            card: digimon_engine::card_source::CardHandle,
        ) -> Vec<digimon_engine::effect::Effect> {
            vec![digimon_engine::effect::EffectBuilder::new(
                card,
                digimon_engine::enums::EffectTiming::OnLeaveField,
            )
            .process(|ctx| {
                let Some(paid) = ctx.event_card() else {
                    return;
                };
                if ctx.game.players[0]
                    .trash
                    .iter()
                    .any(|c| c.handle() == paid && c.card_id(&ctx.game.card_data) == "DELAY-DNA")
                {
                    assert!(ctx.add_to_hand_from_trash(0, paid));
                }
            })
            .build()]
        }
    }
    struct MoveObserver(bool);
    impl digimon_engine::effect::CardEffect for MoveObserver {
        fn effects(
            &self,
            card: digimon_engine::card_source::CardHandle,
        ) -> Vec<digimon_engine::effect::Effect> {
            let replay = self.0;
            vec![digimon_engine::effect::Effect::when_digivolving(card)
                .process(move |ctx| {
                    let index = ctx.game.players[0]
                        .battle_area
                        .iter()
                        .position(|p| p.top_card().card_id(&ctx.game.card_data) == "RED")
                        .unwrap();
                    let source = ctx.game.players[0].battle_area[index].top_card().handle();
                    ctx.game
                        .return_to_hand(digimon_engine::permanent::PermanentHandle {
                            player: 0,
                            index: index as u8,
                        });
                    if replay {
                        assert!(ctx
                            .game
                            .play_card_from_effect_without_cost(0, source)
                            .is_some());
                    }
                })
                .build()]
        }
    }
    for nested_prevention in [false, true] {
        let prevention = if nested_prevention {
            "\n  - kind: replacement\n    trigger: when_would_be_deleted\n    optional: true\n    active_when: {replacement_cause: cost}\n    process:\n      - cancel_replacement: {}\n"
        } else {
            ""
        };
        let yaml = format!("{YAML}{prevention}");
        let mut runner = setup_yaml(false, &yaml);
        runner.register_effect("RED", std::sync::Arc::new(RecoverOption));
        if remove_observer {
            runner.register_effect("RESULT", std::sync::Arc::new(MoveObserver(replay)));
        }
        // Keep the observer in slot zero throughout cost removal and DNA.
        if stable_slot {
            runner.game.players[0].battle_area.swap(0, 3);
        }
        trigger(&mut runner, 1);
        let mut cloned = runner.game.clone();
        for game in [&mut runner.game, &mut cloned] {
            pick(game, None);
            if nested_prevention {
                game.resolve_selection(0, digimon_engine::action::space::PASS)
                    .unwrap();
            }
            // Rules 15-8-3-2: triggers caused by the cost must wait until
            // the currently processing Delay reward has finished.
            assert!(!game.players[0]
                .hand
                .iter()
                .any(|card| card.card_id(&game.card_data) == "DELAY-DNA"));
            assert!(game.players[0]
                .trash
                .iter()
                .any(|card| card.card_id(&game.card_data) == "DELAY-DNA"));
            assert!(
                game.pending_selection.is_some(),
                "paid Delay offers its result before derived triggers activate"
            );
            for choice in [Some("Examon"), Some("BLUE"), Some("GREEN")] {
                pick(game, choice);
            }
            if remove_observer {
                if let Some(sel) = game.pending_selection.as_ref() {
                    let action = sel
                        .effect_choices
                        .as_ref()
                        .unwrap()
                        .iter()
                        .find(|c| c.label.starts_with("RESULT slot"))
                        .expect("choose result's evolution effect before pending recovery")
                        .action_id;
                    game.resolve_selection(0, action).unwrap();
                }
            }
            let expected = if remove_observer && !replay {
                vec!["RESULT"]
            } else if stable_slot && !replay {
                vec!["RED", "RESULT"]
            } else {
                vec!["RESULT", "RED"]
            };
            assert_eq!(top_ids(game), expected);
            assert_eq!(game.memory, 8);
            assert!(game.pending_selection.is_none());
            assert_eq!(game.players[0].hand.iter().any(|card| card.card_id(&game.card_data) == "DELAY-DNA"), !remove_observer,
                "only the surviving original observer recovers the Option after reward finishes (remove={remove_observer}, replay={replay}, nested={nested_prevention})");
        }
    }
}

#[test]
fn delay_field_dna_cost_observers_wait_until_reward_finishes_on_clone() {
    assert_delay_observer_sequence(true, false, false);
}

#[test]
fn delay_field_dna_shifted_observer_retains_pending_trigger_on_clone() {
    assert_delay_observer_sequence(false, false, false);
}

#[test]
fn delay_field_dna_removed_or_replayed_observer_loses_pending_trigger_on_clone() {
    assert_delay_observer_sequence(false, true, false);
    assert_delay_observer_sequence(false, true, true);
}

#[test]
fn delay_field_dna_retained_material_inherited_trigger_loses_timing_on_clone() {
    struct InheritedRecovery;
    impl digimon_engine::effect::CardEffect for InheritedRecovery {
        fn effects(
            &self,
            card: digimon_engine::card_source::CardHandle,
        ) -> Vec<digimon_engine::effect::Effect> {
            vec![digimon_engine::effect::EffectBuilder::new(
                card,
                digimon_engine::enums::EffectTiming::OnLeaveField,
            )
            .inherited()
            .process(|ctx| {
                let Some(paid) = ctx.event_card() else {
                    return;
                };
                if ctx.game.players[0]
                    .trash
                    .iter()
                    .any(|c| c.handle() == paid && c.card_id(&ctx.game.card_data) == "DELAY-DNA")
                {
                    assert!(ctx.add_to_hand_from_trash(0, paid));
                }
            })
            .build()]
        }
    }
    let mut runner = setup(false);
    runner.register_effect("RED", std::sync::Arc::new(InheritedRecovery));
    // Stage the observer as BLUE's inherited source. BLUE is the retained
    // first material slot, but DNA creates a new Digimon (8-2-3, 15-4-4-3).
    let source = runner.game.players[0]
        .battle_area
        .remove(3)
        .card_sources
        .pop()
        .unwrap();
    runner.game.players[0].battle_area[1]
        .card_sources
        .insert(0, source);
    trigger(&mut runner, 1);
    let mut cloned = runner.game.clone();
    for game in [&mut runner.game, &mut cloned] {
        for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
            pick(game, choice);
        }
        assert_eq!(top_ids(game), vec!["RESULT"]);
        assert!(game.players[0].battle_area[0]
            .card_sources
            .iter()
            .any(|c| c.card_id(&game.card_data) == "RED"));
        assert!(
            !game.players[0]
                .hand
                .iter()
                .any(|c| c.card_id(&game.card_data) == "DELAY-DNA"),
            "pending effect belonged to the old material Digimon, not the new DNA Digimon"
        );
        assert!(game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&game.card_data) == "DELAY-DNA"));
        assert!(game.pending_selection.is_none());
    }
}

#[test]
fn delay_field_dna_preserves_result_dna_origin_on_clone() {
    struct DnaReward;
    impl digimon_engine::effect::CardEffect for DnaReward {
        fn effects(
            &self,
            card: digimon_engine::card_source::CardHandle,
        ) -> Vec<digimon_engine::effect::Effect> {
            vec![digimon_engine::effect::Effect::when_digivolving(card)
                .condition(|ctx| ctx.event_dna_origin() == Some(true))
                .process(|ctx| {
                    ctx.game.memory += 1;
                })
                .build()]
        }
    }
    let mut runner = setup(false);
    runner.register_effect("RESULT", std::sync::Arc::new(DnaReward));
    trigger(&mut runner, 1);
    let mut cloned = runner.game.clone();
    for game in [&mut runner.game, &mut cloned] {
        for choice in [None, Some("Examon"), Some("BLUE"), Some("GREEN")] {
            pick(game, choice);
        }
        assert_eq!(top_ids(game), vec!["RESULT", "RED"]);
        assert!(game.pending_selection.is_none());
        assert_eq!(
            game.memory, 9,
            "DNA-gated result reward must activate after the replacement finishes"
        );
    }
}

#[test]
fn delay_field_dna_examon_sweep_on_opponent_turn_on_clone() {
    let mut low = material("LOW", CardColor::Red);
    low.dp = Some(1000);
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/st1/ST1-16.yaml"))
        .unwrap()
        .from_dsl_yaml(include_str!("../cards/bt20/BT20-045.yaml"))
        .unwrap()
        .add_card(material("BLUE", CardColor::Blue))
        .add_card(material("GREEN", CardColor::Green))
        .add_card(material("FILL", CardColor::Red))
        .add_card(low)
        .memory(10)
        .hand(0, &["BT20-045", "BT20-045"])
        .hand(1, &["ST1-16"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();
    runner.game.turn_count = 3;
    runner.game.turn_player_idx = 1;
    let option = runner.place_on_field(0, "DELAY-DNA", None);
    runner.game.players[0].battle_area[option.index as usize].option_state = OptionState::Delayed {
        owner: 0,
        placed_on_turn: 2,
        trash_on_turn: 100,
        trigger: DelayTrigger::EndOfYourNextTurn,
    };
    runner.place_on_field(0, "BLUE", None);
    runner.place_on_field(0, "GREEN", None);
    let victim = runner.place_on_field(1, "FILL", None);
    let victim_card = runner.game.players[1].battle_area[victim.index as usize]
        .top_card()
        .handle();
    runner.place_on_field(1, "LOW", None);
    let action = digimon_engine::action::space::PLAY_HAND_START;
    assert!(
        digimon_engine::action::mask::build_action_mask(&runner.game, 1)[action as usize] > 0.0
    );
    runner.game.decode_action(action, 1);
    let select = digimon_engine::action::encode_attack(0, 1);
    runner
        .game
        .resolve_selection(1, select)
        .expect("Gaia targets BLUE");
    let mut cloned = runner.game.clone();
    for game in [&mut runner.game, &mut cloned] {
        for choice in [None, Some("Examon"), Some("GREEN"), Some("BLUE")] {
            pick(game, choice);
        }
        assert_eq!(top_ids(game), vec!["BT20-045"]);
        assert!(game.pending_selection.is_none());
        assert!(
            !game.players[1]
                .battle_area
                .iter()
                .any(|p| p.top_card().card_id(&game.card_data) == "FILL"),
            "DNA Examon must bottom-deck opposing highest-DP Digimon; field {:?}",
            game.players[1]
                .battle_area
                .iter()
                .map(|p| p.top_card().card_id(&game.card_data))
                .collect::<Vec<_>>()
        );
        assert_eq!(game.players[1].deck.first().unwrap().handle(), victim_card);
        assert_eq!(game.players[1].battle_area.len(), 1);
        // A later ordinary evolution must not inherit the inner DNA event's flag.
        let result = game.players[0]
            .hand
            .iter()
            .position(|c| c.card_id(&game.card_data) == "BT20-045")
            .unwrap();
        let card = game.players[0].hand[result].handle();
        let mut ctx = digimon_engine::effect_context::EffectContext::new(game, card, None, 0);
        assert!(ctx.effect_initiated_digivolve_ignore_requirements(
            0,
            result,
            digimon_engine::permanent::PermanentHandle {
                player: 0,
                index: 0
            },
            digimon_engine::enums::CostDelta::Free
        ));
        assert_eq!(
            game.players[1].battle_area.len(),
            1,
            "ordinary evolution must not sweep the surviving low-DP Digimon"
        );
    }
}
