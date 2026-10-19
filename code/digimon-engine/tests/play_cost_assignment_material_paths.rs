//! Typed assignment must survive existing parked Assembly/DigiXros paths.
use digimon_engine::{
    action::{
        build_action_mask,
        space::{PASS, PLAY_HAND_START, TRASH_EFFECT_START},
    },
    debug_runner::{make_test_card, DebugRunner},
    rules::Rules,
};
fn fixture(assembly: bool, late: bool) -> DebugRunner {
    fixture_inner(assembly, late, false)
}
fn fixture_inner(assembly: bool, late: bool, opt: bool) -> DebugRunner {
    let path = if assembly {
        "  - kind: assembly\n    materials:\n      - {name_contains: Part A, zones: [trash], stack_under: true}\n      - {name_contains: Part B, zones: [trash], stack_under: true}\n    cost: 3"
    } else {
        "  - kind: digixros\n    materials:\n      - filter: {name_is: Part A}\n        zones: [battle_area]\n        cost_delta: -2\n    cost: 20"
    };
    let target=format!("card: SET-MATERIAL-TARGET\nname: Material target\nkind: digimon\nlevel: 6\ncolor: [white]\ncost: 20\ndp: 12000\nalt_paths:\n{path}\n");
    let setter="card: SET-MATERIAL-SETTER\nname: Material setter\nkind: tamer\ncolor: [white]\ncost: 1\neffects:\n  - kind: cost_assignment\n    when_any_ally_played: {name_is: Material target}\n    set_cost: 5\n    pay_cost:\n      - draw: {of: you, count: 1}\n";
    let setter = if std::env::var_os("COST_ASSIGNMENT_BASELINE").is_some() {
        setter
            .replace("kind: cost_assignment", "kind: cost_reduction")
            .replace("set_cost: 5", "amount: 0")
    } else {
        setter.to_string()
    };
    let material = if late {
        "card: SET-MATERIAL-A\nname: Part A\nkind: digimon\nlevel: 4\ncolor: [white]\ncost: 3\ndp: 3000\neffects:\n  - kind: replacement\n    trigger: when_would_leave_battle_area\n    optional: true\n    active_when: {replacement_subject_is_mine: true, name_is: Part A}\n    process:\n      - draw: {of: you, count: 1}\n"
    } else {
        "card: SET-MATERIAL-A\nname: Part A\nkind: digimon\nlevel: 4\ncolor: [white]\ncost: 3\ndp: 3000\neffects: []\n"
    };
    let material = if opt {
        material.replace(
            "    optional: true\n",
            "    optional: true\n    once_per_turn: true\n",
        )
    } else {
        material.to_string()
    };
    let mut rules = Rules::standard();
    rules.memory_range = (-100, 100);
    let mut r = DebugRunner::builder()
        .with_rules(rules)
        .from_dsl_yaml(&target)
        .unwrap()
        .from_dsl_yaml(&setter)
        .unwrap()
        .from_dsl_yaml(&material)
        .unwrap()
        .add_card(make_test_card("SET-MATERIAL-B", "Part B"))
        .add_card(make_test_card("SET-MATERIAL-DRAW", "Marker"))
        .hand(0, &["SET-MATERIAL-TARGET"])
        .deck(0, &["SET-MATERIAL-DRAW"; 12])
        .memory(100)
        .start();
    r.skip_mulligan();
    r.place_on_field(0, "SET-MATERIAL-SETTER", None);
    if assembly {
        r.inject_trash(0, "SET-MATERIAL-A");
        r.inject_trash(0, "SET-MATERIAL-B");
    } else {
        r.place_on_field(0, "SET-MATERIAL-A", None);
    }
    r.game.enter_main_phase();
    r
}
fn choose(r: &mut DebugRunner, a: u16) {
    assert_eq!(
        build_action_mask(&r.game, 0)[a as usize],
        1.0,
        "actual choice {a}"
    );
    r.execute_action(0, a).unwrap();
}
fn start(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    choose(r, 30);
}
fn played_sources(r: &DebugRunner) -> usize {
    r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().card_id(&r.game.card_data) == "SET-MATERIAL-TARGET")
        .expect("completed target")
        .card_sources
        .len()
}
#[test]
fn assignment_survives_assembly_decline_and_both_material_parks_in_clones() {
    let mut r = fixture(true, false);
    start(&mut r);
    assert_eq!(r.memory(), 100);
    let saved = r.game.clone();
    choose(&mut r, PASS);
    assert_eq!(r.memory(), 95);
    assert_eq!(played_sources(&r), 1);
    r.game = saved;
    choose(&mut r, TRASH_EFFECT_START);
    let second = r.game.clone();
    for _ in 0..2 {
        r.game = second.clone();
        choose(&mut r, TRASH_EFFECT_START + 1);
        assert_eq!(r.memory(), 98, "Set5 minus Assembly3");
        assert_eq!(played_sources(&r), 3);
        assert_eq!(r.game.players[0].trash.len(), 0);
    }
}
#[test]
fn assignment_survives_digixros_material_selection_and_stop_clones() {
    let mut r = fixture(false, false);
    start(&mut r);
    let saved = r.game.clone();
    choose(&mut r, PASS);
    assert_eq!(r.memory(), 95);
    assert_eq!(played_sources(&r), 1);
    r.game = saved;
    choose(&mut r, 1);
    let selected = r.game.clone();
    for _ in 0..2 {
        r.game = selected.clone();
        choose(&mut r, PASS);
        assert_eq!(r.memory(), 97, "Set5 minus selected DigiXros2");
        assert_eq!(played_sources(&r), 2);
    }
}
#[test]
fn assignment_survives_late_digixros_leave_window_accept_and_decline() {
    let mut r = fixture(false, true);
    start(&mut r);
    choose(&mut r, 1);
    choose(&mut r, PASS);
    assert!(
        r.game.pending_selection.is_some(),
        "actual late material leave window parks"
    );
    assert_eq!(r.memory(), 100);
    let saved = r.game.clone();
    for accept in [false, true] {
        r.game = saved.clone();
        let a = if accept {
            *r.game
                .pending_selection
                .as_ref()
                .unwrap()
                .valid_action_ids
                .iter()
                .find(|a| **a != PASS)
                .unwrap()
        } else {
            PASS
        };
        choose(&mut r, a);
        assert!(r.game.pending_selection.is_none());
        assert_eq!(r.memory(), 97);
        assert_eq!(played_sources(&r), 2);
        assert_eq!(r.game.players[0].hand.len(), if accept { 2 } else { 1 });
    }
}

#[test]
fn own_once_per_turn_leave_effect_survives_digixros_limbo_in_both_cloned_branches() {
    let mut r = fixture_inner(false, true, true);
    start(&mut r);
    choose(&mut r, 1);
    choose(&mut r, PASS);
    let parked = r.game.clone();
    assert!(parked.pending_selection.is_some());
    for accept in [false, true] {
        for _ in 0..2 {
            r.game = parked.clone();
            let a = if accept {
                *r.game
                    .pending_selection
                    .as_ref()
                    .unwrap()
                    .valid_action_ids
                    .iter()
                    .find(|a| **a != PASS)
                    .unwrap()
            } else {
                PASS
            };
            choose(&mut r, a);
            assert!(
                r.game.pending_selection.is_none(),
                "the same immediate effect must not reoffer itself"
            );
            assert_eq!(
                r.memory(),
                97,
                "the captured assignment and material reduction survive the window"
            );
            assert_eq!(played_sources(&r), 2);
            assert_eq!(
                r.game.players[0].hand.len(),
                if accept { 2 } else { 1 },
                "own paid rider resolves exactly once after accept and never after decline"
            );
        }
    }
}
