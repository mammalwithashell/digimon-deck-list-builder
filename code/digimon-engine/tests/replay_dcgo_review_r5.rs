//! DCGO replay: round-4 review findings on the replay (review lane, round 5).
//!
//! 3. A `SelectCountEffect` whose candidate set is ONE value is DCGO's forced
//!    count (`IDegeneration.Degeneration`, CardController.cs:5281-5297, asks
//!    it even when `min(digivolution cards, N)` leaves one value, `CanNoSelect:
//!    false`), which the engine never asks (CR 4.3 16-12-1). Consuming it with
//!    a note is right only when the engine resolved that effect: since the
//!    decision before the count, an effect of the count's actor chose a target
//!    and exactly `count` cards left one permanent for the trash. With nothing
//!    resolved the count is DCGO running an effect the engine did not: a halt.
//! 8. Under a card-less trigger-order entry, an unstamped DCGO decline that
//!    meets effect activity it cannot attribute ("cannot tell", finding 13)
//!    neither confirms nor refutes the entry: it is UNDETERMINED (an
//!    ambiguity), never a rejection.
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

const DEDIGIVOLVE: &str = r#"
card: TEST-DEDIGI
name: Test De-Digivolve
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] <De-Digivolve 1> 1 of your opponent's Digimon"
    process:
      - select_opponent_permanent:
          bind_as: tgt
          filter: { kind: digimon }
          prompt: "Select 1 of your opponent's Digimon to De-Digivolve 1"
      - de_digivolve:
          target: tgt
          amount: 1
"#;

fn gain_one(id: &str) -> String {
    format!(
        r#"
card: {id}
name: Test Gain One {id}
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Gain 1 memory"
    process:
      - gain_memory: 1
"#
    )
}

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

// ── 3. a forced single-value count ───────────────────────────────────────────

/// P0's main phase: TEST-DEDIGI's [Start of Your Main Phase] <De-Digivolve 1>
/// is resolving and asks its target among P1's TOP (Lv.4 on a Lv.3 SRC: one
/// card to trash) and OTHER.
fn dedigivolve_target_parked() -> Game {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(DEDIGIVOLVE)
        .expect("de-digivolve fixture compiles")
        .add_card(digimon("SRC", 3))
        .add_card(digimon("TOP", 4))
        .add_card(digimon("OTHER", 4))
        .deck(0, &["SRC"; 8])
        .deck(1, &["SRC"; 8])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_stack(1, &["SRC", "TOP"]);
    r.place_on_field(1, "OTHER", Some(0));
    let tamer = r.place_on_field(0, "TEST-DEDIGI", Some(0));
    r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(tamer));
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the target pick is parked");
    assert_eq!(p.selecting_player, 0, "{}", p.prompt);
    r.game
}

/// DCGO asked the target, then the forced count {1}; the engine trashed TOP
/// with no count prompt. The count row is consumed with a note naming that
/// resolution.
#[test]
fn forced_count_after_the_engine_resolved_the_de_digivolve_is_a_note() {
    let mut s = staged(
        dedigivolve_target_parked(),
        vec![
            sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["TOP", "OTHER"],
                "targets": [{"player": 1, "frame": 0}]})),
            sel(serde_json::json!({"prompt": "SelectCountEffect", "count": 1, "candidates": [1]})),
        ],
    );
    replay_all(&mut s);
    let trash: Vec<String> = s.game.players[1].trash.iter().map(|c| c.card_id(&s.game.card_data).to_string()).collect();
    assert_eq!(trash, vec!["TOP"], "the engine's <De-Digivolve 1> trashed TOP");
    assert!(
        s.replay_notes().iter().any(|n| n.contains("SelectCountEffect") && n.contains("forced") && n.contains("TOP")),
        "{:?}",
        s.replay_notes()
    );
}

/// The engine's target is OTHER (no digivolution card: its <De-Digivolve 1>
/// trashes nothing), yet DCGO's forced count names 1 card: the window shows
/// no card leaving for the trash, so this is not the resolution DCGO counted.
/// It halts.
#[test]
fn forced_count_the_engine_resolution_does_not_match_halts() {
    let mut s = staged(
        dedigivolve_target_parked(),
        vec![
            sel(serde_json::json!({"prompt": "SelectPermanentEffect", "board_p1": ["TOP", "OTHER"],
                "targets": [{"player": 1, "frame": 1}]})),
            sel(serde_json::json!({"prompt": "SelectCountEffect", "count": 1, "candidates": [1]})),
        ],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    s.step();
    let reason = halt_reason(&s, 1);
    assert!(reason.contains("SelectCountEffect") && reason.contains("forced"), "{reason}");
}

// ── 8. "cannot tell" under a card-less order entry is undetermined ───────────

/// P0's main phase: GAIN-A and GAIN-B's mandatory [Start of Your Main Phase]
/// "gain 1 memory" are ordered (the order prompt is parked).
fn two_gains_ordered() -> Game {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(&gain_one("GAIN-A"))
        .expect("gain fixture compiles")
        .from_dsl_yaml(&gain_one("GAIN-B"))
        .expect("gain fixture compiles")
        .add_card(digimon("FILL", 3))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    let a = r.place_on_field(0, "GAIN-A", Some(0));
    let b = r.place_on_field(0, "GAIN-B", Some(0));
    for t in [a, b] {
        r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(t));
    }
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the order is parked");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder) && p.valid_action_ids.len() == 2, "{:?}", p.kind);
    r.game
}

/// A card-less index 0, then P0's unstamped decline. Under either entry the
/// engine resolves both gains (memory changes from a source card) and then
/// meets the decline with nothing parked: the decline names no card, so the
/// replay cannot tell whether a gain was the effect DCGO declined. That
/// neither proves nor refutes the entry: the order is reported as an
/// ambiguity, not as "no entry fits".
#[test]
fn cannot_tell_decline_under_each_order_entry_is_an_ambiguity_not_a_refutation() {
    let mut s = staged(
        two_gains_ordered(),
        vec![
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 0})),
            sel(serde_json::json!({"prompt": "generic_bool", "bool_value": false, "unstamped": true})),
        ],
    );
    s.step();
    let reason = halt_reason(&s, 0);
    assert!(reason.contains("is ambiguous"), "{reason}");
    assert!(reason.contains("cannot tell"), "{reason}");
    assert!(!reason.contains("no entry"), "{reason}");
}
