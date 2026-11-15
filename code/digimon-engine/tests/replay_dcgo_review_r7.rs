//! DCGO replay: round-6 review findings (correctness + faithfulness review of
//! round 6), fixed in round 7. The in-module tests of `runners/replay.rs`
//! cover the typed footprint / identity-reference findings (2, 3); this file
//! holds the ones that need a staged session.
//!
//! 1. `MultipleSkills` -1 against a FORCED order whose only effect SCHEDULES a
//!    delayed effect (`EffectContext::schedule_delayed`): the round-6 footprint
//!    never read `Game::scheduled_effects`, so the -1 was accepted as "the run
//!    left nothing behind".
//! 4. The forced-count halt said "met no prompt" on the path that also runs
//!    with a prompt parked.
//! 7. A copy resolved through a `card_index` the converter DERIVED
//!    (`card_index_source` `frame_model` / `hand_listing`) is said so in a
//!    replay note; a recorder-sourced index stays silent.
//!
//! Staged with `DebugRunner` and synthetic YAML fixtures. No recording content.

use std::collections::HashMap;

use digimon_engine::action::space::{encode_digivolve, PLAY_HAND_START};
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
    /// DCGO card index of each action row's played card, by row.
    action_indexes: Vec<Option<i32>>,
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
    fn dcgo_action_card_index(&self, row: u32) -> Option<i32> {
        self.action_indexes.get(row as usize).copied().flatten()
    }
}

fn spec(action_id: u16, card_id: Option<&str>, selection: Option<digimon_engine::dcgo_recording::SelectionRow>) -> StepSpec {
    StepSpec {
        actor: selection.as_ref().map_or(0, |s| s.actor),
        action_id,
        phase: "Main".into(),
        source: if selection.is_some() { "selection" } else { "main_phase" }.into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection,
        board_p0: None,
        board_p1: None,
        card_id: card_id.map(str::to_string),
        dna_materials: None,
        inferred: false,
    }
}

fn sel(json: serde_json::Value) -> StepSpec {
    let mut v = serde_json::json!({ "step": 0, "actor": 0, "phase": "Main" });
    for (k, val) in json.as_object().unwrap() {
        v[k] = val.clone();
    }
    let row: digimon_engine::dcgo_recording::SelectionRow = serde_json::from_value(v).expect("selection row");
    spec(0, None, Some(row))
}

fn staged(game: Game, steps: Vec<StepSpec>) -> ReplaySession {
    let n = steps.len();
    ReplaySession::with_source(Box::new(StagedDcgo { game, steps, action_indexes: vec![None; n] }), &HashMap::new(), false)
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

// ── 1. -1 against a forced order whose only effect schedules a delayed effect ─

const SCHEDULE_DELAYED: &str = r#"
card: TEST-SCHEDULE
name: Test Schedule
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] At the end of your turn, lose 1 memory"
    process:
      - schedule_delayed:
          when: end_of_your_turn
          body:
            - lose_memory: 1
"#;

/// P0's main phase with TWO queued copies of the same mandatory [Start of
/// Your Main Phase] trigger of one Tamer (the forced order prompt is parked;
/// picking either entry leaves the same game).
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

/// The forced trigger's only effect is a delayed effect: no zone, memory,
/// permanent, modifier or granted effect changes, yet the engine's game now
/// holds an effect DCGO's skip never scheduled.
#[test]
fn decline_all_against_a_forced_trigger_that_only_schedules_a_delayed_effect_halts() {
    let game = twin_forced_order(SCHEDULE_DELAYED, "TEST-SCHEDULE");
    assert!(game.scheduled_effects.is_empty());
    let memory = game.memory;
    let mut s = staged(game, vec![sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1}))]);
    s.step();
    let reason = halt_reason(&s, 0);
    assert!(reason.contains("MultipleSkills -1") && reason.contains("scheduled"), "{reason}");
    assert_eq!(s.game.memory, memory, "nothing but the scheduled entry differs");
}

// ── 4. the forced-count halt names what the engine did ───────────────────────

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

/// With the engine's own hand pick parked, the halt must not say the count
/// "met no prompt": a prompt IS parked (it is just not this count's).
#[test]
fn forced_count_halt_with_a_prompt_parked_names_the_parked_prompt() {
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
    assert!(!reason.contains("met no prompt"), "a prompt is parked: {reason}");
    assert!(reason.contains("is parked") && reason.contains("Trash 1 card in your hand"), "{reason}");
}

/// Guard (green on the old code): with nothing parked the halt still says so.
#[test]
fn forced_count_halt_with_no_prompt_still_says_it_met_no_prompt() {
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
    assert!(s.game.pending_selection.is_none(), "nothing is parked");
    s.step();
    let reason = halt_reason(&s, 1);
    assert!(reason.contains("met no prompt"), "{reason}");
}

