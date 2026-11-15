//! DCGO replay: round-5 review findings (correctness + faithfulness review of
//! round 5), fixed in round 6.
//!
//! 1. `MultipleSkills` -1 against a FORCED order. DCGO's -1 skipped every
//!    stacked effect (MultipleSkills.cs:474-480), a non-optional one included
//!    when it is skippable (ICardEffect.cs:364-367). The engine has to RUN a
//!    forced trigger, so the -1 stands only when that run provably left
//!    nothing behind. The round-5 judgement read the owner's effect events and
//!    a footprint of players / memory / modifiers; it missed a forced trigger
//!    that parks ANOTHER player's prompt, one that only arms a digivolution
//!    cost reducer, and one that only registers a granted effect.
//! 2. The -1 fold had no actor check: P1's -1 met P0's parked order.
//! 4. A forced single-value `SelectCountEffect` (<De-Digivolve N>) is
//!    consumed only when the engine's window shows the targeted permanent
//!    lost exactly its top N cards and KEPT its place on the field - the
//!    same evidence whether a prompt is parked or not. A deletion of an
//!    N-card Digimon is not a De-Digivolve N.
//!
//! Staged with `DebugRunner` and synthetic YAML fixtures. No recording content.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{SelectionKind, TriggerSource};

fn digimon(id: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c
}

struct StagedDcgo {
    game: Game,
    steps: Vec<StepSpec>,
}

impl RecordingSource for StagedDcgo {
    fn build_initial_game(&self, _: &HashMap<String, CardData>) -> Result<Game, ReplayError> {
        Ok(self.game.clone())
    }
    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        *game = self.game.clone();
        Ok(())
    }
    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }
    fn default_policy(&self) -> StepPolicy {
        StepPolicy::CheckThenApply
    }
    fn omits_dcgo_silent_prompts(&self) -> bool {
        true
    }
}

fn sel(json: serde_json::Value) -> StepSpec {
    let mut v = serde_json::json!({ "step": 0, "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    let row: digimon_engine::dcgo_recording::SelectionRow = serde_json::from_value(v).expect("selection row");
    StepSpec {
        actor: row.actor,
        action_id: 0,
        phase: "Main".into(),
        source: "selection".into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection: Some(row),
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(s.divergences().is_empty() && !s.is_paused(), "row {before} diverged: {:?}", s.divergences());
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

fn halt_reason(s: &ReplaySession, at: u32) -> String {
    assert!(s.is_paused(), "the replay stops; notes {:?}", s.replay_notes());
    assert_eq!(s.current_step(), at);
    match s.divergences() {
        [d] => match &d.kind {
            DivergenceKind::SelectionResolution { reason } => reason.clone(),
            other => panic!("expected SelectionResolution, got {other:?}"),
        },
        other => panic!("expected one divergence, got {other:?}"),
    }
}

// ── 1. -1 against a forced order whose run leaves something behind ───────────

const OPP_TRASH: &str = r#"
card: TEST-OPP-TRASH
name: Test Opponent Trash
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Your opponent trashes 1 card in their hand"
    process:
      - as_selecting_player:
          of: opponent
          body:
            - select_hand:
                of: opponent
                bind_as: discarded
                filter: {}
                prompt: "Trash 1 card in your hand"
            - trash_from_hand_by_index: { of: opponent, hand_index: discarded }
"#;

const ARM_REDUCER: &str = r#"
card: TEST-ARM-REDUCER
name: Test Arm Reducer
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] The next time one of your Digimon would digivolve this turn, reduce the cost by 1"
    process:
      - arm_digivolve_cost_reducer:
          amount: 1
          single_fire: true
"#;

const GRANT_BODY: &str = r#"
card: TEST-GRANT-BODY
name: Test Grant Body
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Your opponent's Digimon gain '[When Attacking] lose 2 memory' until the end of their next turn"
    process:
      - grant_triggered_effect:
          dynamic: true
          target:
            of: opponent
            zone: [battle_area]
            kind: digimon
          timing: when_attacking
          expiry: end_of_opponents_next_turn
          body:
            - lose_memory: 2
"#;

/// P0's main phase with TWO queued copies of the same mandatory [Start of
/// Your Main Phase] trigger of one Tamer: the forced order prompt is parked,
/// and picking either entry leaves the same game (so the replay's order
/// exploration converges, and only what the forced run LEFT decides).
fn twin_forced_order(yaml: &str, card: &str) -> Game {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("fixture compiles")
        .add_card(digimon("FILL", 3))
        .hand(0, &["FILL"])
        .hand(1, &["FILL", "FILL"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(1, "FILL", Some(0));
    let t = r.place_on_field(0, card, Some(0));
    for _ in 0..2 {
        r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(t));
    }
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the order is parked");
    assert!(
        matches!(p.kind, SelectionKind::TriggerOrder) && !p.is_optional && p.valid_action_ids.len() == 2,
        "{:?} optional {} valid {:?}",
        p.kind,
        p.is_optional,
        p.valid_action_ids
    );
    r.game
}

/// The forced trigger parks P1's "trash 1 card" pick: the engine is still
/// running an effect DCGO skipped, so the -1 may not stand. (The round-5
/// review expected this to pass; the replay already stops here because the
/// prompt is a stack card's own - this pins that, and the owner-aware
/// loop keeps it true for prompts of cards outside the stack, which the
/// in-module `decline_stack_after_a_forced_run_errors_on_a_prompt_of_a_card_outside_the_stack`
/// test covers.)
#[test]
fn decline_all_against_a_forced_trigger_that_parks_the_opponents_prompt_halts() {
    let game = twin_forced_order(OPP_TRASH, "TEST-OPP-TRASH");
    let mut s = staged(game, vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1}))]);
    s.step();
    let reason = halt_reason(&s, 0);
    assert!(reason.contains("MultipleSkills -1") && reason.contains("P1"), "{reason}");
}

