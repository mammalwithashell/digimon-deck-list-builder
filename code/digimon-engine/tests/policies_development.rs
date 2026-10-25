use digimon_engine::action::{build_action_mask, space::*};
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, GamePhase};
use digimon_engine::permanent::Permanent;
use digimon_engine::policies::{DevelopmentPolicy, Policy};

#[test]
fn successive_visible_trigger_choices_are_not_a_nonprogress_loop() {
    replay_teacher_regression(include_str!(
        "../../../docs/superpowers/research/2026-10-25-development-progress-regression.json"));
}

#[test]
fn nested_tamer_payment_choices_are_not_a_nonprogress_loop() {
    replay_teacher_regression(include_str!(
        "../../../docs/superpowers/research/2026-10-25-development-hand-progress-regression.json"));
}

#[test]
fn development_only_control_repeated_targets_finish() {
    let cases: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../docs/superpowers/research/2026-10-25-development-control-regressions.json")).unwrap();
    for case in cases { replay_teacher_regression(&case.to_string()); }
}

fn replay_teacher_regression(fixture: &str) {
    use digimon_engine::{card_data::CardData, runners::HeadlessRunner, search::decision_player};
    let case: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let cards = CardData::load_from_str(&std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../data/cards.json")).unwrap()).unwrap();
    let deck = |key: &str| case[key].as_array().unwrap().iter()
        .map(|c| c.as_str().unwrap().to_owned()).collect();
    let mut runner = HeadlessRunner::new(deck("deck1"), deck("deck2"), &cards,
        false, false, false, Some(case["seed"].as_u64().unwrap())).unwrap();
    let mut teachers = [DevelopmentPolicy::default(), DevelopmentPolicy::default()];
    for (index, row) in case["prefix"].as_array().unwrap().iter().enumerate() {
        let player = decision_player(&runner.game);
        let mask = build_action_mask(&runner.game, player);
        teachers[player as usize].decide(&runner.game, &mask)
            .unwrap_or_else(|e| panic!("prefix {index}: {e}"));
        runner.step(row["action"].as_u64().unwrap() as u16);
    }
    let player = decision_player(&runner.game);
    let mask = build_action_mask(&runner.game, player);
    let next = teachers[player as usize].decide(&runner.game, &mask).unwrap();
    assert!(mask[next.action as usize] > 0.5, "same target for a distinct visible trigger must remain usable");
    runner.step(next.action);
    for _ in 0..(1200 - case["prefix"].as_array().unwrap().len() - 1) {
        if runner.is_game_over() { break; }
        let player = decision_player(&runner.game);
        let mask = build_action_mask(&runner.game, player);
        let decision = teachers[player as usize].decide(&runner.game, &mask).unwrap();
        runner.step(decision.action);
    }
    assert!(runner.is_game_over(), "recorded teacher game must finish below the cap");
}

fn position(cost: u16, memory: i16) -> DebugRunner {
    let mut rookie = make_test_card("DEV-3", "Rookie");
    rookie.level = Some(3);
    rookie.colors = vec![CardColor::Red];
    let mut champion = make_test_card("DEV-4", "Champion");
    champion.level = Some(4);
    champion.play_cost = 7;
    champion.colors = vec![CardColor::Red];
    champion.evo_costs = vec![EvoCost {
        card_color: 0,
        level: 3,
        memory_cost: cost,
    }];
    let mut r = DebugRunner::builder()
        .add_card(rookie)
        .add_card(champion)
        .hand(0, &["DEV-4", "DEV-3"])
        .hand(1, &["DEV-3", "DEV-4"])
        .deck(0, &["DEV-3", "DEV-4", "DEV-3", "DEV-4"])
        .deck(1, &["DEV-4", "DEV-3", "DEV-4", "DEV-3"])
        .security(1, &["DEV-3", "DEV-4"])
        .memory(memory)
        .start();
    r.skip_mulligan();
    let card = r.game.players[0].hand.pop().unwrap();
    r.game.players[0]
        .battle_area
        .push(Permanent::new(card, r.game.turn_count));
    r.game.current_phase = GamePhase::Main;
    r
}

#[test]
fn useful_evolution_beats_hard_play_and_counts_replacement() {
    let r = position(2, 1);
    let mask = build_action_mask(&r.game, 0);
    let mut policy = DevelopmentPolicy::default();
    let decision = policy.decide(&r.game, &mask).unwrap();
    assert_eq!(decision.action, encode_digivolve(0, 0));
    let features = decision
        .candidates
        .iter()
        .find(|c| c.action == decision.action)
        .unwrap();
    assert_eq!(features.net_hand_gain, Some(0));
    assert!(features.replacement);
    assert_eq!(features.base_memory_cost, Some(2));
}

#[test]
fn excessive_memory_handoff_does_not_force_evolution() {
    let r = position(9, 0);
    let mask = build_action_mask(&r.game, 0);
    assert_ne!(
        DevelopmentPolicy::default().select(&r.game, &mask),
        encode_digivolve(0, 0)
    );
}

#[test]
fn hidden_hand_and_deck_order_do_not_change_features() {
    let mut r = position(2, 1);
    let mask = build_action_mask(&r.game, 0);
    let before = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    r.game.players[1].hand.reverse();
    r.game.players[1].deck.reverse();
    r.game.players[1].security.reverse();
    r.game.players[0].deck.reverse();
    let after = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    assert_eq!(before, after);
}

#[test]
fn simple_route_cost_matches_actual_payment_and_scoring_does_not_mutate_game() {
    let mut r = position(2, 5);
    let mask = build_action_mask(&r.game, 0);
    let before = r.game.memory;
    let decision = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    assert_eq!(r.game.memory, before);
    let candidate = decision
        .candidates
        .iter()
        .find(|c| c.action == encode_digivolve(0, 0))
        .unwrap();
    r.game.decode_action(candidate.action, 0);
    assert_eq!(
        before - r.game.memory,
        candidate.base_memory_cost.unwrap() as i16
    );
}

#[test]
fn attack_into_empty_unprotected_board_precedes_development() {
    let mut r = position(0, 5);
    r.game.players[1].security.clear();
    r.game.turn_count = 3;
    r.game.players[0].battle_area[0].turn_played = 1;
    let mask = build_action_mask(&r.game, 0);
    let attack = encode_attack(0, SECURITY_TARGET);
    assert!(mask[attack as usize] > 0.0);
    assert_eq!(DevelopmentPolicy::default().select(&r.game, &mask), attack);
}

#[test]
fn pure_draw_metadata_never_counts_conditional_or_filtering_bodies_as_free_gain() {
    use digimon_dsl::compiled::{CompiledBindingRef, CompiledPlayerRef, CompiledStep};
    use digimon_engine::policies::development::HandBenefit;
    let draw = CompiledStep::Draw {
        of: CompiledPlayerRef::You,
        count: 2,
    };
    assert_eq!(
        HandBenefit::from_steps(&[draw.clone()]).literal_draw,
        Some(2)
    );
    assert_eq!(
        HandBenefit::from_steps(&[CompiledStep::Optional(vec![draw.clone()])]).literal_draw,
        None
    );
    assert_eq!(
        HandBenefit::from_steps(&[
            draw,
            CompiledStep::TrashFromHandByIndex {
                of: CompiledPlayerRef::You,
                hand_index: CompiledBindingRef::Named("payment".into())
            }
        ])
        .literal_draw,
        None
    );
}

#[test]
fn inactive_draw_text_does_not_create_an_immediate_hand_bonus() {
    let mut r = position(2, 5);
    let card = r.game.players[0].hand[0].data_index;
    r.game.card_data[card].effect_text = "[When Attacking] Draw 3.".into();
    let mask = build_action_mask(&r.game, 0);
    let decision = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    assert_eq!(
        decision
            .candidates
            .iter()
            .find(|c| c.action == 0)
            .unwrap()
            .net_hand_gain,
        Some(-1)
    );
}

#[test]
fn clone_has_independent_progress_history() {
    let r = position(2, 5);
    let mask = build_action_mask(&r.game, 0);
    let policy = DevelopmentPolicy::default();
    let mut a = policy.clone();
    let mut b = policy.clone();
    assert_eq!(
        a.decide(&r.game, &mask).unwrap(),
        b.decide(&r.game, &mask).unwrap()
    );
    a.decide(&r.game, &mask).unwrap();
    assert_ne!(
        a.decide(&r.game, &mask).unwrap().action,
        b.decide(&r.game, &mask).unwrap().action
    );
}

#[test]
fn paged_duplicate_sources_rebind_the_selected_physical_card() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = position(2, 5);
    let mut stack = vec!["DEV-3"; 40];
    stack.push("DEV-4");
    let host = r.place_stack(0, &stack);
    let top = r.game.players[0].battle_area[host.index as usize]
        .top_card()
        .handle();
    let expected = r.game.players[0].battle_area[host.index as usize].card_sources[0].handle();
    EffectContext::new(&mut r.game, top, Some(host), 0).select_own_sources(
        "Paged duplicate sources",
        1,
        1,
        move |_, reference| reference.permanent == host,
        move |ctx, chosen| {
            assert_eq!(chosen[0].card, expected);
            ctx.game.memory = 9;
        },
    );
    let mut teacher = DevelopmentPolicy::default();
    let mut actions = Vec::new();
    for _ in 0..4 {
        if r.game.pending_selection.is_none() {
            break;
        }
        let mask = build_action_mask(&r.game, 0);
        let action = teacher.decide(&r.game, &mask).unwrap().action;
        assert!(mask[action as usize] > 0.5);
        actions.push(action);
        r.execute_action(0, action).unwrap();
    }
    assert_eq!(actions, vec![59, 58, 30]);
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.memory, 9);
}

