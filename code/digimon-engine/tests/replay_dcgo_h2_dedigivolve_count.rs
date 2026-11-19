//! DCGO replay, hunt round 2 (cluster `dedigivolve-count`): the count prompts of
//! a formula `<De-Digivolve 1> for each ...` and of a declared formula
//! `<De-Digivolve N>`.
//!
//! DCGO's `IDegeneration.Degeneration` (CardController.cs:5281-5297) opens a
//! `SelectCountEffect` with `MaxCount = min(digivolution cards, N)` and
//! `CanNoSelect: false` for EVERY instance. Two shapes follow:
//!
//! 1. **Declared** (EX13-066 Option: `EX13_066.cs` runs ONE `<De-Digivolve N>`
//!    with N = the Digimon count): the engine parks the count declaration
//!    (CR 4.3 16-12-1), so DCGO's count row answers it (`declared: true`).
//! 2. **Per-instance** (BT19-073, BT21-060, BT24-041, BT25-075: N separate
//!    `<De-Digivolve 1>` instances, CR 4.3 16-12-7 / 16-12-8): each instance
//!    opens a count whose only value is 1, so DCGO records a RUN of
//!    `SelectCountEffect 1` rows. The engine asks none of them; recorder 0.2.0
//!    writes no candidate set, so a row is consumed only against the engine's
//!    own window: a target was chosen, the top cards of one permanent went to
//!    the trash and the run does not exceed the instances the engine ran.
//!
//! Staged with `DebugRunner` and synthetic YAML fixtures. No recording content.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::TriggerSource;

/// `<De-Digivolve 1> for each of your Digimon` as N separate instances (the
/// default formula reading), then an unrelated pick so a prompt of another
/// kind is parked while DCGO's count rows are replayed.
const PER_INSTANCE: &str = r#"
card: TEST-FOREACH
name: Test For Each
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] <De-Digivolve 1> 1 opponent Digimon for each of your Digimon"
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          prompt: "Select 1 of your opponent's Digimon to De-Digivolve"
      - de_digivolve:
          target: tgt
          amount:
            base: 0
            per:
              card_count_in_zone:
                of: you
                zone: battle_area
                filter: { kind: digimon }
            delta: 1
          stop_at_level: 3
      - select_opponent_permanent:
          bind_as: after
          filter: { kind: digimon }
          prompt: "An unrelated pick after the De-Digivolve"
"#;

/// The same, with no trailing prompt, so nothing is parked after the instances.
const PER_INSTANCE_LAST: &str = r#"
card: TEST-FOREACH
name: Test For Each
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] <De-Digivolve 1> 1 opponent Digimon for each of your Digimon"
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          prompt: "Select 1 of your opponent's Digimon to De-Digivolve"
      - de_digivolve:
          target: tgt
          amount:
            base: 0
            per:
              card_count_in_zone:
                of: you
                zone: battle_area
                filter: { kind: digimon }
            delta: 1
          stop_at_level: 3
"#;

/// ONE `<De-Digivolve N>`, N = the Digimon count (`declared: true`).
const DECLARED: &str = r#"
card: TEST-FOREACH
name: Test For Each
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] <De-Digivolve N> 1 opponent Digimon, N = your Digimon"
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          prompt: "Select 1 of your opponent's Digimon to De-Digivolve"
      - de_digivolve:
          target: tgt
          amount:
            base: 0
            per:
              card_count_in_zone:
                of: you
                zone: battle_area
                filter: { kind: digimon }
            delta: 1
          stop_at_level: 3
          declared: true
"#;

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

fn pick_top() -> StepSpec {
    sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["TOP", "OTHER"],
        "targets": [{"player": 1, "frame": 0}]}))
}

fn unit_count() -> StepSpec {
    sel(serde_json::json!({"prompt": "SelectCountEffect", "count": 1}))
}

