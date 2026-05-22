//! Phase 2 Track B — DSL surface tests for the `activation_cost:` step.
//!
//! Parse + compile + lowering coverage for the new step that lifts
//! "by suspending this Tamer" / "by returning this Tamer to the bottom
//! of the deck" cost-gates onto `EffectBuilder::activation_cost(...)`.
//!
//! Also: behavioral regression for the pre-cost decline prompt — a single
//! optional triggered effect carrying an `activation_cost` must expose a
//! `TriggerOrder` selection (with PASS) BEFORE the cost fires, and that
//! pre-cost decision must evaluate the effect's condition against the
//! queued effect's trigger context (PUPPETS-G028 review, Issue 1).

use digimon_engine::dsl::compile::compile;
use digimon_engine::dsl::compiled::{
    CompiledActivationCostKind, CompiledClause, CompiledStep,
};
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::step::StepSpec;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;
use digimon_engine::selection::SelectionKind;

fn parse_card(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).expect("test YAML parses")
}

const SUSPEND_SELF_BODY: &str = r#"
card: TEST-AC-001
name: SuspendSelfCostCard
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - activation_cost: { suspend_self: true }
      - draw: { of: you, count: 1 }
      - gain_memory: 1
"#;

const RETURN_SELF_BODY: &str = r#"
card: TEST-AC-002
name: ReturnSelfCostCard
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: start_of_your_main_phase
    optional: true
    process:
      - activation_cost: { return_self_to_deck_bottom: true }
      - draw: { of: you, count: 2 }
"#;

#[test]
fn parse_activation_cost_suspend_self_step() {
    let spec = parse_card(SUSPEND_SELF_BODY);
    let triggered = spec
        .effects
        .first()
        .and_then(|c| c.as_triggered())
        .expect("triggered clause present");
    let first = triggered
        .process
        .first()
        .expect("at least one process step");
    match first {
        StepSpec::ActivationCost(args) => {
            assert!(args.suspend_self, "suspend_self should round-trip");
            assert!(
                !args.return_self_to_deck_bottom,
                "return_self_to_deck_bottom should default to false"
            );
        }
        other => panic!("expected ActivationCost step, got {:?}", other),
    }
}

#[test]
fn parse_activation_cost_return_self_step() {
    let spec = parse_card(RETURN_SELF_BODY);
    let triggered = spec
        .effects
        .first()
        .and_then(|c| c.as_triggered())
        .expect("triggered clause present");
    match &triggered.process[0] {
        StepSpec::ActivationCost(args) => {
            assert!(args.return_self_to_deck_bottom);
            assert!(!args.suspend_self);
        }
        other => panic!("expected ActivationCost step, got {:?}", other),
    }
}

#[test]
fn compile_activation_cost_lowers_to_suspend_self_kind() {
    let spec = parse_card(SUSPEND_SELF_BODY);
    let compiled = compile(&spec).expect("activation_cost suspend_self: true compiles cleanly");
    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("compiled triggered clause");
    match &triggered.process[0] {
        CompiledStep::ActivationCost { kind } => {
            assert_eq!(*kind, CompiledActivationCostKind::SuspendSelf);
        }
        other => panic!("expected CompiledStep::ActivationCost, got {:?}", other),
    }
}

#[test]
fn compile_activation_cost_lowers_to_return_self_kind() {
    let spec = parse_card(RETURN_SELF_BODY);
    let compiled = compile(&spec).expect("should compile cleanly");
    let triggered = compiled
        .effects
        .iter()
        .find_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .expect("compiled triggered clause");
    match &triggered.process[0] {
        CompiledStep::ActivationCost { kind } => {
            assert_eq!(*kind, CompiledActivationCostKind::ReturnSelfToDeckBottom);
        }
        other => panic!("expected CompiledStep::ActivationCost, got {:?}", other),
    }
}

fn collect_errors(yaml: &str) -> Vec<String> {
    let spec = parse_card(yaml);
    match compile(&spec) {
        Ok(_) => Vec::new(),
        Err(errs) => errs.into_iter().map(|e| e.message).collect(),
    }
}

#[test]
fn validator_rejects_activation_cost_mid_body() {
    let yaml = r#"
card: TEST-AC-003
name: BadMidBody
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - draw: { of: you, count: 1 }
      - activation_cost: { suspend_self: true }
"#;
    let messages = collect_errors(yaml);
    assert!(
        !messages.is_empty(),
        "activation_cost must not be allowed mid-body — validator must reject"
    );
    let joined = messages.join("\n");
    assert!(
        joined.contains("activation_cost must be the first step"),
        "validator error must explain the placement rule; got: {}",
        joined
    );
}

#[test]
fn validator_rejects_activation_cost_with_no_kind_set() {
    let yaml = r#"
card: TEST-AC-004
name: NoKind
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - activation_cost: {}
      - draw: { of: you, count: 1 }
"#;
    let messages = collect_errors(yaml);
    assert!(
        !messages.is_empty(),
        "activation_cost with no kind set must be rejected"
    );
    let joined = messages.join("\n");
    assert!(
        joined.contains("activation_cost requires exactly one cost kind"),
        "got: {}",
        joined
    );
}

#[test]
fn validator_rejects_activation_cost_with_both_kinds_set() {
    let yaml = r#"
card: TEST-AC-005
name: BothKinds
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    optional: true
    process:
      - activation_cost: { suspend_self: true, return_self_to_deck_bottom: true }
      - draw: { of: you, count: 1 }
"#;
    let messages = collect_errors(yaml);
    assert!(
        !messages.is_empty(),
        "activation_cost with both suspend_self + return_self_to_deck_bottom must be rejected"
    );
    let joined = messages.join("\n");
    assert!(joined.contains("mutually exclusive"), "got: {}", joined);
}