#[test]
fn optional_draw_is_accepted_for_the_actual_deciding_player() {
    let yaml = r#"
card: DEV-DRAW
name: Optional Draw
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    optional: true
    process:
      - draw: {of: you, count: 2}
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .hand(1, &["DEV-DRAW"])
        .deck(1, &["DEV-DRAW"; 5])
        .start();
    r.skip_mulligan();
    r.play(1, 0);
    let player = digimon_engine::policies::current_decision_player(&r.game);
    assert_eq!(player, 1);
    let mask = build_action_mask(&r.game, player);
    let decision = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    assert_eq!(decision.player, 1);
    assert_ne!(decision.action, PASS);
    assert!(mask[decision.action as usize] > 0.0);
}

#[test]
fn dna_accounts_for_lost_body_and_still_chooses_an_efficient_route() {
    use digimon_engine::card_data::{DnaCost, DnaRequirement};
    let mut r = position(2, 5);
    r.place_stack(0, &["DEV-3"]);
    let index = r.game.players[0].hand[0].data_index;
    let requirement = DnaRequirement {
        level: 3,
        card_colors: vec![CardColor::Red],
        name_contains: String::new(),
        text_contains: String::new(),
    };
    r.game.card_data[index].dna_costs = vec![DnaCost {
        requirement1: requirement.clone(),
        requirement2: requirement,
        memory_cost: 0,
    }];
    let mask = build_action_mask(&r.game, 0);
    assert!(mask[DNA_DIGIVOLVE_START as usize] > 0.5);
    let d = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    let dna = d
        .candidates
        .iter()
        .find(|c| c.action == DNA_DIGIVOLVE_START)
        .unwrap();
    let regular = d
        .candidates
        .iter()
        .find(|c| c.action == encode_digivolve(0, 0))
        .unwrap();
    assert_eq!(dna.bodies_consumed, 1);
    assert!(
        regular.score > dna.score,
        "preserve the second body when normal evolution is efficient"
    );
    r.game.card_data[index].evo_costs[0].memory_cost = 9;
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(
        DevelopmentPolicy::default().select(&r.game, &mask),
        DNA_DIGIVOLVE_START
    );
}

