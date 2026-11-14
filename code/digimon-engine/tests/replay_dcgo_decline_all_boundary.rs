//! DCGO replay: `MultipleSkills` -1 ("Don't activate these effects") drops
//! ONE stack - the effects of the panel it answers - never the stack under it.
//!
//! DCGO processes the triggers stacked at one moment in one
//! `ActivateMultipleSkills` call (`AutoProcessing.TriggeredSkillProcess`,
//! AutoProcessing.cs:584-600); triggers that arise while one of them resolves
//! are stacked anew and met by a nested call before the outer call picks its
//! next effect. -1 clears that call's own `StackedSkillInfos`
//! (MultipleSkills.cs:474-480, the list built from its own `skillInfos`,
//! :166-171); the outer call's remaining effects stay and are asked next (CR
//! 4.3 8-1: effects triggered during an effect's resolution resolve after it,
//! before the remaining waiting effects - the engine's nested queue).
//!
//! raw-20261113T025510Z#g6 replay step 108 (raw seq 1006-1007): RizeGreymon
//! (ST24-06)'s [When Attacking] trashed face-down cards from two Marcus Damon
//! & Thomas H. Norstein (ST24-13), stacking both Tamers' optional "by
//! suspending this Tamer" triggers; P1 answered that stack with -1, and the
//! very next row chose Kudamon (EX13-026)'s [When Attacking] from the OUTER
//! stack. The replay's -1 kept declining past the nested stack into the outer
//! one, met its mandatory order, and stopped.
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

const IF_HAND_GAIN: &str = r#"
card: TEST-HAND-GAIN
name: Test Hand Gain
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] If you have 2 or more cards in hand, gain 1 memory"
    process:
      - if:
          condition:
            count_gte:
              filter: { of: you, zone: [hand] }
              n: 2
          then:
            - gain_memory: 1
"#;

const DRAW_ONE: &str = r#"
card: TEST-DRAW-ONE
name: Test Draw One
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Draw 1"
    process:
      - draw: { of: you, count: 1 }
"#;

const PLAYER: &str = r#"
card: TEST-PLAYER
name: Test Player
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: start_of_your_main_phase
    summary: "[Start of Your Main Phase] Play 1 [Played] trait Digimon from your hand without paying the cost"
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter:
            all_of:
              - kind: digimon
              - trait_has: Played
          prompt: "Play 1 [Played] Digimon from your hand"
      - play_from_hand_free:
          of: you
          hand_index: pick
"#;

const WATCH: &str = r#"
card: TEST-WATCH
name: Test Watch
kind: tamer
color: [yellow]
cost: 3
traits: [Test]
effects:
  - when: on_ally_played
    optional: true
    active_when: { your_turn: true }
    trigger_if:
      all_of:
        - event_target_kind: digimon
        - event_target_is_source: false
    summary: "[Your Turn] When one of your Digimon is played, you may gain 1 memory"
    process:
      - gain_memory: 1
"#;

fn digimon(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(3);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

/// P0's main phase, hand FILL + DIGI. Three mandatory [Start of Your Main
/// Phase] triggers wait (HAND-GAIN, DRAW-ONE, PLAYER); HAND-GAIN and
/// DRAW-ONE give different games in the two orders. PLAYER plays DIGI, which
/// stacks the two WATCH Tamers' optional triggers on top.
fn outer_stack_under_a_nested_optional_one() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(IF_HAND_GAIN)
        .expect("hand-gain fixture compiles")
        .from_dsl_yaml(DRAW_ONE)
        .expect("draw fixture compiles")
        .from_dsl_yaml(PLAYER)
        .expect("player fixture compiles")
        .from_dsl_yaml(WATCH)
        .expect("watch fixture compiles")
        .add_card(digimon("FILL", &[]))
        .add_card(digimon("DIGI", &["Played"]))
        .hand(0, &["FILL", "DIGI"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 6])
        .start();
    r.set_first_player(0);
    r.skip_mulligan();
    r.game.enter_main_phase();
    r.place_on_field(0, "TEST-WATCH", Some(0));
    r.place_on_field(0, "TEST-WATCH", Some(0));
    let a = r.place_on_field(0, "TEST-HAND-GAIN", Some(0));
    let b = r.place_on_field(0, "TEST-DRAW-ONE", Some(0));
    let c = r.place_on_field(0, "TEST-PLAYER", Some(0));
    for t in [a, b, c] {
        r.game.enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(t));
    }
    r.game.drain_effect_queue();
    let p = r.game.pending_selection.as_ref().expect("the outer order is parked");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder) && !p.is_optional, "{:?}", p.kind);
    r
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
    let row: digimon_engine::dcgo_recording::SelectionRow =
        serde_json::from_value(v).expect("selection row");
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

fn source_card_id(game: &Game) -> Option<String> {
    let p = game.pending_selection.as_ref()?;
    game.card_data_for_handle(p.source_card).map(|d| d.card_id.clone())
}

/// -1 declines the two WATCH triggers (the nested stack) and stops there:
/// the next row chooses DRAW-ONE from the outer stack, and HAND-GAIN is
/// still waiting (draw first: 2 cards in hand when it resolves).
#[test]
fn decline_all_drops_the_nested_stack_and_leaves_the_outer_one_to_the_next_row() {
    let r = outer_stack_under_a_nested_optional_one();
    let memory = r.game.memory;
    let mut s = staged(
        r.game,
        vec![
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 2, "chosen_effect_card": "TEST-PLAYER"})),
            sel(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["DIGI"]})),
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1})),
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 0, "chosen_effect_card": "TEST-DRAW-ONE"})),
        ],
    );
    replay_all(&mut s);
    assert_eq!(s.game.players[0].hand.len(), 2, "FILL plus the drawn card");
    // HAND-GAIN, the lone trigger left, resolved after the draw (2 cards in
    // hand: +1). Had either WATCH trigger run, memory would be +2 or more;
    // had HAND-GAIN run first (1 card), +0.
    assert!(s.game.pending_selection.is_none(), "{:?}", source_card_id(&s.game));
    assert_eq!(s.game.memory, memory + 1, "only HAND-GAIN gained, after the draw");
}

/// -1 met with NO later row declines the nested stack only; the outer
/// mandatory order is left parked for whatever follows.
#[test]
fn decline_all_leaves_the_outer_mandatory_order_parked() {
    let r = outer_stack_under_a_nested_optional_one();
    let memory = r.game.memory;
    let mut s = staged(
        r.game,
        vec![
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": 2, "chosen_effect_card": "TEST-PLAYER"})),
            sel(serde_json::json!({"prompt": "SelectHandEffect", "zone": "Hand", "card_ids": ["DIGI"]})),
            sel(serde_json::json!({"prompt": "MultipleSkills", "int_value": -1})),
        ],
    );
    replay_all(&mut s);
    assert_eq!(s.game.memory, memory);
    let p = s.game.pending_selection.as_ref().expect("the outer order waits");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder) && !p.is_optional, "{:?} {}", p.kind, p.prompt);
    assert_eq!(p.valid_action_ids.len(), 2, "{}", p.prompt);
    let _ = DivergenceKind::SelectionResolution { reason: String::new() };
}