// ─── Pre-cost decline prompt: trigger-context-gated condition ───────────────
//
// PUPPETS-G028 review, Issue 1. A single optional triggered effect carrying an
// `activation_cost_fn` installs a `TriggerOrder` selection (with PASS) BEFORE
// the cost fires, so the player can decline without paying. The decision is
// gated by `needs_pre_cost_prompt`, which pre-evaluates the effect's condition.
//
// The condition here reads the trigger context (`event_target_trait_has`), so
// the pre-cost check must install the queued effect's trigger context before
// evaluating it — exactly as the real `run_queued_effect` drain path does.
// Without that, the condition silently sees `current_trigger_context == None`,
// fails, and the cost auto-fires with NO decline opportunity (a
// no-approximations violation).

/// Synthetic Tamer mirroring the EX11-054 clause-2 shape: a single optional
/// triggered observer with `activation_cost: { suspend_self: true }` gated on
/// an event-context condition (`event_target_owner` + `event_target_trait_has`).
const PRE_COST_EVENT_GATED: &str = r#"
card: TST-AC-PRECOST
name: PreCostEventGatedCard
kind: tamer
color: [yellow]
cost: 3
effects:
  - when: on_any_digimon_played
    active_when: { all_turns: true }
    optional: true
    summary: "When your Reptile is played, by suspending self draw 1"
    condition:
      all_of:
        - event_target_owner: you
        - event_target_trait_has: Reptile
        - source_is_unsuspended: true
    process:
      - activation_cost: { suspend_self: true }
      - draw: { of: you, count: 1 }
"#;

fn pre_cost_runner(played_traits: Vec<String>) -> (DebugRunner, usize) {
    let mut played = make_test_card("PLAYED-AC-PRECOST", "PlayedDigimon");
    played.card_kind = CardKind::Digimon;
    played.level = Some(3);
    played.dp = Some(2000);
    played.play_cost = 0;
    played.traits = played_traits;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(PRE_COST_EVENT_GATED)
        .expect("inline DSL compiles")
        .add_card(played)
        .add_card(make_test_card("DRAW-AC-PRECOST", "DrawFiller"))
        .deck(0, &["DRAW-AC-PRECOST"])
        .memory(20)
        .start();

    let tamer = runner.place_on_field(0, "TST-AC-PRECOST", None);
    assert!(
        !runner.game.player(0).battle_area[tamer.index as usize].is_suspended,
        "Tamer must start unsuspended so the suspend cost is payable"
    );

    // Put the to-be-played Digimon in P0's hand.
    use digimon_engine::card_source::CardSource;
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "PLAYED-AC-PRECOST")
        .expect("PLAYED-AC-PRECOST registered");
    let card_index = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(CardSource::new(data_idx, 0, card_index));
    let hand_index = runner.game.players[0].hand.len() - 1;
    (runner, hand_index)
}

/// TRUE branch: the played Digimon has the [Reptile] trait, so the
/// event-context condition genuinely passes. The player MUST be offered the
/// pre-cost decline prompt — a pending `TriggerOrder` selection with PASS.
///
/// Before the Issue 1 fix this FAILED: `needs_pre_cost_prompt` evaluated the
/// condition with `current_trigger_context == None`, so `event_target_owner` /
/// `event_target_trait_has` returned false, no prompt installed, and the cost
/// auto-fired.
#[test]
fn pre_cost_prompt_appears_when_event_condition_is_true() {
    let (mut runner, hand_index) = pre_cost_runner(vec!["Reptile".to_string()]);
    let hand_before = runner.hand_size(0);

    runner.play(0, hand_index);

    assert_eq!(
        runner.pending_kind(),
        Some(SelectionKind::TriggerOrder),
        "an optional activation-cost trigger whose event-condition is TRUE \
         must install a pre-cost TriggerOrder selection so the player can decline"
    );
    assert!(
        runner.pending_is_optional(),
        "the pre-cost TriggerOrder must expose PASS (the trigger is optional)"
    );

    // The suspend cost must NOT have fired yet — we are still at the decline
    // prompt, before the body runs.
    let tamer = &runner.game.player(0).battle_area[0];
    assert!(
        !tamer.is_suspended,
        "the suspend activation cost must not fire before the decline prompt"
    );
    // ... and no draw has happened yet (REPTILE play consumed nothing extra).
    assert_eq!(
        runner.hand_size(0),
        hand_before - 1,
        "only the played Digimon left hand; the Draw 1 body has not run yet"
    );
}

/// FALSE branch: the played Digimon is a [Beast] (not Reptile), so the
/// event-context condition genuinely fails. No prompt must appear and the
/// effect auto-skips WITHOUT paying the suspend cost.
#[test]
fn pre_cost_prompt_absent_when_event_condition_is_false() {
    let (mut runner, hand_index) = pre_cost_runner(vec!["Beast".to_string()]);

    runner.play(0, hand_index);

    assert!(
        runner.pending_selection().is_none(),
        "no pre-cost prompt may appear when the event-condition is FALSE \
         (non-Reptile played) — the effect auto-skips"
    );
    let tamer = &runner.game.player(0).battle_area[0];
    assert!(
        !tamer.is_suspended,
        "the suspend cost must not be paid when the condition fails"
    );
}
