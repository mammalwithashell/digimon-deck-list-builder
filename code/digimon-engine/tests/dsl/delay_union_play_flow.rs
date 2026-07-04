//! Engine-level contract tests for `DelaySelfTrashPlayFromUnionFlow`
//! (lower_replacement.rs) — the Delay-Option leave-replacement whose body is
//! `delete_permanent { target: source }` → `select_union_zone` →
//! `play_union_bound_free`.
//!
//! Gap: G-ENGINE-DELAY-SELF-TRASH-THEN-FREE-PLAY-IN-LEAVE-REPLACEMENT
//! (docs/RUST_ENGINE_GAPS.md, RESOLVED 2026-07-04). Before the recognised
//! flow existed, this shape fell into the generic replacement step runner:
//! the accepted replacement stayed parked while the reward selection was
//! pending, and after the selection chain completed the parked Proceed commit
//! deleted the subject by its STALE battle-area index — which by then pointed
//! at the freshly played reward card (the self-trash shifted the subject down,
//! the play pushed the reward into the subject's old slot). Net effect: the
//! reward card went to the trash and the leaving subject survived on the
//! field. The flow instead marks the replacement `handled()`, pays the
//! self-trash, finishes the OWNED leave by stable card handle, then runs the
//! reward steps with `replacement_subject` re-bound as the subject's stable
//! top-card snapshot (so `play_cost_eq_binding` reads the pre-leave printed
//! cost — the DCGO BT19_099.cs:261 hashtable-snapshot comparator).
//!
//! Reference card: BT19-099 (tests/cards_behavioral/bt19/bt19_099.rs);
//! hand-only sibling: ST20-14 (`DelaySelfTrashPlayFromHandFlow`).

use digimon_engine::action::space::{PASS, PLAY_HAND_START, REPLACEMENT_ACCEPT};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

/// Synthetic Delay-Option with the recognised union-flow leave replacement:
/// when an own [Watched]-name Digimon would leave, <Delay> — trash self, then
/// optionally play 1 [Reward]-trait Digimon with play cost = subject's + 1
/// from hand or trash, free.
const FIXTURE_YAML: &str = r#"
card: F-UNION-DELAY
name: "Fixture Union Delay"
kind: option
color: [purple]
cost: 2
traits: []

effects:
  - kind: replacement
    trigger: when_would_leave_battle_area
    optional: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - kind: digimon
        - name_contains: "Watched"
    summary: "own Watched would leave: <Delay> trash self, may play 1 [Reward] at cost+1 from hand/trash free"
    process:
      - delete_permanent: { target: source }
      - select_union_zone:
          of: you
          zones: [hand, trash]
          optional: true
          bind_as: reward_pick
          filter:
            all_of:
              - kind: digimon
              - trait_has: Reward
              - play_cost_eq_binding:
                  binding: replacement_subject
                  offset: 1
                  op: eq
          prompt: "Play 1 [Reward] Digimon with cost = leaving Digimon's cost + 1 (free)"
      - play_union_bound_free:
          binding: reward_pick
"#;

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn watched_digimon(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, "Watched One");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c
}

fn reward_digimon(id: &str, play_cost: u16) -> CardData {
    let mut c = make_test_card(id, "Reward One");
    c.card_kind = CardKind::Digimon;
    c.level = Some(6);
    c.dp = Some(11000);
    c.play_cost = play_cost;
    c.colors = vec![CardColor::Purple];
    c.traits = vec!["Reward".to_string()];
    c
}

fn seat_as_delay_option(
    runner: &mut DebugRunner,
    handle: digimon_engine::permanent::PermanentHandle,
) {
    use digimon_engine::permanent::OptionState;
    let turn = runner.game.turn_count;
    let perm = &mut runner.game.players[handle.player as usize].battle_area[handle.index as usize];
    perm.option_state = OptionState::Delayed {
        owner: handle.player,
        trash_on_turn: turn + 2,
        trigger: digimon_engine::enums::DelayTrigger::EndOfYourNextTurn,
        placed_on_turn: turn,
    };
}

