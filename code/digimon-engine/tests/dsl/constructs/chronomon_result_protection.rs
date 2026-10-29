//! CR 4.3 §§15-7-2/3/4 (pay the optional condition before its benefit),
//! 16-12-1/4 (De-Digivolve trashes the top stacked card), and 15-16-12
//! (turn-end timing). Card-text idioms: BT26-085 Giant Slayer's opponent-only
//! stack-trash protection and BT26-029 Chronomon's broader stack protection;
//! BT26-085/BT4-011 gate a follow-up on an actual effect digivolution.

use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::{EffectTiming, Expiry, ModifierType, TriggerSource};

const GUARD: &str = r#"
card: TRASH-GUARD
name: Trash Guard
kind: digimon
level: 5
color: [black]
cost: 6
dp: 7000
effects:
  - when: on_play
    process:
      - grant_stack_trash_immunity_from_opponent_effects:
          target: source
          expiry: end_of_opponents_turn
  - when: when_attacking
    process:
      - select_own_sources:
          target: source
          min: 1
          max: 1
          bind_as: paid
          prompt: Choose the source to pay
          then:
            - trash_selected_sources_as_cost: { source_refs: paid }
            - gain_memory: 2
"#;

const ALL_TRASH: &str = r#"
card: ALL-TRASH
name: All Trash
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - trash_all_sources: { target: victim }
"#;

const TOP_SOURCE: &str = r#"
card: TOP-SOURCE
name: Top Source
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - trash_top_source: { target: victim }
"#;

const TOP_STACK: &str = r#"
card: TOP-STACK
name: Top Stack
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - trash_top_stacked_sources: { target: victim, count: 1, include_top_card: true }
"#;

const DEDI: &str = r#"
card: DEDI
name: De-Digivolve
kind: tamer
color: [black]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - de_digivolve: { target: victim, amount: 1 }
"#;

const PICK_TRASH: &str = r#"
card: PICK-TRASH
name: Pick Trash
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - select_opponent_sources:
          target: victim
          min: 1
          max: 1
          bind_as: picked
          prompt: Choose a source
          then:
            - trash_selected_sources: { source_refs: picked }
"#;

const SOURCE_RETURN: &str = r#"
card: SOURCE-RETURN
name: Source Return
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - select_opponent_sources:
          target: victim
          min: 1
          max: 1
          bind_as: picked
          prompt: Choose a source
          then:
            - return_selected_sources_to_hand: { source_refs: picked }
"#;

const RETURN_DECK: &str = r#"
card: RETURN-DECK
name: Return Deck
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - return_all_sources_to_deck: { target: victim, position: bottom }
"#;

const BOUNCE: &str = r#"
card: BOUNCE
name: Bounce
kind: tamer
color: [blue]
cost: 2
effects:
  - when: on_play
    process:
      - select_any_permanent: { bind_as: victim, filter: { name_is: Trash Guard }, prompt: Choose stack }
      - return_to_hand: { target: victim }
"#;

const HOST: &str = r#"
card: RESULT-HOST
name: Result Host
kind: digimon
level: 4
color: [black]
cost: 4
dp: 5000
traits: [ResultHost]
"#;

const EVOLVER: &str = r#"
card: EVOLVER
name: Evolver
kind: tamer
color: [black]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent: { bind_as: target, filter: { trait_has: ResultHost }, prompt: Choose host }
      - select_hand:
          of: you
          bind_as: result
          optional: true
          cost: false
          filter: { name_is: Evolved Result }
          prompt: Choose evolution
      - effect_initiated_digivolve: { target: target, from_hand: result, cost: free }
      - if:
          condition: { effect_digivolved_any_digimon: true }
          then: [{ gain_memory: 3 }]
          else: [{ gain_memory: 1 }]
"#;

const DOUBLE_EVOLVER: &str = r#"
card: EVOLVER
name: Double Evolver
kind: tamer
color: [black]
cost: 2
effects:
  - when: on_play
    process:
      - select_own_permanent: { bind_as: target, filter: { trait_has: ResultHost }, prompt: Choose host }
      - select_hand: { of: you, bind_as: first, filter: { name_is: Evolved Result }, prompt: Choose first evolution }
      - effect_initiated_digivolve: { target: target, from_hand: first, cost: free }
      - select_hand: { of: you, bind_as: second, filter: { name_is: Evolved Result }, prompt: Choose second evolution }
      - effect_initiated_digivolve: { target: target, from_hand: second, cost: free }
      - if:
          condition: { effect_digivolved_any_digimon: true }
          then: [{ gain_memory: 3 }]
