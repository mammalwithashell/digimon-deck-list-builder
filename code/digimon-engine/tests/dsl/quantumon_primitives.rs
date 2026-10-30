//! Independent behavioral lowering evidence for Quantumon's reusable primitives.
use digimon_engine::action::{build_action_mask, space::{PASS, PLAY_HAND_START}};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;

#[test]
fn return_selected_security_to_deck_moves_each_exact_legal_choice_to_the_top() {
    let yaml = r#"
card: SECURITY-RETURNER
name: Security Returner
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: on_play
    process:
      - reveal_security: { of: opponent }
      - select_security:
          of: opponent
          bind_as: chosen
          filter: {}
          prompt: Choose one publicly revealed security card
      - return_selected_security_to_deck: { of: opponent, card: chosen, position: top }
"#;
    for selected in 0..3 {
        let mut r = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
            .add_card(make_test_card("SEC-A", "Security A"))
            .add_card(make_test_card("SEC-B", "Security B"))
            .add_card(make_test_card("SEC-C", "Security C"))
            .add_card(make_test_card("DECK", "Existing deck card"))
            .hand(0, &["SECURITY-RETURNER"])
            .security(1, &["SEC-A", "SEC-B", "SEC-C"])
            .deck(1, &["DECK", "DECK"])
            .memory(5).start();
        r.game.current_phase = GamePhase::Main;
        let before: Vec<_> = r.game.players[1].security.iter().map(|card| card.handle()).collect();
        let old_deck: Vec<_> = r.game.players[1].deck.iter().map(|card| card.handle()).collect();
        r.game.drain_events();
        assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
        r.game.decode_action(PLAY_HAND_START, 0);
        let view = r.pending_selection_view().expect("mandatory physical security selection");
        assert_eq!(view.selecting_player, 0);
        assert!(!view.is_optional);
        assert_eq!(view.valid_action_ids.len(), before.len());
        let mask = build_action_mask(&r.game, view.selecting_player);
        assert_eq!(mask[PASS as usize], 0.0);
        assert!(view.valid_action_ids.iter().all(|&action| mask[action as usize] == 1.0));
        r.execute_action(view.selecting_player, view.valid_action_ids[selected]).unwrap();
        r.auto_resolve().unwrap();
        assert!(r.pending_selection_view().is_none());
        assert_eq!(r.game.players[1].deck.last().unwrap().handle(), before[selected]);
        assert_eq!(r.game.players[1].deck[..old_deck.len()].iter().map(|card| card.handle()).collect::<Vec<_>>(), old_deck);
        let remaining: Vec<_> = before.iter().enumerate().filter_map(|(index, &card)| (index != selected).then_some(card)).collect();
        assert_eq!(r.game.players[1].security.iter().map(|card| card.handle()).collect::<Vec<_>>(), remaining);
        assert!(r.game.players[1].trash.is_empty());
        assert!(!r.game.events.iter().any(|event| matches!(event,
            digimon_engine::events::GameEvent::SecurityReveal { .. })), "a selected return is not a security check");
    }
}

fn pass_until_next_turn(r: &mut DebugRunner) {
    let ending_player = r.game.turn_player();
    for _ in 0..4 {
        assert!(!r.game.game_over);
        let mask = build_action_mask(&r.game, ending_player);
        assert_eq!(mask[PASS as usize], 1.0, "canonical breeding/main pass must be legal");
        r.game.decode_action(PASS, ending_player);
        r.auto_resolve().unwrap();
        if r.game.turn_player() != ending_player { return; }
    }
    panic!("legal passes did not advance the canonical turn cycle");
}

#[test]
fn start_of_opponents_turn_fires_once_in_the_other_players_window_for_both_owners() {
    let yaml = r#"
card: OPPONENT-TURN-WITNESS
name: Opponent turn witness
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: start_of_opponents_turn
    process:
      - draw: { of: you, count: 1 }
"#;
    for owner in [0u8, 1] {
        let mut r = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
            .add_card(make_test_card("OWN-DRAW", "Owner draw card"))
            .add_card(make_test_card("OPP-DRAW", "Opponent draw card"))
            .deck(owner, &["OWN-DRAW"; 20]).deck(1 - owner, &["OPP-DRAW"; 20])
            .hand(0, &[]).hand(1, &[]).memory(5).start();
        r.place_on_field(owner, "OPPONENT-TURN-WITNESS", Some(0));
        r.game.turn_player_idx = owner as usize;
        r.game.turn_count = 2;
        r.game.current_phase = GamePhase::Main;
        assert!(r.game.players[owner as usize].hand.is_empty());
        pass_until_next_turn(&mut r);
        assert_eq!(r.game.turn_player(), 1 - owner);
        assert!(matches!(r.game.current_phase, GamePhase::Breeding | GamePhase::Main),
            "new turn has fully passed start/unsuspend/draw before inspecting the trigger");
        assert_eq!(r.game.players[(1 - owner) as usize].hand.len(), 1,
            "new turn player's ordinary draw has already completed");
        assert_eq!(r.game.players[owner as usize].hand.len(), 1, "one effect draw during opponent start");
        assert_eq!(r.game.players[owner as usize].hand[0].card_id(&r.game.card_data), "OWN-DRAW");
        r.game.drain_effect_queue();
        assert_eq!(r.game.players[owner as usize].hand.len(), 1, "draining again cannot duplicate the window");
        pass_until_next_turn(&mut r);
        assert_eq!(r.game.turn_player(), owner);
        assert_eq!(r.game.players[owner as usize].hand.len(), 2, "own start adds only its ordinary draw, not this effect");
        pass_until_next_turn(&mut r);
        assert_eq!(r.game.turn_player(), 1 - owner);
        assert_eq!(r.game.players[owner as usize].hand.len(), 3, "next opponent window adds exactly one further effect draw");
    }
}