/// The full-flow contract: accept the <Delay>, the option self-trashes, the
/// subject completes its leave (it is NOT prevented), the reward pick offers
/// the exact cost+1 candidate, and resolving it plays the reward ONTO THE
/// BATTLE AREA — not into the trash (the historical failure mode).
#[test]
fn delay_union_flow_plays_reward_on_field_and_subject_completes_leave() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FIXTURE_YAML)
        .expect("fixture YAML parses")
        .add_card(watched_digimon("F-SUBJ", 10))
        .add_card(reward_digimon("F-REWARD", 11))
        .add_card(filler("FILL"))
        .memory(10)
        .hand(0, &["F-REWARD"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let option = runner.place_on_field(0, "F-UNION-DELAY", Some(0));
    seat_as_delay_option(&mut runner, option);
    let subject = runner.place_on_field(0, "F-SUBJ", None);
    let subject_card = runner.top_card(subject);

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("optional replacement accept prompt installs");
    assert_eq!(pending.kind, SelectionKind::Replacement);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");

    // Cost paid + owned leave finished BEFORE the reward pick.
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "F-UNION-DELAY"),
        "the option self-trashes to pay the <Delay> cost"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == subject_card),
        "the leaving subject completes its leave to the trash (not prevented)"
    );

    // The reward pick surfaces as the ordinary union-zone selection.
    let sel = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the reward union-zone pick installs");
    assert_eq!(
        sel.kind,
        SelectionKind::UnionZone {
            zones: UnionZoneSet::HAND | UnionZoneSet::TRASH
        }
    );
    assert!(sel.is_optional);
    let action = sel
        .valid_action_ids
        .iter()
        .copied()
        .find(|&a| a != PASS)
        .expect("the cost+1 reward candidate is offered");
    assert_eq!(action, PLAY_HAND_START, "hand index 0 candidate");

    let player = sel.selecting_player;
    runner
        .game
        .resolve_selection(player, action)
        .expect("pick the reward");
    runner.game.drain_effect_queue();

    let reward_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_id(&runner.game.card_data) == "F-REWARD");
    assert!(
        reward_on_field,
        "the reward card must be PLAYED onto the battle area — the historical \
         defect trashed it via the parked replacement's stale-index Proceed commit"
    );
    assert!(
        !runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "F-REWARD"),
        "the reward card must not end in the trash"
    );
    let subject_on_field = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.card_sources.iter().any(|c| c.handle() == subject_card));
    assert!(
        !subject_on_field,
        "the subject must not survive on the field — the historical defect left \
         it alive after the stale-index commit hit the reward card instead"
    );
}

/// Snapshot comparator: the `play_cost_eq_binding { binding:
/// replacement_subject }` filter reads the subject's PRE-LEAVE printed cost
/// (the subject is already in the trash when the pick installs). A candidate
/// at the subject's own cost (not +1) is rejected; the whiff still pays the
/// cost and the leave still completes.
#[test]
fn delay_union_flow_comparator_uses_pre_leave_subject_cost_snapshot() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FIXTURE_YAML)
        .expect("fixture YAML parses")
        .add_card(watched_digimon("F-SUBJ2", 10))
        .add_card(reward_digimon("F-REWARD-EQ", 10)) // == subject cost, NOT +1
        .add_card(filler("FILL"))
        .memory(10)
        .hand(0, &["F-REWARD-EQ"])
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let option = runner.place_on_field(0, "F-UNION-DELAY", Some(0));
    seat_as_delay_option(&mut runner, option);
    let subject = runner.place_on_field(0, "F-SUBJ2", None);
    let subject_card = runner.top_card(subject);

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);
    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "the equal-cost candidate fails the +1 comparator — no reward prompt"
    );
    assert!(
        runner.game.players[0]
            .hand
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "F-REWARD-EQ"),
        "the rejected candidate stays in hand"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "F-UNION-DELAY"),
        "the <Delay> cost was still paid (DCGO trashes before the candidate scan)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == subject_card),
        "the subject still completes its leave"
    );
}

/// Offer gating matches the DCGO reference (BT19_099.cs CanUseCondition,
/// lines 205-219): the accept prompt installs even with ZERO eligible reward
/// candidates anywhere — the candidate scan is post-self-trash (lines
/// 275-277). Accepting trashes the option, plays nothing, and the subject
/// still leaves.
#[test]
fn delay_union_flow_offer_installs_with_zero_candidates() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(FIXTURE_YAML)
        .expect("fixture YAML parses")
        .add_card(watched_digimon("F-SUBJ3", 10))
        .add_card(filler("FILL"))
        .memory(10)
        .deck(0, &["FILL"; 5])
        .deck(1, &["FILL"; 5])
        .start();

    let option = runner.place_on_field(0, "F-UNION-DELAY", Some(0));
    seat_as_delay_option(&mut runner, option);
    let subject = runner.place_on_field(0, "F-SUBJ3", None);
    let subject_card = runner.top_card(subject);

    runner
        .game
        .delete_permanent_with_cause(subject, ReplacementCause::OwnEffect);

    let pending = runner
        .game
        .pending_selection
        .as_ref()
        .expect("the accept prompt installs even with zero reward candidates");
    assert_eq!(pending.kind, SelectionKind::Replacement);

    runner
        .game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept the <Delay> response");
    runner.game.drain_effect_queue();

    assert!(
        runner.game.pending_selection.is_none(),
        "zero candidates: no reward prompt (post-trash whiff)"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.card_id(&runner.game.card_data) == "F-UNION-DELAY"),
        "the option still self-trashed"
    );
    assert!(
        runner.game.players[0]
            .trash
            .iter()
            .any(|c| c.handle() == subject_card),
        "the subject still completes its leave"
    );
}