#[test]
fn discard_keeps_the_only_visible_evolution_bridge() {
    let yaml = r#"
card: DEV-PAYMENT
name: Payment
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          filter: {}
          bind_as: payment
          prompt: Choose payment
      - trash_from_hand_by_index: {of: you, hand_index: payment}
      - draw: {of: you, count: 2}
"#;
    // Load the typed continuation through the normal DSL runner.
    let base = position(2, 5);
    let cards = base.game.card_data.clone();
    let mut builder = DebugRunner::builder().from_dsl_yaml(yaml).unwrap();
    for card in cards.iter().cloned() {
        builder = builder.add_card(card);
    }
    let mut r = builder
        .hand(0, &["DEV-PAYMENT", "DEV-4", "DEV-3"])
        .deck(0, &["DEV-3"; 10])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.place_stack(0, &["DEV-3"]);
    r.play(0, 0);
    assert!(r.game.pending_selection.is_some());
    let mask = build_action_mask(&r.game, 0);
    let decision = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    assert_eq!(
        decision.action, 1,
        "discard the spare rookie, preserve the champion bridge"
    );
    assert_eq!(
        decision
            .candidates
            .iter()
            .find(|c| c.action == 1)
            .unwrap()
            .route,
        "hand_payment"
    );
}

#[test]
fn optional_retrieval_selects_a_visible_evolution_bridge() {
    let yaml = r#"
card: DEV-RETRIEVE
name: Retrieval
kind: tamer
color: [red]
cost: 0
effects:
  - when: on_play
    process:
      - select_trash:
          of: you
          filter: {}
          bind_as: pick
          optional: true
          prompt: Retrieve one card
      - add_to_hand_from_trash: {of: you, card: pick}
"#;
    let cards = position(2, 5).game.card_data.clone();
    let mut builder = DebugRunner::builder().from_dsl_yaml(yaml).unwrap();
    for card in cards.iter().cloned() { builder = builder.add_card(card); }
    let mut r = builder.hand(0, &["DEV-RETRIEVE", "DEV-3", "DEV-4"])
        .deck(0, &["DEV-3"; 10]).memory(5).start();
    r.skip_mulligan();
    r.place_stack(0, &["DEV-3"]);
    for _ in 0..2 {
        let card = r.game.players[0].hand.remove(1);
        r.game.players[0].trash.push(card);
    }
    r.play(0, 0);
    let mask = build_action_mask(&r.game, 0);
    let decision = DevelopmentPolicy::default().decide(&r.game, &mask).unwrap();
    assert_eq!(decision.action, TRASH_EFFECT_START + 1);
    r.execute_action(0, decision.action).unwrap();
    assert!(r.game.players[0].hand.iter().any(|c| c.card_id(&r.game.card_data) == "DEV-4"));
}