/// The forced trigger's only effect is a digivolution cost reducer on the
/// player: no zone, memory, permanent or modifier changes, yet the engine's
/// game now differs from the one DCGO's -1 left.
#[test]
fn decline_all_against_a_forced_trigger_that_only_arms_a_cost_reducer_halts() {
    let game = twin_forced_order(ARM_REDUCER, "TEST-ARM-REDUCER");
    assert!(game.player_digivolve_cost_reducers.is_empty());
    let mut s = staged(game, vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1}))]);
    s.step();
    let reason = halt_reason(&s, 0);
    assert!(reason.contains("MultipleSkills -1") && reason.contains("cost reducer"), "{reason}");
}

/// The forced trigger's only effect is a granted triggered effect. (Already
/// caught before round 6: the grant is a modifier entry. Kept as a guard now
/// that the footprint also reads the granted-effect registry directly.)
#[test]
fn decline_all_against_a_forced_trigger_that_only_grants_an_effect_halts() {
    let game = twin_forced_order(GRANT_BODY, "TEST-GRANT-BODY");
    let mut s = staged(game, vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1}))]);
    s.step();
    let reason = halt_reason(&s, 0);
    assert!(reason.contains("MultipleSkills -1"), "{reason}");
}

// ── 2. the -1 fold needs the actor's own order ───────────────────────────────

/// P1's -1 against P0's parked order is not an answer to it.
#[test]
fn decline_all_of_the_other_player_is_an_actor_divergence() {
    let game = twin_forced_order(ARM_REDUCER, "TEST-ARM-REDUCER");
    let mut s = staged(game, vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1, "actor": 1}))]);
    s.step();
    assert!(s.is_paused(), "the replay stops; notes {:?}", s.replay_notes());
    match s.divergences() {
        [d] => assert!(
            matches!(d.kind, DivergenceKind::Actor { expected: 0, recorded: 1 }),
            "expected an actor divergence, got {:?}",
            d.kind
        ),
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(s.game.pending_selection.is_some(), "P0's order is still parked");
}

// ── 4. a forced count needs the same evidence parked or not ──────────────────

const DELETE_TARGET: &str = r#"
card: TEST-DELETE
name: Test Delete
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Delete 1 of your opponent's Digimon"
    process:
      - select_opponent_permanent:
          bind_as: victim
          filter: { kind: digimon }
          prompt: "Delete 1 of your opponent's Digimon"
      - delete_permanent: { target: victim }
"#;

