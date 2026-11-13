//! DCGO replay: answers recorded for the REMOTE seat of a PvP recording.
//!
//! The opponent's client has no recorder, so its rows carry less than the
//! local seat's:
//!
//! * a `MultipleSkills` row holds DCGO's index into ITS ordering of the
//!   simultaneous effects but no `chosen_effect_card` (there is no
//!   `effect_activation` row to read it from), and our trigger-order list does
//!   not share DCGO's ordering. The replay must prove the choice from the
//!   actor's next recorded row, or stop with a divergence. It must never read
//!   the index positionally.
//! * an `OptionalSkill` answer cannot be stamped with an engine action; the
//!   converter writes it as an `unstamped` `generic_bool`, which answers only
//!   an optional accept gate of that seat and is otherwise a no-op.
//!
//! Field witness: the Virus Busters vs Toho Braves PvP game of 2026-11-13
//! (opponent P0). At EX12-046 Shishimamon's End of Attack DCGO's index 0 was
//! the inherited "play 1 [TB] Digimon with 5000 DP or less from hand" while
//! the engine's 0 was <Execute>'s self-delete; the opponent's next row picked
//! Shellmon (EX12-026) from hand.
//!
//! The game is staged with `DebugRunner` and served through a minimal
//! `RecordingSource` (a DCGO-policy source whose steps are the rows under
//! test). That skips the recording's setup, which these rules do not touch.
//! Not covered here: the converter (`convert_raw.py`) that writes the rows.

use std::collections::HashMap;

use digimon_engine::action::space::REPLACEMENT_ACCEPT;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::dcgo_recording::SelectionRow;
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::game::Game;
use digimon_engine::runners::replay::{
    DivergenceKind, RecordingSource, ReplayError, ReplaySession, StepPolicy, StepSpec,
};
use digimon_engine::selection::{SelectionKind, TriggerSource};

const SHISHIMAMON: &str = "EX12-046";

/// A second inherited [End of Attack] whose follow-up is an OPPONENT field
/// pick, so it can never answer a hand-card row.
const DEBUFFER_YAML: &str = r#"
card: TEST-EOA-DEBUFF
name: Test End of Attack Debuffer
kind: digimon
level: 4
color: [yellow]
cost: 5
dp: 5000
traits: [Test]
effects:
  - scope: inherited
    when: end_of_attack
    optional: true
    summary: "[Inherited][End of Attack] 1 opponent Digimon gets -1000 DP"
    process:
      - select_opponent_permanent:
          bind_as: debuff_target
          filter: { kind: digimon }
          prompt: "Choose 1 opponent Digimon to get -1000 DP"
      - add_dp_modifier:
          target: debuff_target
          value: -1000
          expiry: end_of_opponents_turn
"#;

fn digimon(id: &str, level: u8, dp: i32, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.play_cost = 5;
    card.dp = Some(dp);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

/// P0 (the remote seat) attacked with CARRIER, whose sources are
/// `sources` (bottom first); its End of Attack effects are now pending.
fn end_of_attack_with(sources: &[&str]) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card(SHISHIMAMON)
        .expect("EX12-046 YAML loads")
        .from_dsl_yaml(DEBUFFER_YAML)
        .expect("debuffer fixture compiles")
        .add_card(digimon("CARRIER", 6, 9000, &[]))
        .add_card(digimon("TB-LOW", 4, 5000, &["Shambala", "TB"]))
        .add_card(digimon("OPP", 4, 6000, &[]))
        .hand(0, &["TB-LOW"])
        .memory(3)
        .start();
    runner.game.turn_count = 1;
    runner.place_stack(1, &["OPP"]);
    let mut stack: Vec<&str> = sources.to_vec();
    stack.push("CARRIER");
    let carrier = runner.place_stack(0, &stack);
    runner
        .game
        .enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    runner
}

/// A DCGO-policy source over a staged game: the steps are the rows under
/// test, the initial game is the staged one.
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

