//! BT18-097 official observation 8e806eea; image and base DCGO BT18_097 checked.
//! CR 4.3 4-4-2: cards placed under Tamers go to the bottom.
use digimon_engine::action::{
    build_action_mask,
    space::{PASS, TRASH_EFFECT_START},
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
fn base() -> DebugRunnerBuilder {
    let mut b = DebugRunner::builder()
        .dsl_card("BT18-097")
        .expect("BT18-097 YAML");
    for (id, name) in [
        ("TEST-A", "A"),
        ("TEST-A2", "A"),
        ("TEST-B", "B"),
        ("TEST-C", "C"),
        ("TEST-D", "D"),
        ("TEST-E", "E"),
        ("TEST-F", "F"),
        ("TEST-M", "MagnaGarurumon"),
        ("TEST-O", "Other"),
    ] {
        let mut c = make_test_card(id, name);
        c.traits = if id == "TEST-O" {
            vec![]
        } else {
            vec!["Hybrid".into()]
        };
        c.level = Some(6);
        c.dp = Some(12000);
        b = b.add_card(c);
    }
    for (id, inherited) in [("TEST-T", true), ("TEST-N", false)] {
        let mut c = make_test_card(id, id);
        c.card_kind = CardKind::Tamer;
        c.level = None;
        c.dp = None;
        c.play_cost = 9;
        c.colors = vec![CardColor::Yellow, CardColor::Purple, CardColor::Black];
        if inherited {
            c.inherited_text = "[Your Turn] This Digimon gets +1000 DP.".into();
        }
        b = b.add_card(c);
    }
    b.deck(0, &["TEST-O"; 20])
        .deck(1, &["TEST-O"; 20])
        .memory(10)
}
fn setup(hand: &[&str], trash: &[&str], sources: usize) -> DebugRunner {
    let mut r = base().hand(0, hand).start();
    for id in trash {
        r.inject_trash(0, id);
    }
    let mut stack = vec!["TEST-O"; sources];
    stack.push("TEST-T");
    r.place_stack(0, &stack);
    r.game.current_phase = GamePhase::Main;
    r
}
fn main(r: &mut DebugRunner) {
    assert!(build_action_mask(&r.game, 0)[0] > 0.5);
    r.game.play_option_from_hand(0, 0);
}
fn pick(r: &mut DebugRunner, action: u16) {
    let p = r
        .game
        .pending_selection
        .as_ref()
        .expect("player choice")
        .selecting_player;
    assert!(
        build_action_mask(&r.game, p)[action as usize] > 0.5,
        "legal choice {action}"
    );
    r.execute_action(p, action).unwrap();
}
fn first(r: &mut DebugRunner) {
    let a = r
        .pending_selection()
        .unwrap()
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .unwrap();
    pick(r, a);
}
fn top(r: &DebugRunner, i: usize) -> String {
    r.game.players[0].battle_area[i]
        .top_card()
        .card_id(&r.game.card_data)
        .into()
}
#[test]
fn bt18_097_main_mixed_distinct_names_filters_and_places_at_most_five() {
    let mut r = setup(
        &["BT18-097", "TEST-A", "TEST-O"],
        &["TEST-A2", "TEST-B", "TEST-C", "TEST-D", "TEST-E", "TEST-F"],
        0,
    );
    main(&mut r);
    first(&mut r);
    let m = build_action_mask(&r.game, 0);
    assert!(m[0] > 0.5);
    assert_eq!(m[1], 0.0);
    pick(&mut r, 0);
    assert_eq!(
        build_action_mask(&r.game, 0)[TRASH_EFFECT_START as usize],
        0.0
    );
    for offset in 1..=4 {
        pick(&mut r, TRASH_EFFECT_START + offset);
    }
    assert!(r.pending_selection().is_none());
    assert_eq!(
        r.game.players[0].battle_area[0].digivolution_cards().len(),
        6
    );
    assert_eq!(top(&r, 0), "TEST-T");
    assert_eq!(
        r.game.players[0].battle_area[0]
            .card_sources
            .iter()
            .map(|c| c.card_id(&r.game.card_data))
            .collect::<Vec<_>>(),
        ["TEST-E", "TEST-D", "TEST-C", "TEST-B", "TEST-A", "TEST-T"]
    );
    assert_eq!(r.game.players[0].trash.len(), 3); // duplicate, sixth name, used Option
}
#[test]
fn bt18_097_decline_placement_still_digivolves_from_either_zone_for_free() {
    for from_trash in [false, true] {
        let mut r = if from_trash {
            setup(&["BT18-097"], &["TEST-M"], 5)
        } else {
            setup(&["BT18-097", "TEST-M"], &[], 5)
        };
        main(&mut r);
        pick(&mut r, PASS);
        first(&mut r);
        first(&mut r);
        assert_eq!(top(&r, 0), "TEST-M");
        assert_eq!(r.memory(), 7);
        assert_eq!(
            r.game.players[0].battle_area[0].digivolution_cards().len(),
            7
        );
        assert!(
            r.game.players[0]
                .hand
                .iter()
                .any(|c| c.card_id(&r.game.card_data) == "TEST-O"),
            "digivolution draw"
        );
    }
}
#[test]
fn bt18_097_digivolution_host_is_independent_of_placement_host() {
    let mut r = setup(&["BT18-097", "TEST-A", "TEST-M"], &[], 0);
    let mut stack = vec!["TEST-O"; 5];
    stack.push("TEST-N");
    r.place_stack(0, &stack);
    main(&mut r);
    first(&mut r);
    pick(&mut r, 0);
    pick(&mut r, PASS);
    assert_eq!(
        r.pending_selection()
            .unwrap()
            .valid_action_ids
            .iter()
            .filter(|a| **a != PASS)
            .count(),
        1,
        "pending={:?}; stacks={:?}",
        r.pending_selection(),
        r.game.players[0]
            .battle_area
            .iter()
            .map(|p| p.card_sources.len())
            .collect::<Vec<_>>()
    );
    first(&mut r);
    first(&mut r);
    assert_eq!(top(&r, 0), "TEST-T");
    assert_eq!(top(&r, 1), "TEST-M");
    assert_eq!(
        r.game.players[0].battle_area[0].digivolution_cards().len(),
        2
    );
}
#[test]
fn bt18_097_threshold_is_five_sources_and_evolution_is_optional() {
    for n in [4, 5, 9] {
        let mut r = setup(&["BT18-097", "TEST-M"], &[], n);
        main(&mut r);
        pick(&mut r, PASS);
        if n >= 5 {
            assert!(r.pending_selection().is_some());
            pick(&mut r, PASS);
        }
        assert!(r.pending_selection().is_none());
        assert_eq!(top(&r, 0), "TEST-T");
    }
}
#[test]
fn bt18_097_security_plays_inherited_tamer_from_hand_or_trash_then_returns_option() {
    for from_trash in [false, true] {
        let mut r = base().security(1, &["BT18-097"]).start();
        r.add_to_hand(1, "TEST-N");
        r.add_to_hand(1, "TEST-O");
        if from_trash {
            r.inject_trash(1, "TEST-T");
        } else {
            r.add_to_hand(1, "TEST-T");
        }
        let h = r.place_on_field(0, "TEST-O", Some(0));
        r.attack_player(h, 1, false);
        let m = build_action_mask(&r.game, 1);
        assert_eq!(m[0], 0.0);
        assert_eq!(m[1], 0.0);
        first(&mut r);
        r.auto_resolve().unwrap();
        assert_eq!(r.memory(), 10);
        assert_eq!(r.game.players[1].battle_area.len(), 1);
        assert_eq!(
            r.game.players[1].battle_area[0]
                .top_card()
                .card_id(&r.game.card_data),
            "TEST-T"
        );
        assert!(r.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "BT18-097"));
    }
}
#[test]
fn bt18_097_security_decline_or_no_target_still_returns_option() {
    for eligible in [false, true] {
        let mut r = base().security(1, &["BT18-097"]).start();
        r.add_to_hand(1, if eligible { "TEST-T" } else { "TEST-N" });
        let h = r.place_on_field(0, "TEST-O", Some(0));
        r.attack_player(h, 1, false);
        if eligible {
            pick(&mut r, PASS);
        }
        r.auto_resolve().unwrap();
        assert!(r.game.players[1].battle_area.is_empty());
        assert!(r.game.players[1]
            .hand
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "BT18-097"));
    }
}
