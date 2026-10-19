//! Official EX2-055 Set(0) / Q3347 physical-prefix expectations.
//! REAPER_TEST_YAML permits declaration-only behavioral RED; never ship that baseline.
use digimon_engine::action::{
    mask::build_action_mask,
    space::{encode_attack, PASS, PLAY_HAND_START, SECURITY_TARGET, TRASH_EFFECT_START},
};
use digimon_engine::count_choice::{
    CountChoiceOperation, COUNT_NEXT_ACTION, COUNT_PREVIOUS_ACTION,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
const MOTHER:&str="card: EX2-007\nname: Mother D-Reaper\nkind: digi_egg\ncolor: [white]\ndp: 15000\nform: D-Reaper\ntraits: [Ability Synthesis Agent]\neffects: []\n";
fn runner(n: usize, x_at: Option<usize>) -> DebugRunner {
    let yaml =
        std::fs::read_to_string(std::env::var("REAPER_TEST_YAML").unwrap_or_else(|_| {
            concat!(env!("CARGO_MANIFEST_DIR"), "/cards/ex2/EX2-055.yaml").into()
        }))
        .expect("Reaper YAML or explicit metadata baseline");
    let mut ace = make_test_card("PAY-ACE", "Payment ACE");
    ace.ace_overflow = Some(-4);
    let mut r=DebugRunner::builder().add_card(ace)
 .from_dsl_yaml(include_str!("../cards/ex2/EX2-046.yaml")).unwrap()
 .from_dsl_yaml("card: PAY-OBSERVER\nname: Source observer\nkind: tamer\ncolor: [white]\ncost: 1\neffects:\n  - when: on_digivolution_card_trashed\n    optional: true\n    once_per_turn: true\n    process:\n      - draw: {of: you, count: 1}\n").unwrap()
 .from_dsl_yaml("card: REAPER-SET-THREE\nname: Reaper setter three\nkind: tamer\ncolor: [white]\ncost: 1\neffects:\n  - kind: cost_assignment\n    set_cost: 3\n    reduction_timing: before_pay_cost\n    when_any_ally_played: { name_is: Reaper }\n    pay_cost:\n      - draw: {of: you, count: 1}\n").unwrap()
 .from_dsl_yaml(&yaml).unwrap().from_dsl_yaml(include_str!("../cards/ex2/EX2-007.yaml")).unwrap()
 .from_dsl_yaml(include_str!("../cards/bt9/BT9-109.yaml")).unwrap()
 .add_card(make_test_card("PAY-SOURCE","Payment source")).hand(0,&["EX2-055"])
 .security(1,&["PAY-SOURCE";4]).deck(0,&["PAY-SOURCE";12]).deck(1,&["PAY-SOURCE";12]).memory(10).start();
    r.skip_mulligan();
    let mut stack = vec!["EX2-046"; n];
    if let Some(i) = x_at {
        stack[i] = "BT9-109";
    }
    stack.push("EX2-007");
    r.place_stack(0, &stack);
    r.game.enter_main_phase();
    r
}
fn select(r: &mut DebugRunner, a: u16) {
    assert_eq!(
        build_action_mask(&r.game, 0)[a as usize],
        1.0,
        "choice {a} must actually be legal"
    );
    r.execute_action(0, a).unwrap();
}
fn start(r: &mut DebugRunner) {
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 1.0);
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(
        r.game.pending_selection.is_some(),
        "Set(0) must offer actual optional physical host payment"
    );
}
fn host(r: &mut DebugRunner) {
    start(r);
    assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
    select(r, 100);
    assert!(
        r.game
            .pending_selection
            .as_ref()
            .and_then(|p| p.effect_choices.as_ref())
            .is_some(),
        "physical prefix count must be explicit, even singleton"
    );
}
fn offered(r: &DebugRunner) -> Vec<(u64, u64, u16)> {
    r.game
        .pending_selection
        .as_ref()
        .unwrap()
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .filter_map(|c| match c.count_choice?.operation {
            CountChoiceOperation::Commit(v) => Some((v.declared, v.payable, c.action_id)),
            _ => None,
        })
        .collect()
}
fn commit(r: &mut DebugRunner, n: u64) {
    loop {
        if let Some((_, _, a)) = offered(r).into_iter().find(|v| v.0 == n) {
            select(r, a);
            return;
        }
        select(r, COUNT_NEXT_ACTION);
    }
}
fn reaper_present(r: &DebugRunner) -> bool {
    r.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&r.game.card_data) == "EX2-055")
}