fn row(json: serde_json::Value) -> StepSpec {
    let selection: SelectionRow = serde_json::from_value(json).expect("selection row");
    StepSpec {
        actor: selection.actor,
        action_id: 0,
        phase: "Main".into(),
        source: "selection".into(),
        memory_after: None,
        dcgo_memory: None,
        turn: None,
        is_game_over: None,
        expected_digest: None,
        selection: Some(selection),
        board_p0: None,
        board_p1: None,
        card_id: None,
        dna_materials: None,
        inferred: false,
    }
}

fn session(runner: DebugRunner, steps: Vec<StepSpec>) -> ReplaySession {
    ReplaySession::with_source(Box::new(StagedDcgo { game: runner.game, steps }), &HashMap::new(), false)
        .expect("staged session")
}

/// The engine's order-prompt position of the trigger whose source is `card_id`.
fn order_position(game: &Game, card_id: &str) -> usize {
    let p = game.pending_selection.as_ref().expect("an order prompt is parked");
    assert!(matches!(p.kind, SelectionKind::TriggerOrder), "expected a trigger order, got {:?}", p.kind);
    p.effect_choices
        .as_ref()
        .expect("trigger entries")
        .iter()
        .position(|e| e.source_card.and_then(|c| game.card_data_for_handle(c)).is_some_and(|d| d.card_id == card_id))
        .unwrap_or_else(|| panic!("{card_id} trigger not offered"))
}

fn on_field(game: &Game, player: usize, card_id: &str) -> bool {
    game.players[player].battle_area.iter().any(|p| p.top_card().card_id(&game.card_data) == card_id)
}

fn in_hand(game: &Game, player: usize, card_id: &str) -> bool {
    game.players[player].hand.iter().any(|c| c.card_id(&game.card_data) == card_id)
}

/// The index DCGO recorded points, read positionally, at the WRONG trigger
/// (the debuffer); the opponent's next row picks a hand card, which only the
/// Shishimamon trigger asks for. The row is resolved by that evidence.
#[test]
fn cardless_order_index_is_resolved_by_the_next_card_row_not_by_position() {
    let runner = end_of_attack_with(&["TEST-EOA-DEBUFF", SHISHIMAMON]);
    let wrong = order_position(&runner.game, "TEST-EOA-DEBUFF");
    let mut s = session(
        runner,
        vec![
            row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":wrong})),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["TB-LOW"]})),
        ],
    );

    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1, "only the MultipleSkills row is consumed");
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(on_field(&s.game, 0, "TB-LOW"), "Shishimamon's inherited play ran first and played TB-LOW");
    assert!(!in_hand(&s.game, 0, "TB-LOW"));
}

/// A card-less index followed by nothing that can tell the two effects apart
/// (an int row matches every branch positionally) is not guessed: the replay
/// stops with a SelectionResolution divergence that names the ambiguity.
#[test]
fn ambiguous_cardless_order_index_is_a_divergence_not_a_guess() {
    let runner = end_of_attack_with(&["TEST-EOA-DEBUFF", SHISHIMAMON]);
    let before = runner.game.verification_digest();
    let mut s = session(
        runner,
        vec![
            row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":0})),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"generic_int","int_value":0})),
        ],
    );

    s.step();
    assert!(s.is_paused(), "an unprovable order must stop the replay");
    assert_eq!(s.current_step(), 0);
    assert_eq!(s.game.verification_digest(), before, "nothing was applied");
    match s.divergences() {
        [d] => match &d.kind {
            DivergenceKind::SelectionResolution { reason } => {
                assert!(reason.contains("ambiguous"), "{reason}");
                assert!(reason.contains("no chosen_effect_card"), "{reason}");
            }
            other => panic!("expected SelectionResolution, got {other:?}"),
        },
        other => panic!("expected one divergence, got {other:?}"),
    }
}