/// P0's main phase: the fixture's [Start of Your Main Phase] is resolving and
/// asks its target. P0 has `own` Digimon (the N of the formula); P1 holds
/// `stack` (bottom first) on one slot and `OTHER` (Lv.4, no digivolution card)
/// on another.
fn board(yaml: &str, own: usize, stack: &[&str]) -> Game {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("fixture compiles")
        .add_card(digimon("OWN", 4))
        .add_card(digimon("SRC", 3))
        .add_card(digimon("MID", 4))
        .add_card(digimon("BIG", 5))
        .add_card(digimon("TOP", 6))
        .add_card(digimon("OTHER", 4))
        .deck(0, &["SRC"; 8])
        .deck(1, &["SRC"; 8])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    for _ in 0..own {
        r.place_on_field(0, "OWN", Some(0));
    }
    r.place_stack(1, stack);
    r.place_on_field(1, "OTHER", Some(0));
    let tamer = r.place_on_field(0, "TEST-FOREACH", Some(0));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(tamer));
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the target pick is parked");
    assert_eq!(p.selecting_player, 0, "{}", p.prompt);
    r.game
}

fn trash_of(s: &ReplaySession, player: usize) -> Vec<String> {
    s.game.players[player].trash.iter().map(|c| c.card_id(&s.game.card_data).to_string()).collect()
}

fn replay_all(s: &mut ReplaySession) {
    while s.current_step() < s.total_steps() {
        let before = s.current_step();
        s.step();
        assert!(
            s.divergences().is_empty() && !s.is_paused(),
            "row {before} diverged: {:?}; notes {:?}",
            s.divergences(),
            s.replay_notes()
        );
        assert!(s.current_step() > before, "row {before} did not advance the cursor");
    }
}

// ── 1. per-instance: a run of candidate-less unit counts ─────────────────────

/// 3 own Digimon: three `<De-Digivolve 1>` instances on
/// `[SRC(Lv3), SRC(Lv3), TOP(Lv6)]` (bottom first). The first trashes TOP, the
/// other two stand on the Lv3 floor with a digivolution card left and trash
/// nothing. DCGO opened `{1}` for each (the target still held a card): three
/// candidate-less count rows. The engine asked none, and an unrelated pick is
/// parked while they arrive; all three are consumed against the engine's
/// window.
#[test]
fn a_run_of_unit_counts_is_consumed_against_the_expanded_instances() {
    let mut s = staged(
        board(PER_INSTANCE, 3, &["SRC", "SRC", "TOP"]),
        vec![pick_top(), unit_count(), unit_count(), unit_count()],
    );
    replay_all(&mut s);
    assert_eq!(trash_of(&s, 1), vec!["TOP"], "instance 1 trashed TOP; 2 and 3 stood on the Lv3 floor");
    assert_eq!(s.game.players[1].battle_area[0].stack_size(), 2, "SRC over SRC stayed on the field");
    assert!(
        s.game.pending_selection.as_ref().is_some_and(|p| p.prompt.contains("unrelated")),
        "the unrelated pick is still parked"
    );
    let notes = s.replay_notes().iter().filter(|n| n.contains("<De-Digivolve> instance")).count();
    assert_eq!(notes, 3, "{:?}", s.replay_notes());
}

/// A fourth count after three instances is a count the engine never opened: it
/// halts instead of being swallowed.
#[test]
fn a_count_beyond_the_engines_instances_halts() {
    let mut s = staged(
        board(PER_INSTANCE_LAST, 3, &["SRC", "SRC", "TOP"]),
        vec![pick_top(), unit_count(), unit_count(), unit_count(), unit_count()],
    );
    for _ in 0..5 {
        s.step();
    }
    assert!(s.is_paused(), "the fourth count halts; notes {:?}", s.replay_notes());
    assert_eq!(s.current_step(), 4, "the pick and counts 1..=3 were consumed");
    assert!(
        s.replay_notes().iter().any(|n| n.contains("open only 3 count")),
        "{:?}",
        s.replay_notes()
    );
}