/// <De-Digivolve 1> on one of the opponent's Digimon, then the player's own
/// hand pick (so a prompt is parked when DCGO's forced count arrives).
const DEDIGIVOLVE_THEN_PICK: &str = r#"
card: TEST-DEDIGI-PICK
name: Test De-Digivolve Then Pick
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] <De-Digivolve 1> 1 of your opponent's Digimon, then trash 1 card in your hand"
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          prompt: "Select 1 of your opponent's Digimon to De-Digivolve 1"
      - de_digivolve:
          target: tgt
          amount: 1
      - select_hand:
          of: you
          bind_as: mine
          filter: {}
          prompt: "Trash 1 card in your hand"
      - trash_from_hand_by_index: { of: you, hand_index: mine }
"#;

/// P0's main phase with the tamer's trigger resolving: its target pick over
/// P1's TOP (Lv.4 on a Lv.3 SRC, two cards) and OTHER (one card) is parked.
fn target_pick_parked(yaml: &str, card: &str) -> Game {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("fixture compiles")
        .add_card(digimon("SRC", 3))
        .add_card(digimon("TOP", 4))
        .add_card(digimon("OTHER", 4))
        .hand(0, &["SRC", "SRC"])
        .deck(0, &["SRC"; 8])
        .deck(1, &["SRC"; 8])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_stack(1, &["SRC", "TOP"]);
    r.place_on_field(1, "OTHER", Some(0));
    let tamer = r.place_on_field(0, card, Some(0));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(tamer));
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the target pick is parked");
    assert_eq!(p.selecting_player, 0, "{}", p.prompt);
    r.game
}

/// DCGO asked the target and then a forced count of 2 (its candidate set is
/// {2}). The engine DELETED the 2-card Digimon: both cards went to the trash,
/// but the Digimon is gone - a <De-Digivolve 2> leaves it on the field.
#[test]
fn forced_count_of_n_is_not_a_deletion_of_an_n_card_digimon() {
    let mut s = staged(
        target_pick_parked(DELETE_TARGET, "TEST-DELETE"),
        vec![
            sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["TOP", "OTHER"],
                "targets": [{"player": 1, "frame": 0}]})),
            sel(serde_json::json!({"prompt": "SelectCountEffect", "count": 2, "candidates": [2]})),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    s.step();
    let reason = halt_reason(&s, 1);
    assert!(reason.contains("SelectCountEffect") && reason.contains("forced"), "{reason}");
}

/// With the engine's own next prompt parked (P0's hand pick), the forced
/// count still needs the engine to have resolved the effect: here the target
/// was OTHER (one card, nothing to trash), so nothing left any permanent.
#[test]
fn forced_count_with_a_prompt_parked_and_nothing_resolved_halts() {
    let mut s = staged(
        target_pick_parked(DEDIGIVOLVE_THEN_PICK, "TEST-DEDIGI-PICK"),
        vec![
            sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["TOP", "OTHER"],
                "targets": [{"player": 1, "frame": 1}]})),
            sel(serde_json::json!({"prompt": "SelectCountEffect", "count": 1, "candidates": [1]})),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.game.pending_selection.is_some(), "the own-hand pick is parked");
    s.step();
    let reason = halt_reason(&s, 1);
    assert!(reason.contains("SelectCountEffect") && reason.contains("forced"), "{reason}");
}

/// ... and with the same prompt parked, the engine DID de-digivolve TOP: the
/// count is consumed with a note naming that resolution, as it is with no
/// prompt parked.
#[test]
fn forced_count_with_a_prompt_parked_after_the_de_digivolve_is_a_note() {
    let mut s = staged(
        target_pick_parked(DEDIGIVOLVE_THEN_PICK, "TEST-DEDIGI-PICK"),
        vec![
            sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["TOP", "OTHER"],
                "targets": [{"player": 1, "frame": 0}]})),
            sel(serde_json::json!({"prompt": "SelectCountEffect", "count": 1, "candidates": [1]})),
        ],
    );
    replay_all(&mut s);
    assert!(s.game.pending_selection.is_some(), "the own-hand pick is still parked");
    assert!(
        s.replay_notes().iter().any(|n| n.contains("SelectCountEffect") && n.contains("forced") && n.contains("TOP")),
        "{:?}",
        s.replay_notes()
    );
}