/// A row that names a card is not evidence for a choice it cannot answer
/// under ANY order: with no entry leading to a prompt that takes it, the
/// replay reports that instead of taking an index.
#[test]
fn cardless_order_index_with_an_unanswerable_next_row_is_a_divergence() {
    let runner = end_of_attack_with(&["TEST-EOA-DEBUFF", SHISHIMAMON]);
    let mut s = session(
        runner,
        vec![
            row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":0})),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["CARRIER"]})),
        ],
    );
    s.step();
    assert!(s.is_paused());
    match s.divergences() {
        [d] => assert!(
            matches!(&d.kind, DivergenceKind::SelectionResolution { reason } if reason.contains("no entry")),
            "{d:?}"
        ),
        other => panic!("expected one divergence, got {other:?}"),
    }
}

/// An unstamped `false` declines the "you may" gate it meets: P0 refused the
/// Shishimamon play, so TB-LOW stays in hand.
#[test]
fn unstamped_false_declines_a_parked_optional_gate() {
    let runner = end_of_attack_with(&[SHISHIMAMON]);
    let mut s = session(
        runner,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":false,"unstamped":true}))],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    assert!(in_hand(&s.game, 0, "TB-LOW"), "declined: nothing played");
    assert!(!on_field(&s.game, 0, "TB-LOW"));
    assert!(
        s.game.pending_selection.as_ref().is_none_or(|p| !p.valid_action_ids.contains(&REPLACEMENT_ACCEPT)),
        "the gate was answered"
    );
    // Shishimamon's "you may play" folds its may into the hand pick (no outer
    // accept/decline): that pick is the activation gate, and `false` declines it.
    assert!(s.game.pending_selection.is_none(), "the folded gate was declined: {:?}", s.game.pending_selection);
    assert!(s.replay_notes().is_empty(), "{:?}", s.replay_notes());
}

/// An unstamped `true` against the folded gate (the effect's own optional pick)
/// only means "proceed to the pick": the pick stays parked for the row that
/// names the card, and that row plays it.
#[test]
fn unstamped_true_against_a_folded_gate_leaves_the_pick_for_the_next_row() {
    let runner = end_of_attack_with(&[SHISHIMAMON]);
    let mut s = session(
        runner,
        vec![
            row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true})),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["TB-LOW"]})),
        ],
    );
    let before = s.game.verification_digest();
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    assert_eq!(s.game.verification_digest(), before, "no engine action for a yes");
    assert!(s.replay_notes().is_empty(), "the gate matched: {:?}", s.replay_notes());
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(on_field(&s.game, 0, "TB-LOW"));
}

/// The same row with no optional gate parked (the engine asks no such
/// question here) is consumed without touching the game, whatever its value.
#[test]
fn unstamped_bool_without_a_parked_gate_is_a_no_op() {
    for value in [false, true] {
        let runner = DebugRunner::builder()
            .add_card(digimon("CARRIER", 6, 9000, &[]))
            .hand(0, &["CARRIER"])
            .memory(3)
            .start();
        assert!(runner.game.pending_selection.is_none());
        let before = runner.game.verification_digest();
        let mut s = session(
            runner,
            vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":value,"unstamped":true}))],
        );
        s.step();
        assert!(s.divergences().is_empty(), "{:?}", s.divergences());
        assert_eq!(s.current_step(), 1, "the row is consumed");
        assert_eq!(s.game.verification_digest(), before, "and changes nothing (value {value})");
    }
}

/// An unstamped answer never answers a prompt that is not an optional accept
/// gate: with two triggers waiting to be ordered, a `true` is a no-op (it is
/// not read as "the first effect").
#[test]
fn unstamped_bool_does_not_answer_an_order_prompt() {
    let runner = end_of_attack_with(&["TEST-EOA-DEBUFF", SHISHIMAMON]);
    let before = runner.game.verification_digest();
    let mut s = session(
        runner,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true}))],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    assert_eq!(s.game.verification_digest(), before);
    assert!(matches!(s.game.pending_selection.as_ref().map(|p| &p.kind), Some(SelectionKind::TriggerOrder)));
    assert_eq!(s.replay_notes().len(), 1, "the unmatched row is noted: {:?}", s.replay_notes());
}

