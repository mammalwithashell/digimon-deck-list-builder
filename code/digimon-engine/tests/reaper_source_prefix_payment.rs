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
 .from_dsl_yaml(&yaml).unwrap().from_dsl_yaml(MOTHER).unwrap()
 .from_dsl_yaml(include_str!("../cards/bt9/BT9-109.yaml")).unwrap()
 .add_card(make_test_card("PAY-SOURCE","Payment source")).hand(0,&["EX2-055"])
 .security(1,&["PAY-SOURCE";4]).deck(0,&["PAY-SOURCE";12]).deck(1,&["PAY-SOURCE";12]).memory(10).start();
    r.skip_mulligan();
    let mut stack = vec!["PAY-SOURCE"; n];
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
#[test]
fn printed_reaper_metadata_remains_twenty_no_level() {
    let r = runner(7, None);
    let c = &r.game.players[0].hand[0];
    assert_eq!(c.play_cost(&r.game.card_data), 20);
    assert_eq!(c.level(&r.game.card_data), None);
    assert_eq!(c.dp(&r.game.card_data), Some(15000));
    assert_eq!(c.card_kind(&r.game.card_data), CardKind::Digimon);
}
#[test]
fn seven_source_singleton_is_explicit_and_sets_zero() {
    let mut r = runner(7, None);
    host(&mut r);
    assert_eq!(offered(&r), vec![(7, 7, 30)]);
    assert_eq!(r.memory(), 10);
    commit(&mut r, 7);
    assert_eq!(r.memory(), 10);
    assert!(reaper_present(&r));
    assert_eq!(r.game.players[0].trash.len(), 7);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
}
#[test]
fn all_prefixes_through_300_are_reachable_with_reversible_navigation() {
    for n in [8, 36, 37, 42, 300] {
        let mut r = runner(n, None);
        host(&mut r);
        let before = r.game.verification_digest();
        let mut seen = vec![];
        loop {
            seen.extend(offered(&r).into_iter().map(|v| v.0));
            if build_action_mask(&r.game, 0)[COUNT_NEXT_ACTION as usize] == 0.0 {
                break;
            }
            select(&mut r, COUNT_NEXT_ACTION);
            assert_eq!(r.memory(), 10);
            assert_eq!(r.game.players[0].trash.len(), 0);
        }
        assert_eq!(seen, (7..=n as u64).collect::<Vec<_>>());
        while build_action_mask(&r.game, 0)[COUNT_PREVIOUS_ACTION as usize] > 0.0 {
            select(&mut r, COUNT_PREVIOUS_ACTION);
        }
        assert_eq!(r.game.verification_digest(), before);
        commit(&mut r, n as u64);
        assert_eq!(r.game.players[0].trash.len(), n);
        assert_eq!(r.memory(), 10);
    }
}
#[test]
fn x_antibody_counts_in_physical_prefix_but_not_actual_payment() {
    for x in [0, 3, 7] {
        let mut r = runner(8, Some(x));
        host(&mut r);
        assert_eq!(
            offered(&r),
            if x < 7 {
                vec![(8, 7, 30)]
            } else {
                vec![(7, 7, 30), (8, 7, 31)]
            }
        );
        commit(&mut r, 8);
        assert_eq!(r.memory(), 10);
        assert_eq!(r.game.players[0].trash.len(), 7);
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 2);
        assert_eq!(
            r.game.players[0].battle_area[0].card_sources[0].card_id(&r.game.card_data),
            "BT9-109"
        );
    }
}
#[test]
fn protected_prefix_boundary_retains_every_legal_declared_count() {
    let mut r = runner(10, Some(8));
    host(&mut r);
    assert_eq!(
        offered(&r),
        vec![(7, 7, 30), (8, 8, 31), (9, 8, 32), (10, 9, 33)]
    );
    commit(&mut r, 9);
    assert_eq!(r.game.players[0].trash.len(), 8);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 3);
}
#[test]
fn insufficient_actual_payment_does_not_offer_assignment() {
    let mut r = runner(7, Some(0));
    r.game.decode_action(PLAY_HAND_START, 0);
    assert!(r.game.pending_selection.is_none());
    assert!(reaper_present(&r));
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 8);
}
#[test]
fn optional_host_decline_does_not_trash_or_assign() {
    let mut r = runner(7, None);
    start(&mut r);
    select(&mut r, PASS);
    assert!(reaper_present(&r));
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 8);
    assert_eq!(r.game.memory, 10);
    assert_eq!(
        r.game.turn_player(),
        1,
        "ordinary twenty payment passes the turn"
    );
}
#[test]
fn later_page_clone_commits_independent_exact_prefix() {
    let mut r = runner(42, None);
    host(&mut r);
    select(&mut r, COUNT_NEXT_ACTION);
    let saved = r.game.clone();
    commit(&mut r, 42);
    assert_eq!(r.game.players[0].trash.len(), 42);
    r.game = saved;
    commit(&mut r, 35);
    assert_eq!(r.game.players[0].trash.len(), 35);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 8);
    assert_eq!(r.memory(), 10);
}
#[test]
fn reordered_physical_prefix_fails_without_assignment() {
    let mut r = runner(8, None);
    host(&mut r);
    r.game.players[0].battle_area[0].card_sources.swap(0, 1);
    commit(&mut r, 7);
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 9);
    assert_eq!(r.game.turn_player(), 1);
}
#[test]
fn fresh_same_card_host_is_not_original_payment_host() {
    let mut r = runner(7, None);
    host(&mut r);
    let old = r.game.players[0].battle_area.remove(0);
    r.place_stack(
        0,
        &[
            "PAY-SOURCE",
            "PAY-SOURCE",
            "PAY-SOURCE",
            "PAY-SOURCE",
            "PAY-SOURCE",
            "PAY-SOURCE",
            "PAY-SOURCE",
            "EX2-007",
        ],
    );
    assert_ne!(
        old.top_card().handle(),
        r.game.players[0].battle_area[0].top_card().handle()
    );
    commit(&mut r, 7);
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 8);
    assert_eq!(r.game.turn_player(), 1);
}
#[test]
fn successfully_paid_reaper_has_rush_immediately() {
    let mut r = runner(7, None);
    host(&mut r);
    commit(&mut r, 7);
    assert_eq!(
        build_action_mask(&r.game, 0)[encode_attack(1, SECURITY_TARGET) as usize],
        1.0
    );
}