#[test]
fn stale_security_menu_uses_legal_refresh_then_rebinds() {
    use digimon_engine::effect_context::EffectContext;
    let mut r = position(2, 5);
    let host = r.place_stack(0, &["DEV-3"]);
    let top = r.game.players[0].battle_area[host.index as usize].top_card().handle();
    // Install a real menu, then simulate a security-order invalidation.
    let cards: Vec<_> = r.game.players[0].deck.drain(..2).collect();
    r.game.players[0].security.extend(cards);
    EffectContext::new(&mut r.game, top, Some(host), 0).select_security(0,
        "Refresh blind security", false, |_, _| true, |_, _| {});
    r.game.players[0].security.swap(0, 1);
    let mut teacher = DevelopmentPolicy::default();
    let mask = build_action_mask(&r.game, 0);
    let refresh = teacher.decide(&r.game, &mask).unwrap().action;
    assert_eq!(refresh, 59);
    r.execute_action(0, refresh).unwrap();
    let mask = build_action_mask(&r.game, 0);
    let commit = teacher.decide(&r.game, &mask).unwrap().action;
    assert_eq!(commit, 30);
    r.execute_action(0, commit).unwrap();
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn empty_or_concede_only_masks_fail_instead_of_fabricating_a_choice() {
    let r = position(2, 1);
    let mut mask = vec![0.0; ACTION_SPACE_SIZE];
    assert!(DevelopmentPolicy::default().decide(&r.game, &mask).is_err());
    mask[CONCEDE_GAME as usize] = 1.0;
    assert!(DevelopmentPolicy::default().decide(&r.game, &mask).is_err());
}

#[test]
fn mandatory_nonprogress_menu_fails_after_exhausting_legal_choices() {
    let r = position(2, 1);
    let mut mask = vec![0.0; ACTION_SPACE_SIZE];
    mask[encode_digivolve(0, 0) as usize] = 1.0;
    let mut policy = DevelopmentPolicy::default();
    policy.decide(&r.game, &mask).unwrap();
    assert!(policy.decide(&r.game, &mask).unwrap_err().contains("exhausted legal progress"));
}

#[test]
fn repeated_semantic_state_uses_progress_alternative_and_reset_clears_history() {
    let r = position(2, 1);
    let mask = build_action_mask(&r.game, 0);
    let mut policy = DevelopmentPolicy::default();
    let first = policy.decide(&r.game, &mask).unwrap().action;
    let second = policy.decide(&r.game, &mask).unwrap().action;
    assert_ne!(first, second);
    policy.reset();
    assert_eq!(first, policy.decide(&r.game, &mask).unwrap().action);
}