/// Fewer count rows than instances is not a halt by itself: the rows that are
/// there are consumed (the replay does not demand a row for every instance).
#[test]
fn a_shorter_run_is_consumed_row_by_row() {
    let mut s = staged(
        board(PER_INSTANCE_LAST, 3, &["SRC", "SRC", "TOP"]),
        vec![pick_top(), unit_count(), unit_count()],
    );
    replay_all(&mut s);
    assert_eq!(trash_of(&s, 1), vec!["TOP"]);
}

/// Instances that each trash a card: 2 own Digimon on `[SRC, MID, BIG]` trash
/// BIG then MID (2 instances, 2 cards): two count rows, a third halts.
#[test]
fn instances_that_each_trash_a_card_open_one_count_each() {
    let mut s = staged(
        board(PER_INSTANCE_LAST, 2, &["SRC", "MID", "BIG"]),
        vec![
            sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["BIG", "OTHER"],
                "targets": [{"player": 1, "frame": 0}]})),
            unit_count(),
            unit_count(),
            unit_count(),
        ],
    );
    for _ in 0..3 {
        s.step();
    }
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?} {:?}", s.divergences(), s.replay_notes());
    assert_eq!(trash_of(&s, 1), vec!["BIG", "MID"]);
    s.step();
    assert!(s.is_paused(), "a third count has no instance behind it; notes {:?}", s.replay_notes());
}

/// A count with no decision in front of it that shows the engine resolving a
/// De-Digivolve (no target was chosen) is not consumed.
#[test]
fn a_unit_count_with_no_resolved_de_digivolve_halts() {
    let mut s = staged(board(PER_INSTANCE_LAST, 3, &["SRC", "TOP"]), vec![unit_count()]);
    s.step();
    assert!(s.is_paused(), "notes {:?}", s.replay_notes());
    assert_eq!(s.current_step(), 0);
}

/// The engine's target is OTHER (no digivolution card): nothing was trashed, so
/// DCGO's count names no instance the engine ran.
#[test]
fn a_unit_count_when_nothing_was_trashed_halts() {
    let mut s = staged(
        board(PER_INSTANCE_LAST, 3, &["SRC", "TOP"]),
        vec![
            sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["TOP", "OTHER"],
                "targets": [{"player": 1, "frame": 1}]})),
            unit_count(),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    s.step();
    assert!(s.is_paused(), "notes {:?}", s.replay_notes());
    assert_eq!(s.current_step(), 1);
}

// ── 2. declared: ONE <De-Digivolve N> ────────────────────────────────────────

/// 3 own Digimon, `declared: true`, `[SRC, MID, BIG, TOP]` (3 trashable): the
/// engine parks the count declaration 1..=3 and DCGO's count 2 (no candidate
/// set) answers it: TOP and BIG are trashed, MID stays.
#[test]
fn a_declared_count_answers_the_parked_declaration() {
    let mut s = staged(
        board(DECLARED, 3, &["SRC", "MID", "BIG", "TOP"]),
        vec![pick_top(), sel(serde_json::json!({"prompt": "SelectCountEffect", "count": 2}))],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(
        s.game.pending_selection.as_ref().is_some_and(|p| p.prompt.contains("declare how many")),
        "the engine parks the <De-Digivolve N> declaration"
    );
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?} {:?}", s.divergences(), s.replay_notes());
    assert_eq!(trash_of(&s, 1), vec!["TOP", "BIG"], "declared 2 of 3");
}

/// `declared: true` but only ONE trashable card (`[SRC, TOP]`): the engine
/// asks nothing (CR 4.3 16-12-1), DCGO's count was `{1}` and is a unit count
/// covered by the one trashed card.
#[test]
fn a_declared_count_with_one_trashable_card_is_a_unit_count() {
    let mut s = staged(board(DECLARED, 3, &["SRC", "TOP"]), vec![pick_top(), unit_count()]);
    replay_all(&mut s);
    assert_eq!(trash_of(&s, 1), vec!["TOP"]);
}