#[test]
fn payable_assignment_exposes_native_play_at_zero_memory_without_mask_side_effects() {
    let mut r = runner(7, None);
    r.game.set_memory(0);
    let before = r.game.verification_digest();
    assert_eq!(
        build_action_mask(&r.game, 0)[PLAY_HAND_START as usize],
        1.0,
        "a payable Set(0) is an affordable native declaration"
    );
    assert_eq!(
        r.game.verification_digest(),
        before,
        "availability cannot pay or allocate a receipt"
    );
    host(&mut r);
    commit(&mut r, 7);
    assert!(reaper_present(&r));
    assert_eq!(r.memory(), 0);
}
#[test]
fn insufficient_or_opponent_sources_cannot_fund_zero_memory_declaration() {
    for protected in [false, true] {
        let mut r = runner(
            if protected { 7 } else { 6 },
            if protected { Some(0) } else { None },
        );
        r.game.set_memory(0);
        assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 0.0);
    }
    let mut r = runner(7, None);
    let mother = r.game.players[0].battle_area.remove(0);
    r.game.players[1].battle_area.push(mother);
    r.game.set_memory(0);
    assert_eq!(build_action_mask(&r.game, 0)[PLAY_HAND_START as usize], 0.0);
}

#[test]
fn source_payment_applies_owner_relative_ace_overflow_and_owner_trash() {
    for owner in [0, 1] {
        let mut r = runner(7, None);
        let data = r
            .game
            .card_data
            .iter()
            .position(|c| c.card_id == "PAY-ACE")
            .unwrap();
        let ace =
            digimon_engine::card_source::CardSource::new(data, owner, r.game.next_card_index());
        let handle = ace.handle();
        r.game.players[0].battle_area[0].card_sources[0] = ace;
        r.game.set_memory(3);
        host(&mut r);
        commit(&mut r, 7);
        let relative = if r.game.turn_player() == 0 {
            r.game.memory
        } else {
            -r.game.memory
        };
        assert_eq!(relative, if owner == 0 { -1 } else { 7 });
        assert!(r.game.players[owner as usize]
            .trash
            .iter()
            .any(|c| c.handle() == handle));
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
        assert!(reaper_present(&r));
    }
}
#[test]
fn all_source_payment_moves_complete_before_first_observer_prompt() {
    let mut r = runner(7, None);
    r.place_on_field(0, "PAY-OBSERVER", None);
    let paid: Vec<_> = r.game.players[0].battle_area[0]
        .card_sources
        .iter()
        .take(7)
        .map(|c| c.handle())
        .collect();
    host(&mut r);
    commit(&mut r, 7);
    assert!(
        r.game.pending_selection.is_some(),
        "source-trash observers must still trigger"
    );
    for h in paid {
        assert!(
            r.game.players[0].trash.iter().any(|c| c.handle() == h),
            "no partial payment is visible to an observer"
        );
    }
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    let saved = r.game.clone();
    for accept in [false, true] {
        r.game = saved.clone();
        for _ in 0..20 {
            if r.game.pending_selection.is_none() {
                break;
            }
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
            select(&mut r, a);
        }
        assert!(r.game.pending_selection.is_none());
        assert!(reaper_present(&r));
        assert_eq!(r.memory(), 10);
    }
}
fn attack_fixture(searchers: usize) -> DebugRunner {
    let mut r = runner(0, None);
    r.game.players[0].battle_area.clear();
    r.game.players[0].hand.clear();
    r.place_on_field(0, "EX2-055", Some(0));
    for _ in 0..searchers {
        r.inject_trash(0, "EX2-046");
    }
    r.inject_trash(0, "PAY-SOURCE");
    r.game.enter_main_phase();
    r
}
fn attack_start(r: &mut DebugRunner) {
    let a = encode_attack(0, SECURITY_TARGET);
    assert_eq!(build_action_mask(&r.game, 0)[a as usize], 1.0);
    r.game.decode_action(a, 0);
}
#[test]
fn attack_payment_two_searchers_has_both_physical_orders_and_clone_safe_unsuspend() {
    let mut r = attack_fixture(3);
    attack_start(&mut r);
    let saved = r.game.clone();
    let chosen = [
        r.game.players[0].trash[0].handle(),
        r.game.players[0].trash[1].handle(),
    ];
    for (first, second) in [(0usize, 1usize), (1, 0)] {
        r.game = saved.clone();
        select(&mut r, TRASH_EFFECT_START + first as u16);
        assert_eq!(
            build_action_mask(&r.game, 0)[PASS as usize],
            0.0,
            "after starting the two-card cost it cannot finish with one"
        );
        select(&mut r, TRASH_EFFECT_START + second as u16);
        assert!(!r.game.players[0].battle_area[0].is_suspended);
        let stack = &r.game.players[0].battle_area[0].card_sources;
        assert_eq!(stack.len(), 3);
        assert_eq!(stack[0].handle(), chosen[second]);
        assert_eq!(stack[1].handle(), chosen[first]);
        assert_eq!(r.game.players[0].trash.len(), 2);
    }
}
#[test]
fn attack_cost_decline_and_insufficient_searchers_do_not_unsuspend() {
    for n in [1, 2] {
        let mut r = attack_fixture(n);
        attack_start(&mut r);
        if n == 2 {
            assert_eq!(build_action_mask(&r.game, 0)[PASS as usize], 1.0);
            select(&mut r, PASS);
        }
        assert!(r.game.players[0].battle_area[0].is_suspended);
        assert_eq!(r.game.players[0].trash.len(), n + 1);
        assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    }
}