fn choose_source(r: &mut DebugRunner, card: digimon_engine::card_source::CardHandle) {
    let a = r
        .game
        .pending_selection
        .as_ref()
        .unwrap()
        .effect_choices
        .as_ref()
        .unwrap()
        .iter()
        .find(|e| e.source_card == Some(card))
        .expect("physical effect source is offered")
        .action_id;
    select(r, a);
}
fn begin_reaper_payment(r: &mut DebugRunner) {
    let card = r.game.players[0].hand[0].handle();
    start(r);
    choose_source(r, card);
}
fn decline_remaining(r: &mut DebugRunner) {
    for _ in 0..12 {
        if r.game.pending_selection.is_none() {
            return;
        }
        assert_eq!(
            build_action_mask(&r.game, 0)[PASS as usize],
            1.0,
            "only ordinary optional remaining gates may be declined"
        );
        select(r, PASS);
    }
    panic!("unsettled optional payment");
}
#[test]
fn real_mother_searcher_x_prefix_preserves_actual_protection_and_owner_effects() {
    let mut r = runner(8, Some(0));
    let mother = r.game.players[0].battle_area[0].top_card().handle();
    let x = r.game.players[0].battle_area[0].card_sources[0].handle();
    begin_reaper_payment(&mut r);
    select(&mut r, 100);
    assert_eq!(offered(&r), vec![(8, 7, 30)]);
    commit(&mut r, 8);
    decline_remaining(&mut r);
    assert!(reaper_present(&r));
    assert_eq!(r.memory(), 10);
    assert_eq!(
        r.game.players[0]
            .trash
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "EX2-046")
            .count(),
        7
    );
    let host = r.game.players[0]
        .battle_area
        .iter()
        .find(|p| p.top_card().handle() == mother)
        .unwrap();
    assert_eq!(host.card_sources.len(), 2);
    assert_eq!(host.card_sources[0].handle(), x);
}
#[test]
fn multiple_real_mothers_offer_each_payable_host_exclude_insufficient_protected_and_opponent() {
    let mut r = runner(7, Some(0));
    r.place_stack(
        0,
        &[
            "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-007",
        ],
    );
    r.place_stack(
        0,
        &[
            "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-007",
        ],
    );
    r.place_stack(
        1,
        &[
            "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-046", "EX2-007",
        ],
    );
    begin_reaper_payment(&mut r);
    let mask = build_action_mask(&r.game, 0);
    assert_eq!(mask[100], 0.0);
    assert_eq!(mask[101], 1.0);
    assert_eq!(mask[102], 1.0);
    assert_eq!(mask[110], 0.0);
    let saved = r.game.clone();
    for index in [1usize, 2] {
        r.game = saved.clone();
        select(&mut r, 100 + index as u16);
        commit(&mut r, 7);
        decline_remaining(&mut r);
        assert!(reaper_present(&r));
        assert_eq!(r.game.players[0].battle_area[index].card_sources.len(), 1);
        assert_eq!(
            r.game.players[0].battle_area[3 - index].card_sources.len(),
            8
        );
        assert_eq!(r.game.players[1].battle_area[0].card_sources.len(), 8);
    }
}
#[test]
fn real_mother_reduction_and_reaper_assignment_both_orders_are_legal_and_clone_safe() {
    let mut r = runner(7, None);
    let reaper = r.game.players[0].hand[0].handle();
    let mother = r.game.players[0].battle_area[0].top_card().handle();
    start(&mut r);
    let saved = r.game.clone();
    for mother_first in [false, true] {
        r.game = saved.clone();
        choose_source(&mut r, if mother_first { mother } else { reaper });
        if mother_first {
            let accept = *r
                .game
                .pending_selection
                .as_ref()
                .unwrap()
                .valid_action_ids
                .iter()
                .find(|a| **a != PASS)
                .unwrap();
            select(&mut r, accept);
        }
        select(&mut r, 100);
        commit(&mut r, 7);
        decline_remaining(&mut r);
        assert!(reaper_present(&r));
        assert_eq!(r.memory(), 10);
        assert_eq!(r.game.players[0].trash.len(), 7);
    }
}

#[test]
fn real_reaper_and_another_paid_setter_expose_both_final_assignment_orders() {
    let mut r = runner(7, None);
    let setter = r.place_on_field(0, "REAPER-SET-THREE", None);
    let setter_card = r.game.players[0].battle_area[setter.index as usize]
        .top_card()
        .handle();
    let reaper = r.game.players[0].hand[0].handle();
    let mother = r.game.players[0].battle_area[0].top_card().handle();
    start(&mut r);
    choose_source(&mut r, mother);
    select(&mut r, PASS);
    let saved = r.game.clone();
    for reaper_first in [false, true] {
        r.game = saved.clone();
        choose_source(&mut r, if reaper_first { reaper } else { setter_card });
        if !reaper_first {
            choose_source(&mut r, setter_card);
        }
        select(&mut r, 100);
        commit(&mut r, 7);
        if reaper_first {
            choose_source(&mut r, setter_card);
        }
        assert!(r.game.pending_selection.is_none());
        assert!(reaper_present(&r));
        assert_eq!(
            r.memory(),
            if reaper_first { 7 } else { 10 },
            "last completed setter establishes base"
        );
        assert_eq!(
            r.game.players[0].hand.len(),
            1,
            "both payment costs complete, including setter draw"
        );
        assert_eq!(r.game.players[0].trash.len(), 7);
    }
}