"#;

const RESULT: &str = r#"
card: RESULT-A
name: Evolved Result
kind: digimon
level: 5
color: [black]
cost: 7
dp: 8000
alt_paths:
  - kind: digivolve
    from: { trait_has: ResultHost }
    cost: 3
"#;

const REPLACER: &str = r#"
card: RESULT-REPLACER
name: Result Replacer
kind: digimon
level: 4
color: [black]
cost: 4
dp: 5000
traits: [ResultHost]
effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      count_gte:
        filter:
          of: you
          zone: [hand, trash]
          name_is: Evolved Result
          can_digivolve_from_source: true
        n: 1
    process:
      - select_union_zone:
          of: you
          zones: [hand, trash]
          bind_as: result
          filter: { name_is: Evolved Result, can_digivolve_from_source: true }
          prompt: Choose replacement evolution
      - effect_initiated_digivolve: { target: replacement_subject, source: result, cost: free }
      - if:
          condition: { effect_digivolved_any_digimon: true }
          then: [{ cancel_replacement: {} }]
"#;

const DELETE: &str = r#"
card: DELETE
name: Delete
kind: tamer
color: [purple]
cost: 2
effects:
  - when: on_play
    process:
      - select_opponent_permanent: { bind_as: victim, filter: { kind: digimon }, prompt: Choose deletion target }
      - delete_permanent: { target: victim }
"#;

fn runner(yamls: &[&str]) -> DebugRunner {
    let mut b = DebugRunner::builder();
    for yaml in yamls {
        b = b.from_dsl_yaml(yaml).unwrap();
    }
    for (id, level) in [("LOW", 3), ("MID", 4)] {
        let mut card = make_test_card(id, id);
        card.level = Some(level);
        card.dp = Some(4000);
        b = b.add_card(card);
    }
    let mut r = b
        .deck(0, &["LOW"; 30])
        .deck(1, &["LOW"; 30])
        .memory(5)
        .start();
    r.skip_mulligan();
    r.game.enter_main_phase();
    r
}

fn fire(r: &mut DebugRunner, timing: EffectTiming, h: PermanentHandle) {
    r.game
        .enqueue_triggered(timing, TriggerSource::Permanent(h));
    r.game.drain_effect_queue();
}

/// Inspect the complete pending mask, including the other controller's mask.
fn choices(r: &DebugRunner, player: u8, count: usize, optional: bool) -> Vec<u16> {
    let pending = r
        .game
        .pending_selection
        .as_ref()
        .expect("real player choice");
    assert_eq!(pending.selecting_player, player);
    assert_eq!(pending.is_optional, optional);
    let ids: Vec<_> = pending
        .valid_action_ids
        .iter()
        .copied()
        .filter(|a| *a != PASS)
        .collect();
    assert_eq!(ids.len(), count, "{}: {:?}", pending.prompt, ids);
    let mask = build_action_mask(&r.game, player);
    for (a, v) in mask.iter().enumerate() {
        assert_eq!(
            *v,
            if ids.contains(&(a as u16)) || (optional && a == PASS as usize) {
                1.0
            } else {
                0.0
            },
            "mask at {a}"
        );
    }
    assert!(build_action_mask(&r.game, 1 - player)
        .iter()
        .all(|v| *v == 0.0));
    ids
}

fn pick(r: &mut DebugRunner, player: u8, count: usize, optional: bool, index: usize) {
    let ids = choices(r, player, count, optional);
    r.execute_action(player, ids[index]).unwrap();
}