// ── 7. a copy named by a derived card index says so ──────────────────────────

fn tamer(id: &str, name: &str) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::White];
    c
}

fn host() -> CardData {
    let mut c = make_test_card("HOST", "ShineGreymon");
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 12;
    c.traits = vec!["DATA SQUAD".into()];
    c
}

/// As `replay_dcgo_burst_identity.rs`: P0's main phase, 8 memory; battle area
/// [MARCUS (an older copy with a source under it), HOST]; hand [MARCUS,
/// BT25-104]. The rows play the hand MARCUS, burst BT25-104 onto HOST and pick
/// the Tamer to return, naming the played copy by DCGO index `PLAYED`.
fn burst_board() -> Game {
    let mut r = DebugRunner::builder()
        .dsl_card("BT25-104")
        .expect("BT25-104 production YAML")
        .add_card(host())
        .add_card(tamer("MARCUS", "Marcus Damon"))
        .add_card(tamer("OTHER", "Some Other Tamer"))
        .deck(0, &["OTHER"; 10])
        .deck(1, &["OTHER"; 10])
        .security(0, &["OTHER"; 5])
        .security(1, &["OTHER"; 5])
        .memory(8)
        .start();
    r.set_first_player(0);
    let old = r.place_on_field(0, "MARCUS", Some(0));
    r.push_source(old, "OTHER");
    r.place_on_field(0, "HOST", Some(0));
    r.add_to_hand(0, "MARCUS");
    r.add_to_hand(0, "BT25-104");
    r.game.enter_main_phase();
    r.game
}

const PLAYED: i32 = 87;

/// The burst pick naming the played copy by `PLAYED`, with `source` as the
/// target's `card_index_source` (`None` = the field absent).
fn burst_session(source: Option<&str>) -> ReplaySession {
    let game = burst_board();
    let hand = |id: &str| {
        game.players[0].hand.iter().position(|c| c.card_id(&game.card_data) == id).expect("in hand") as u16
    };
    let host_slot = game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&game.card_data) == "HOST")
        .expect("host") as u16;
    let mut target = serde_json::json!({"player": 0, "frame": 1, "card_index": PLAYED});
    if let Some(s) = source {
        target["card_index_source"] = serde_json::json!(s);
    }
    let pick = serde_json::from_value(serde_json::json!({
        "step": 2, "actor": 0, "phase": "Main", "prompt": "SelectPermanentEffect", "mechanic": "burst",
        "targets": [target], "board_p0": ["HOST", "MARCUS", "MARCUS"], "board_p1": [],
    }))
    .expect("selection row");
    let steps = vec![
        spec(PLAY_HAND_START + hand("MARCUS"), Some("MARCUS"), None),
        spec(encode_digivolve(0, host_slot), Some("BT25-104"), None),
        spec(0, None, Some(pick)),
    ];
    let mut s = ReplaySession::with_source(
        Box::new(StagedDcgo { game, steps, action_indexes: vec![Some(PLAYED), None, None] }),
        &HashMap::new(),
        false,
    )
    .expect("staged session");
    for _ in 0..3 {
        if s.is_complete() || s.is_paused() {
            break;
        }
        s.step();
    }
    assert!(s.divergences().is_empty() && s.is_complete(), "every row replays: {:?}", s.divergences());
    s
}

fn provenance_notes(s: &ReplaySession) -> Vec<String> {
    s.replay_notes().iter().filter(|n| n.contains("copy identity")).cloned().collect()
}

#[test]
fn a_copy_named_by_a_frame_model_index_says_it_was_inferred() {
    let s = burst_session(Some("frame_model"));
    let notes = provenance_notes(&s);
    assert!(
        notes.iter().any(|n| n.contains("inferred by frame model, not recorded") && n.contains(&PLAYED.to_string())),
        "{:?}",
        s.replay_notes()
    );
}

#[test]
fn a_copy_named_by_a_hand_listing_index_says_it_was_inferred() {
    let s = burst_session(Some("hand_listing"));
    assert!(
        provenance_notes(&s).iter().any(|n| n.contains("inferred by hand listing, not recorded")),
        "{:?}",
        s.replay_notes()
    );
}

/// Guard (green on the old code): a recorder-sourced index is DCGO's own
/// value and stays silent.
#[test]
fn a_copy_named_by_a_recorder_index_stays_silent() {
    let s = burst_session(Some("recorder"));
    assert!(provenance_notes(&s).is_empty(), "{:?}", s.replay_notes());
}

/// A recording predating `card_index_source` carries an index of unknown
/// provenance: said so rather than taken for an observation.
#[test]
fn a_copy_named_by_an_index_of_unrecorded_provenance_says_so() {
    let s = burst_session(None);
    assert!(
        provenance_notes(&s).iter().any(|n| n.contains("provenance not recorded")),
        "{:?}",
        s.replay_notes()
    );
}