/// An answered gate leaves no note.
#[test]
fn unstamped_bool_that_answers_a_gate_leaves_no_note() {
    let runner = end_of_attack_with(&[SHISHIMAMON]);
    let mut s = session(
        runner,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":false,"unstamped":true}))],
    );
    s.step();
    assert!(s.divergences().is_empty(), "{:?}", s.divergences());
    assert!(s.replay_notes().is_empty(), "{:?}", s.replay_notes());
}

/// An unstamped answer that meets no gate is consumed with a NON-halting note
/// naming the row, so a later divergence can be traced to it (e.g. a trigger
/// the engine never offered).
#[test]
fn unstamped_bool_without_a_gate_is_noted() {
    let runner = DebugRunner::builder()
        .add_card(digimon("CARRIER", 6, 9000, &[]))
        .hand(0, &["CARRIER"])
        .memory(3)
        .start();
    let mut s = session(
        runner,
        vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":true,"unstamped":true}))],
    );
    s.step();
    assert!(s.divergences().is_empty() && !s.is_paused(), "{:?}", s.divergences());
    assert_eq!(s.current_step(), 1);
    match s.replay_notes() {
        [note] => {
            assert!(note.contains("row 0"), "{note}");
            assert!(note.contains("unstamped generic_bool true"), "{note}");
            assert!(note.contains("no prompt"), "{note}");
        }
        other => panic!("expected one note, got {other:?}"),
    }
}

/// A yes/no-shaped choice INSIDE an effect body is not DCGO's "use this
/// optional effect?" gate: ST24-06 RizeGreymon's "Trash 2 face-down cards" /
/// "Don't" (`select_effect_choice`, the label shape `yes_no_branches` keys on)
/// is registered `optional: false` in DCGO (`ST24_06.cs`), which never sends
/// `SetUseOptional` for it. An unstamped bool must leave it parked.
#[test]
fn unstamped_bool_does_not_answer_a_yes_no_body_choice() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::enums::{EffectSourceKind, GamePhase};
    use digimon_engine::selection::{EffectChoiceEntry, PendingSelection};

    for value in [false, true] {
        let mut runner = DebugRunner::builder()
            .add_card(digimon("CARRIER", 6, 9000, &[]))
            .hand(0, &["CARRIER"])
            .memory(3)
            .start();
        let entry = |label: &str, action_id: u16| EffectChoiceEntry {
            label: label.into(),
            action_id,
            source_card: None,
            source_kind: None,
            timing: None,
            is_optional: false,
            keyword: None,
            observation_metadata: Default::default(),
            count_choice: None,
        };
        runner.game.pending_selection = Some(PendingSelection {
            card_choice: None,
            zone_owner: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: 0,
            previous_phase: GamePhase::Main,
            valid_action_ids: vec![30, 31],
            is_optional: false,
            prompt: "Trash 2 face-down cards?".into(),
            effect_choices: Some(vec![entry("Trash 2 face-down cards", 30), entry("Don't", 31)]),
            source_card: CardHandle(0),
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(|_, _| {}),
            on_decline: None,
        });
        let mut s = session(
            runner,
            vec![row(serde_json::json!({"step":0,"actor":0,"prompt":"generic_bool","bool_value":value,"unstamped":true}))],
        );
        s.step();
        assert!(s.divergences().is_empty(), "{:?}", s.divergences());
        assert_eq!(s.current_step(), 1, "the row is consumed");
        let p = s.game.pending_selection.as_ref().expect("the body choice is still parked (value {value})");
        assert_eq!(p.prompt, "Trash 2 face-down cards?");
        assert_eq!(s.replay_notes().len(), 1, "{:?}", s.replay_notes());
    }
}