fn stack_ids(r: &DebugRunner, h: PermanentHandle) -> Vec<String> {
    r.game.players[h.player as usize].battle_area[h.index as usize]
        .card_sources
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn guard_fixture(extra: &str, owner: u8) -> (DebugRunner, PermanentHandle) {
    let mut r = runner(&[GUARD, extra]);
    r.set_first_player(owner);
    let h = r.place_stack(owner, &["LOW", "MID", "TRASH-GUARD"]);
    r.place_stack(owner, &["LOW", "MID", "TRASH-GUARD"]);
    fire(&mut r, EffectTiming::OnPlay, h);
    (r, h)
}

fn attack(r: &mut DebugRunner, yaml: &str, actor: u8, target: usize) {
    let id = yaml.lines().find_map(|l| l.strip_prefix("card: ")).unwrap();
    let attacker = r.place_on_field(actor, id, None);
    fire(r, EffectTiming::OnPlay, attacker);
    pick(r, actor, 2, false, target);
}

#[test]
fn stack_trash_blocks_only_opponent_effects_on_the_granted_stack() {
    for (yaml, removed) in [(ALL_TRASH, 2), (TOP_SOURCE, 1), (TOP_STACK, 1), (DEDI, 1)] {
        for owner in [0, 1] {
            for (opponent, target) in [(true, 0), (false, 0), (true, 1)] {
                let (mut r, _) = guard_fixture(yaml, owner);
                let actor = if opponent { 1 - owner } else { owner };
                attack(&mut r, yaml, actor, target);
                let h = PermanentHandle {
                    player: owner,
                    index: target as u8,
                };
                let blocked = opponent && target == 0;
                assert_eq!(
                    stack_ids(&r, h).len(),
                    if blocked { 3 } else { 3 - removed },
                    "{yaml}, owner={owner}, opponent={opponent}, target={target}"
                );
                assert_eq!(
                    stack_ids(&r, h),
                    if blocked {
                        vec!["LOW", "MID", "TRASH-GUARD"]
                    } else if yaml == ALL_TRASH {
                        vec!["TRASH-GUARD"]
                    } else {
                        // All three top-trash verbs remove the visible top card.
                        vec!["LOW", "MID"]
                    }
                );
                assert_eq!(
                    r.game.players[owner as usize].trash.len(),
                    if blocked { 0 } else { removed }
                );
                assert!(r.game.pending_selection.is_none());
            }
        }
    }
}

#[test]
fn stack_trash_selected_source_has_two_choices_and_rejects_the_actual_move() {
    for target in [0, 1] {
        let (mut r, h) = guard_fixture(PICK_TRASH, 0);
        attack(&mut r, PICK_TRASH, 1, target);
        pick(&mut r, 1, 2, false, 1);
        let selected = PermanentHandle {
            player: 0,
            index: target as u8,
        };
        assert_eq!(
            stack_ids(&r, selected),
            if target == 0 {
                vec!["LOW", "MID", "TRASH-GUARD"]
            } else {
                vec!["LOW", "TRASH-GUARD"]
            }
        );
        assert_eq!(stack_ids(&r, h).len(), 3);
        assert_eq!(r.game.players[0].trash.len(), target);
    }
}

#[test]
fn stack_trash_does_not_block_own_source_payment_and_its_paid_tail() {
    let (mut r, h) = guard_fixture(ALL_TRASH, 0);
    let memory = r.memory();
    fire(&mut r, EffectTiming::WhenAttacking, h);
    pick(&mut r, 0, 2, false, 0);
    assert_eq!(stack_ids(&r, h), vec!["MID", "TRASH-GUARD"]);
    assert_eq!(r.game.players[0].trash[0].card_id(&r.game.card_data), "LOW");
    assert_eq!(r.memory(), memory + 2, "actual payment enables the tail");
}

#[test]
fn stack_trash_does_not_block_source_returns_or_whole_stack_return() {
    for yaml in [SOURCE_RETURN, RETURN_DECK, BOUNCE] {
        let (mut r, h) = guard_fixture(yaml, 0);
        let deck = r.game.players[0].deck.len();
        attack(&mut r, yaml, 1, 0);
        if yaml == SOURCE_RETURN {
            pick(&mut r, 1, 2, false, 0);
            assert_eq!(stack_ids(&r, h), vec!["MID", "TRASH-GUARD"]);
            assert_eq!(r.game.players[0].hand[0].card_id(&r.game.card_data), "LOW");
            assert!(r.game.players[0].trash.is_empty());
        } else if yaml == RETURN_DECK {
            assert_eq!(stack_ids(&r, h), vec!["TRASH-GUARD"]);
            assert_eq!(r.game.players[0].deck.len(), deck + 2);
            assert!(r.game.players[0].trash.is_empty());
        } else {
            assert_eq!(r.battle_area_size(0), 1);
            assert_eq!(
                r.game.players[0].hand[0].card_id(&r.game.card_data),
                "TRASH-GUARD"
            );
            assert_eq!(
                r.game.players[0].trash.len(),
                2,
                "leaving-stack cleanup remains legal"
            );
        }
    }
}

#[test]
fn stack_trash_expires_after_opponents_turn_for_both_controllers() {
    for owner in [0, 1] {
        let (mut r, h) = guard_fixture(TOP_SOURCE, owner);
        attack(&mut r, TOP_SOURCE, 1 - owner, 0);
        assert_eq!(stack_ids(&r, h).len(), 3);
        r.end_turn();
        attack(&mut r, TOP_SOURCE, 1 - owner, 0);
        assert_eq!(
            stack_ids(&r, h).len(),
            3,
            "still protected during opponent's turn"
        );
        r.end_turn();
        attack(&mut r, TOP_SOURCE, 1 - owner, 0);
        assert_eq!(
            stack_ids(&r, h),
            vec!["LOW", "MID"],
            "opponent-turn expiry owner={owner}"
        );
        assert_eq!(
            r.game.players[owner as usize].trash[0].card_id(&r.game.card_data),
            "TRASH-GUARD",
            "trash_top_source removes the visible top card after protection expires"
        );
    }
}

fn result_fixture() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    result_fixture_with_evolver(EVOLVER)
}

