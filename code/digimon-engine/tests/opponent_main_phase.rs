//! Start-of-opponent-main is a distinct phase-entry event, not turn start.
#![cfg(feature = "dsl-yaml-loader")]
use digimon_engine::action::{
    build_action_mask,
    space::{MOVE_FROM_BREEDING, PASS},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::events::GameEvent;

fn setup(scope: &str, optional: bool) -> DebugRunner {
    let yaml = format!(
        r#"
card: TEST-OPP-MAIN
name: Opponent Main Observer
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: start_of_opponents_main_phase
    scope: {scope}
    optional: {optional}
    process:
      - gain_memory: 2
"#
    );
    let own = r#"
card: TEST-OWN-MAIN
name: Own Main Observer
kind: digimon
level: 3
color: [blue]
cost: 3
dp: 2000
effects:
  - when: start_of_your_main_phase
    optional: true
    process:
      - gain_memory: 1
  - when: on_move
    condition: { event_permanent_is_source: true }
    optional: true
    process:
      - gain_memory: 1
"#;
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&yaml)
        .expect("opponent-main timing compiles")
        .from_dsl_yaml(own)
        .unwrap()
        .add_card(make_test_card("PAD", "Padding"))
        .deck(0, &["PAD"; 12])
        .deck(1, &["PAD"; 12])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.set_first_player(0);
    r.set_phase(GamePhase::Breeding);
    r
}

fn answer(r: &mut DebugRunner, player: u8, decline: bool) {
    let selection = r.pending_selection().expect("a player-visible choice");
    assert_eq!(selection.selecting_player, player);
    let action = if decline {
        PASS
    } else {
        *selection
            .valid_action_ids
            .iter()
            .find(|&&a| a != PASS)
            .unwrap()
    };
    assert!(build_action_mask(&r.game, player)[action as usize] > 0.5);
    r.execute_action(player, action).unwrap();
}

#[test]
fn opponent_main_face_and_inherited_fire_only_at_opposing_main_entry() {
    for inherited in [false, true] {
        let mut r = setup(if inherited { "inherited" } else { "face_up" }, false);
        if inherited {
            r.place_stack(1, &["TEST-OPP-MAIN", "PAD"]);
        } else {
            r.place_on_field(1, "TEST-OPP-MAIN", Some(0));
        }
        let before = r.memory();
        assert_eq!(r.game.current_phase, GamePhase::Breeding);
        assert!(r.pending_selection().is_none());
        r.game.enter_main_phase();
        assert_eq!(r.game.current_phase, GamePhase::Main);
        assert_eq!(r.memory(), before - 2);
        r.set_first_player(1);
        r.set_phase(GamePhase::Breeding);
        let before = r.memory();
        r.game.enter_main_phase();
        assert_eq!(r.memory(), before, "own main must not refire opponent-main");
    }
}

#[test]
fn opponent_main_does_not_activate_an_ordinary_breeding_effect() {
    let mut r = setup("face_up", false);
    r.place_in_breeding(1, "TEST-OPP-MAIN");
    let before = r.memory();
    r.game.enter_main_phase();
    assert_eq!(r.memory(), before);
    assert!(r.pending_selection().is_none());
}

#[test]
fn opponent_main_respects_turn_player_priority_and_clones_optional_choice() {
    let mut r = setup("face_up", true);
    r.place_on_field(0, "TEST-OWN-MAIN", Some(0));
    r.place_on_field(1, "TEST-OPP-MAIN", Some(0));
    let before = r.memory();
    r.game.enter_main_phase();
    assert_eq!(r.pending_selection().unwrap().selecting_player, 0);
    answer(&mut r, 0, false);
    assert_eq!(r.memory(), before + 1);
    assert_eq!(r.pending_selection().unwrap().selecting_player, 1);
    let mut cloned = DebugRunner::wrap(r.game.clone());
    answer(&mut r, 1, false);
    answer(&mut cloned, 1, true);
    assert_eq!(r.memory(), before - 1);
    assert_eq!(cloned.memory(), before + 1);
    for game in [&r, &cloned] {
        assert!(game.pending_selection().is_none());
        assert_eq!(game.game.current_phase, GamePhase::Main);
    }
}

#[test]
fn opponent_main_same_controller_trigger_order_survives_clone() {
    let mut r = setup("face_up", true);
    r.place_on_field(1, "TEST-OPP-MAIN", Some(0));
    r.place_on_field(1, "TEST-OPP-MAIN", Some(0));
    let before = r.memory();
    r.game.enter_main_phase();
    let choice = r.pending_selection().unwrap();
    assert_eq!(choice.selecting_player, 1);
    assert!(
        choice.valid_action_ids.len() >= 2,
        "both effect orders must be legal"
    );
    let mut cloned = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut cloned] {
        answer(game, 1, false); // choose one of the simultaneous effects
        for _ in 0..4 {
            if game.pending_selection().is_none() {
                break;
            }
            answer(game, 1, false);
        }
        assert!(
            game.pending_selection().is_none(),
            "both triggers must finish"
        );
        assert_eq!(game.memory(), before - 4);
        assert_eq!(game.game.current_phase, GamePhase::Main);
    }
}

#[test]
fn opponent_main_waits_for_breeding_choice_and_enters_once_after_clone_resume() {
    let mut r = setup("face_up", true);
    r.place_in_breeding(0, "TEST-OWN-MAIN");
    r.place_on_field(1, "TEST-OPP-MAIN", Some(0));
    let before = r.memory();
    let checkpoint = r.game.events.len();
    assert!(build_action_mask(&r.game, 0)[MOVE_FROM_BREEDING as usize] > 0.5);
    r.game.decode_action(MOVE_FROM_BREEDING, 0);
    assert_eq!(r.pending_selection().unwrap().selecting_player, 0);
    assert!(!r.game.events[checkpoint..].iter().any(|event| matches!(
        event,
        GameEvent::PhaseChange {
            phase: GamePhase::Main,
            ..
        }
    )));
    let mut cloned = DebugRunner::wrap(r.game.clone());
    for game in [&mut r, &mut cloned] {
        answer(game, 0, false); // when moving must finish before main entry
        answer(game, 0, false); // turn player's own-main effect has priority
        answer(game, 1, false); // opposing-main observer
        assert_eq!(game.memory(), before);
        assert_eq!(
            game.game.events[checkpoint..]
                .iter()
                .filter(|event| matches!(
                    event,
                    GameEvent::PhaseChange {
                        phase: GamePhase::Main,
                        ..
                    }
                ))
                .count(),
            1
        );
        assert!(game.pending_selection().is_none());
        assert_eq!(game.game.current_phase, GamePhase::Main);
    }
}