/// A mandatory inherited [End of Attack] whose body leaves a ONE-card deck
/// order (`place_remainder_on_deck` over a single revealed card) - a prompt
/// DCGO never records (`answer_forced_single_ordering_prompt`).
const BOTTOMER_YAML: &str = r#"
card: TEST-EOA-BOTTOM
name: Test End of Attack Bottomer
kind: digimon
level: 4
color: [yellow]
cost: 5
dp: 5000
traits: [Test]
effects:
  - scope: inherited
    when: end_of_attack
    summary: "[Inherited][End of Attack] Reveal the top card of your deck and return it to the bottom"
    process:
      - reveal_top_deck: { of: you, count: 1, bind_as: revealed }
      - place_remainder_on_deck: { of: you, position: bottom }
"#;

/// The trigger-order probe meets every row the way `step()` does: DCGO's
/// silent prompts are answered before EACH row, not once after the unstamped
/// answers. Order [bottomer (mandatory), Shishimamon (optional)]; the rows are
/// the index, an unstamped `false`, then a hand pick naming TB-LOW.
///
/// Under the bottomer-first branch the bottomer leaves a forced one-card deck
/// order in front of Shishimamon's gate, so the `false` can only meet that gate
/// once the order is answered - as `step()` does - and declines it; nothing then
/// asks for a hand card. Under Shishimamon-first the `false` declines it at
/// once. So NO order leads to a prompt that takes the TB-LOW row, and the replay
/// must stop. (A probe that skipped the `false` while the forced order was
/// parked met Shishimamon's gate with the TB-LOW row instead, "accepted" it and
/// played TB-LOW: a game the real replay never reaches.)
#[test]
fn order_probe_answers_silent_prompts_before_each_unstamped_answer() {
    let mut runner = DebugRunner::builder()
        .dsl_card(SHISHIMAMON)
        .expect("EX12-046 YAML loads")
        .from_dsl_yaml(BOTTOMER_YAML)
        .expect("bottomer fixture compiles")
        .add_card(digimon("CARRIER", 6, 9000, &[]))
        .add_card(digimon("TB-LOW", 4, 5000, &["Shambala", "TB"]))
        .add_card(digimon("OPP", 4, 6000, &[]))
        .hand(0, &["TB-LOW"])
        .deck(0, &["OPP"; 5])
        .deck(1, &["OPP"; 5])
        .memory(3)
        .start();
    runner.game.turn_count = 1;
    runner.place_stack(1, &["OPP"]);
    let carrier = runner.place_stack(0, &["TEST-EOA-BOTTOM", SHISHIMAMON, "CARRIER"]);
    runner.game.enqueue_triggered(EffectTiming::EndOfAttack, TriggerSource::Permanent(carrier));
    runner.game.drain_effect_queue();
    let bottomer = order_position(&runner.game, "TEST-EOA-BOTTOM");
    let mut s = session(
        runner,
        vec![
            row(serde_json::json!({"step":0,"actor":0,"prompt":"MultipleSkills","int_value":bottomer})),
            row(serde_json::json!({"step":1,"actor":0,"prompt":"generic_bool","bool_value":false,"unstamped":true})),
            row(serde_json::json!({"step":2,"actor":0,"prompt":"SelectHandEffect","zone":"Hand","card_ids":["TB-LOW"]})),
        ],
    );
    s.step();
    assert!(s.is_paused(), "no order leads to a prompt that takes the TB-LOW row");
    assert_eq!(s.current_step(), 0);
    match s.divergences() {
        [d] => assert!(
            matches!(&d.kind, DivergenceKind::SelectionResolution { reason } if reason.contains("no entry")),
            "{d:?}"
        ),
        other => panic!("expected one divergence, got {other:?}"),
    }
    assert!(in_hand(&s.game, 0, "TB-LOW"));
}