fn result_fixture_with_evolver(evolver: &str) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let second = RESULT.replace("RESULT-A", "RESULT-B");
    let mut r = runner(&[HOST, evolver, RESULT, &second]);
    let h = r.place_on_field(0, "RESULT-HOST", None);
    r.place_on_field(0, "RESULT-HOST", None);
    let actor = r.place_on_field(0, "EVOLVER", None);
    r.add_to_hand(0, "RESULT-A");
    r.add_to_hand(0, "RESULT-B");
    (r, h, actor)
}

#[test]
fn effect_digivolved_result_follows_either_selected_card_and_only_the_selected_host() {
    for index in [0, 1] {
        let (mut r, h, actor) = result_fixture();
        let memory = r.memory();
        fire(&mut r, EffectTiming::OnPlay, actor);
        pick(&mut r, 0, 2, false, 0);
        pick(&mut r, 0, 2, true, index);
        assert_eq!(
            stack_ids(&r, h),
            vec![
                "RESULT-HOST",
                if index == 0 { "RESULT-A" } else { "RESULT-B" }
            ]
        );
        assert_eq!(
            stack_ids(
                &r,
                PermanentHandle {
                    player: 0,
                    index: 1
                }
            ),
            vec!["RESULT-HOST"]
        );
        assert_eq!(r.memory(), memory + 3, "success-only branch");
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn effect_digivolved_result_is_false_when_declined_rejected_or_no_result_exists() {
    for mode in ["declined", "rejected", "empty"] {
        let (mut r, h, actor) = result_fixture();
        if mode == "empty" {
            r.game.players[0].hand.clear();
        }
        if mode == "rejected" {
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
            );
        }
        let memory = r.memory();
        fire(&mut r, EffectTiming::OnPlay, actor);
        pick(&mut r, 0, 2, false, 0);
        if mode == "declined" {
            choices(&r, 0, 2, true);
            r.execute_action(0, PASS).unwrap();
        } else if mode == "rejected" {
            pick(&mut r, 0, 2, true, 1);
        }
        assert_eq!(stack_ids(&r, h), vec!["RESULT-HOST"], "{mode}");
        assert_eq!(r.memory(), memory + 1, "failure branch for {mode}");
        assert_eq!(
            r.game.players[0].hand.len(),
            if mode == "empty" { 0 } else { 2 }
        );
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn effect_digivolved_result_is_false_if_selected_host_disappears_before_commit() {
    let (mut r, _, actor) = result_fixture();
    let memory = r.memory();
    fire(&mut r, EffectTiming::OnPlay, actor);
    pick(&mut r, 0, 2, false, 0);
    let picked = choices(&r, 0, 2, true)[0];
    // A parked choice must retain identity; the adjacent host now occupies index 0.
    let gone = r.game.players[0].battle_area.remove(0);
    r.game.players[0].trash.extend(gone.card_sources);
    let parked = r.game.clone();
    let mut digest = None;
    for cloned in [false, true] {
        if cloned {
            r.game = parked.clone();
        }
        r.execute_action(0, picked).unwrap();
        assert_eq!(
            stack_ids(
                &r,
                PermanentHandle {
                    player: 0,
                    index: 0
                }
            ),
            vec!["RESULT-HOST"]
        );
        assert_eq!(r.memory(), memory + 1);
        assert_eq!(r.game.players[0].hand.len(), 2);
        let actual = r.game.verification_digest();
        if let Some(expected) = digest {
            assert_eq!(actual, expected);
        }
        digest = Some(actual);
    }
}

#[test]
fn selected_host_follows_field_compaction_through_hand_choice_and_clone() {
    for selector in ["select_own_permanent", "select_any_permanent"] {
        for callback in [false, true] {
            let evolver = EVOLVER.replace("select_own_permanent", selector);
            let (mut r, _, actor) = result_fixture_with_evolver(&evolver);
            let memory = r.memory();
            fire(&mut r, EffectTiming::OnPlay, actor);
            if callback {
                // Exercise the legacy selection callback as well as the VM binder.
                r.game.pending_selection_resume = None;
            }
            pick(&mut r, 0, 2, false, 1);
            let picked = choices(&r, 0, 2, true)[0];
            // The selected second host survives, shifted into the removed first slot.
            let gone = r.game.players[0].battle_area.remove(0);
            r.game.players[0].trash.extend(gone.card_sources);
            let parked = r.game.clone();
            let mut digest = None;
            for cloned in [false, true] {
                if cloned {
                    r.game = parked.clone();
                }
                r.execute_action(0, picked).unwrap();
                assert_eq!(
                    stack_ids(
                        &r,
                        PermanentHandle {
                            player: 0,
                            index: 0
                        }
                    ),
                    vec!["RESULT-HOST", "RESULT-A"]
                );
                assert_eq!(
                    stack_ids(
                        &r,
                        PermanentHandle {
                            player: 0,
                            index: 1
                        }
                    ),
                    vec!["EVOLVER"]
                );
                assert_eq!(
                    r.memory(),
                    memory + 3,
                    "surviving selected host remains a successful result for {selector}"
                );
                let actual = r.game.verification_digest();
                if let Some(expected) = digest {
                    assert_eq!(actual, expected);
                }
                digest = Some(actual);
            }
        }
    }
}

#[test]
fn selected_host_identity_survives_its_first_digivolution_and_another_cloned_choice() {
    // Both result cards can be digivolved onto again through the same printed
    // fixture route. The selected permanent survives even though its top changes.
    let first = RESULT.replace("dp: 8000", "dp: 8000\ntraits: [ResultHost]");
    let second = first.replace("RESULT-A", "RESULT-B");
    let mut r = runner(&[HOST, DOUBLE_EVOLVER, &first, &second]);
    let h = r.place_on_field(0, "RESULT-HOST", None);
    r.place_on_field(0, "RESULT-HOST", None);
    let actor = r.place_on_field(0, "EVOLVER", None);
    r.add_to_hand(0, "RESULT-A");
    r.add_to_hand(0, "RESULT-B");
    let memory = r.memory();
    fire(&mut r, EffectTiming::OnPlay, actor);
    pick(&mut r, 0, 2, false, 0);
    pick(&mut r, 0, 2, false, 0);
    assert_eq!(stack_ids(&r, h), vec!["RESULT-HOST", "RESULT-A"]);
    let picked = choices(&r, 0, 1, false)[0];
    let parked = r.game.clone();
    let mut digest = None;
    for cloned in [false, true] {
        if cloned {
            r.game = parked.clone();
        }
        r.execute_action(0, picked).unwrap();
        assert_eq!(
            stack_ids(&r, h),
            vec!["RESULT-HOST", "RESULT-A", "RESULT-B"]
        );
        assert_eq!(
            stack_ids(
                &r,
                PermanentHandle {
                    player: 0,
                    index: 1
                }
            ),
            vec!["RESULT-HOST"]
        );
        assert_eq!(r.memory(), memory + 3);
        let actual = r.game.verification_digest();
        if let Some(expected) = digest {
            assert_eq!(actual, expected);
        }
        digest = Some(actual);
    }
}

#[test]
fn effect_digivolved_result_does_not_leak_from_an_earlier_effect() {
    let (mut r, _, actor) = result_fixture();
    fire(&mut r, EffectTiming::OnPlay, actor);
    pick(&mut r, 0, 2, false, 0);
    pick(&mut r, 0, 2, true, 0);
    let memory = r.memory();
    let h = PermanentHandle {
        player: 0,
        index: 1,
    };
    r.game.modifiers.add(
        h,
        ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
    );
    fire(&mut r, EffectTiming::OnPlay, actor);
    pick(&mut r, 0, 1, false, 0);
    pick(&mut r, 0, 1, true, 0);
    assert_eq!(stack_ids(&r, h), vec!["RESULT-HOST"]);
    assert_eq!(
        r.memory(),
        memory + 1,
        "earlier success cannot satisfy this effect's result"
    );
}

fn replacement_fixture() -> (DebugRunner, PermanentHandle) {
    let second = RESULT.replace("RESULT-A", "RESULT-B");
    let mut r = runner(&[REPLACER, HOST, RESULT, &second, DELETE]);
    let h = r.place_on_field(0, "RESULT-REPLACER", None);
    r.place_on_field(0, "RESULT-HOST", None);
    r.add_to_hand(0, "RESULT-A");
    r.add_to_hand(0, "RESULT-B");
    let trash_card = r.game.players[0].hand.pop().unwrap();
    r.game.players[0].trash.push(trash_card);
    (r, h)
}

fn start_deletion(r: &mut DebugRunner) {
    let killer = r.place_on_field(1, "DELETE", None);
    fire(r, EffectTiming::OnPlay, killer);
    pick(r, 1, 2, false, 0);
}

#[test]
fn replacement_is_canceled_only_after_successful_hand_or_trash_digivolution() {
    for index in [0, 1] {
        let (mut r, h) = replacement_fixture();
        let memory = r.memory();
        start_deletion(&mut r);
        pick(&mut r, 0, 1, true, 0);
        pick(&mut r, 0, 2, false, index);
        assert_eq!(
            stack_ids(&r, h),
            vec![
                "RESULT-REPLACER",
                if index == 0 { "RESULT-A" } else { "RESULT-B" }
            ]
        );
        assert_eq!(r.battle_area_size(0), 2);
        assert_eq!(r.memory(), memory, "free digivolution");
        assert!(!r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "RESULT-REPLACER"));
        assert!(r.game.pending_selection.is_none());
    }
}

#[test]
fn replacement_decline_rejected_commit_or_absent_candidate_does_not_prevent_leave() {
    for mode in ["declined", "rejected", "empty"] {
        let (mut r, h) = replacement_fixture();
        if mode == "empty" {
            r.game.players[0].hand.clear();
            r.game.players[0].trash.clear();
        }
        let memory = r.memory();
        start_deletion(&mut r);
        if mode == "declined" {
            choices(&r, 0, 1, true);
            r.execute_action(0, PASS).unwrap();
        } else if mode == "rejected" {
            pick(&mut r, 0, 1, true, 0);
            let selected = choices(&r, 0, 2, false)[0];
            r.game.modifiers.add(
                h,
                ModifierEntry::simple(ModifierType::CannotDigivolve, 0, Expiry::Permanent, 0),
            );
            r.execute_action(0, selected).unwrap();
        }
        assert_eq!(r.battle_area_size(0), 1, "{mode} must not cancel deletion");
        assert_eq!(
            stack_ids(
                &r,
                PermanentHandle {
                    player: 0,
                    index: 0
                }
            ),
            vec!["RESULT-HOST"]
        );
        assert!(r.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&r.game.card_data) == "RESULT-REPLACER"));
        assert_eq!(r.memory(), memory);
        assert!(r.game.pending_selection.is_none());
    }
}