#[test]
fn parked_opponent_start_choice_delays_unsuspend_and_draw_then_resumes_once() {
    let yaml = r#"
card: PARKED-OPPONENT-START
name: Parked opponent start
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: start_of_opponents_turn
    process:
      - select_effect_choice:
          bind_as: branch
          labels: [Draw, Draw also]
          prompt: Finish opponent start before unsuspend
      - draw: { of: you, count: 1 }
"#;
    for owner in [0u8, 1] {
        let mut r = DebugRunner::builder().from_dsl_yaml(yaml).unwrap()
            .add_card(make_test_card("PAD", "Pad"))
            .deck(0, &["PAD"; 20]).deck(1, &["PAD"; 20])
            .hand(0, &[]).hand(1, &[]).memory(5).start();
        r.place_on_field(owner, "PARKED-OPPONENT-START", Some(0));
        let waiting = r.place_on_field(1 - owner, "PAD", Some(0));
        r.game.players[(1 - owner) as usize].battle_area[waiting.index as usize].is_suspended = true;
        r.game.turn_player_idx = owner as usize;
        r.game.turn_count = 2;
        r.game.current_phase = GamePhase::Main;
        assert_eq!(build_action_mask(&r.game, owner)[PASS as usize], 1.0);
        r.game.decode_action(PASS, owner);
        assert_eq!(r.game.turn_player(), 1 - owner);
        let view = r.pending_selection_view().expect("opponent-start body parks the turn");
        assert_eq!(view.selecting_player, owner);
        assert_eq!(view.valid_action_ids.len(), 2);
        assert!(r.game.players.iter().all(|player| player.hand.is_empty()));
        assert!(r.game.players[(1 - owner) as usize].battle_area[waiting.index as usize].is_suspended);
        let action = view.valid_action_ids[0];
        assert_eq!(build_action_mask(&r.game, owner)[action as usize], 1.0);
        r.execute_action(owner, action).unwrap();
        r.auto_resolve().unwrap();
        assert!(matches!(r.game.current_phase, GamePhase::Breeding | GamePhase::Main));
        assert!(!r.game.players[(1 - owner) as usize].battle_area[waiting.index as usize].is_suspended);
        assert_eq!(r.game.players[owner as usize].hand.len(), 1, "the parked body's one effect draw");
        assert_eq!(r.game.players[(1 - owner) as usize].hand.len(), 1, "one ordinary draw after the whole window");
        r.game.drain_effect_queue();
        assert!(r.game.players.iter().all(|player| player.hand.len() == 1));
    }
}

#[test]
fn own_and_opponent_start_effects_share_turn_player_priority_before_ordinary_draw() {
    let own = r#"
card: OWN-START-CHOICE
name: Own start choice
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: start_of_your_turn
    process:
      - select_effect_choice:
          bind_as: branch
          labels: [Finish own start, Finish own start also]
          prompt: Resolve turn player first
      - gain_memory: 1
"#;
    let other = r#"
card: OTHER-START-CHOICE
name: Other start choice
kind: digimon
level: 3
color: [yellow]
cost: 0
dp: 1000
effects:
  - when: start_of_opponents_turn
    process:
      - select_effect_choice:
          bind_as: branch
          labels: [Finish other start, Finish other start also]
          prompt: Resolve opponent after turn player
      - draw: { of: you, count: 1 }
"#;
    for ending in [0u8, 1] {
        let next = 1 - ending;
        let mut r = DebugRunner::builder().from_dsl_yaml(own).unwrap().from_dsl_yaml(other).unwrap()
            .add_card(make_test_card("PAD", "Pad"))
            .deck(0, &["PAD"; 20]).deck(1, &["PAD"; 20])
            .hand(0, &[]).hand(1, &[]).memory(5).start();
        r.place_on_field(next, "OWN-START-CHOICE", Some(0));
        r.place_on_field(ending, "OTHER-START-CHOICE", Some(0));
        r.game.turn_player_idx = ending as usize;
        r.game.turn_count = 2;
        r.game.current_phase = GamePhase::Main;
        assert_eq!(build_action_mask(&r.game, ending)[PASS as usize], 1.0);
        r.game.decode_action(PASS, ending);
        let view = r.pending_selection_view().expect("turn player's start effect first");
        assert_eq!(view.selecting_player, next);
        let action = view.valid_action_ids[0];
        assert_eq!(build_action_mask(&r.game, next)[action as usize], 1.0);
        r.execute_action(next, action).unwrap();
        let view = r.pending_selection_view().expect("opponent's start effect remains in the same window");
        assert_eq!(view.selecting_player, ending);
        assert!(r.game.players.iter().all(|player| player.hand.is_empty()), "ordinary draw waits for both controllers");
        let action = view.valid_action_ids[0];
        assert_eq!(build_action_mask(&r.game, ending)[action as usize], 1.0);
        r.execute_action(ending, action).unwrap();
        r.auto_resolve().unwrap();
        assert_eq!(r.game.memory, 4, "one turn-player memory effect after passing for three");
        assert!(r.game.players.iter().all(|player| player.hand.len() == 1));
        assert!(matches!(r.game.current_phase, GamePhase::Breeding | GamePhase::Main));
    }
}