#[test]
fn zero_memory_decline_restores_unpaid_declaration_without_assignment() {
    let mut r = runner(7, None);
    r.game.set_memory(0);
    start(&mut r);
    select(&mut r, PASS);
    assert!(!reaper_present(&r));
    assert_eq!(r.game.players[0].hand.len(), 1);
    assert_eq!(
        r.game.players[0].hand[0].card_id(&r.game.card_data),
        "EX2-055"
    );
    assert_eq!(r.game.players[0].trash.len(), 0);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 8);
    assert_eq!(r.memory(), 0);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn attacking_payment_can_be_used_again_same_turn_and_excludes_non_searchers() {
    let mut r = attack_fixture(4);
    for expected_sources in [3usize, 5usize] {
        attack_start(&mut r);
        assert!(
            r.game.pending_selection.is_some(),
            "printed When Attacking has no once-per-turn limit"
        );
        let wrong = r.game.players[0]
            .trash
            .iter()
            .position(|c| c.card_id(&r.game.card_data) == "PAY-SOURCE")
            .unwrap();
        assert_eq!(
            build_action_mask(&r.game, 0)[TRASH_EFFECT_START as usize + wrong],
            0.0
        );
        select(&mut r, TRASH_EFFECT_START);
        select(&mut r, TRASH_EFFECT_START + 1);
        assert!(r.game.pending_selection.is_none());
        assert!(!r.game.players[0].battle_area[0].is_suspended);
        assert_eq!(
            r.game.players[0].battle_area[0].card_sources.len(),
            expected_sources
        );
        assert_eq!(r.game.turn_player(), 0);
    }
    assert_eq!(r.game.players[0].trash.len(), 1);
    assert_eq!(r.game.players[1].security.len(), 2);
}
#[test]
fn opponent_searchers_cannot_complete_own_attack_payment() {
    let mut r = attack_fixture(1);
    r.inject_trash(1, "EX2-046");
    r.inject_trash(1, "EX2-046");
    let opposing_searchers: Vec<_> = r.game.players[1].trash.iter().map(|c| c.handle()).collect();
    attack_start(&mut r);
    assert!(
        r.game.pending_selection.is_none(),
        "two opponent Searchers do not supply your missing second card"
    );
    assert!(r.game.players[0].battle_area[0].is_suspended);
    assert_eq!(r.game.players[0].battle_area[0].card_sources.len(), 1);
    assert_eq!(r.game.players[0].trash.len(), 2);
    for handle in opposing_searchers {
        assert!(
            r.game.players[1].trash.iter().any(|c| c.handle() == handle),
            "opponent Searcher must stay in its owner trash"
        );
    }
    assert_eq!(
        r.game.players[1]
            .trash
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "EX2-046")
            .count(),
        2
    );
    assert_eq!(
        r.game.players[1]
            .trash
            .iter()
            .filter(|c| c.card_id(&r.game.card_data) == "PAY-SOURCE")
            .count(),
        1,
        "normal security battle adds its victim to opponent trash"
    );
    assert_eq!(r.game.players[1].security.len(), 3);
}
