//! `RecordingSource` over a hand-authored scenario line.
//!
//! The third implementation of the trait, alongside `NativeAdapter` (engine
//! recordings) and `DcgoAdapter` (the oracle). Being a `RecordingSource` rather
//! than a bespoke runner is the single most load-bearing reuse decision in this
//! work: the divergence machinery, step policy, reset-and-replay backward seek,
//! and player-perspective conversion are all inherited, and a scenario run is
//! structurally the same object as a corpus replay.
//!
//! **Lowering happens at construction, not at run time.** `from_scenario` walks
//! the line in a throwaway live game, resolving each symbolic step against that
//! position's mask, so an illegal or ambiguous scenario fails in milliseconds
//! -- before Unity is ever launched. That is the whole reason the exam lowers
//! up front rather than letting each engine interpret the symbolic form itself.

use std::collections::HashMap;

use digimon_engine::dcgo_recording::{FrameTarget, SelectionRow};
use digimon_engine::runners::replay::{RecordingSource, ReplayError, StepPolicy, StepSpec};
use digimon_engine::runners::selection_resolve::{payload_pick_count, resolve_next};
use digimon_engine::selection::SelectionKind;
use digimon_engine::{CardData, Game, PlayerId, Rules};

use crate::exam::lower::{lower_step, LowerError};
use crate::exam::scenario::{Expect, Scenario, SelectPayload, StepAction};

/// Seat that acts first in a scenario game.
///
/// Fixed rather than derived from the seed: `Game::new`'s `seed % 2` pick would
/// otherwise decide which seat a scenario's hard-coded `actor: 0` refers to,
/// making the same YAML mean two different lines depending on the seed. DCGO
/// does not honor a job's `first_player` (a standing phase-1 gap), so the
/// reconciliation between the two sides is the runner's problem, not this
/// adapter's -- but our side must at least be deterministic.
const SCENARIO_FIRST_PLAYER: PlayerId = 0;

/// The wire carrier for one lowered `select:` step — the SYMBOLIC identities
/// the emitted DCGO job will carry (`select_card_ids` / `select_value` /
/// `select_has_bool`+`select_bool` / `select_cancel`), never engine action
/// ids. For `targets:` picks, `card_ids` holds the targeted permanents'
/// TOP-CARD ids resolved against the live game at lowering time: identity
/// matching on DCGO's side dissolves the compact-ordering divergence between
/// the two engines' battle-area orderings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectWire {
    pub card_ids: Vec<String>,
    /// Which of the SAME identity's candidates DCGO should take, 0-based.
    /// Only meaningful for DCGO's `MultipleSkills` prompt (our
    /// `TriggerOrder`), whose candidates are stacked TRIGGERS rather than
    /// interchangeable copies — see `SelectPayload::Cards`. Rides the wire as
    /// `select_ordinal`; DCGO's absent sentinel is `int.MinValue`, expressed
    /// by OMITTING the key.
    ///
    /// It is DCGO's own within-card trigger order, resolved against DCGO's own
    /// candidate list. Our side resolves the same symbolic answer against OUR
    /// candidate list — see [`match_one_with_ordinal`] — so a cross-engine
    /// disagreement about that order surfaces as a divergence rather than
    /// being papered over by one shared index.
    pub ordinal: Option<i32>,
    pub value: Option<i32>,
    pub bool_answer: Option<bool>,
    pub cancel: bool,
    /// task_69f10a66 (ruling item 5, `<Raid>`-family fold): DCGO splits some
    /// optional keyword windows into an `OptionalSkill` yes/no gate FOLLOWED
    /// by the pick, while our engine surfaces ONE declinable pick
    /// (PASS = decline). A select row authored with
    /// `expect: {prompt: OptionalSkill}` over a live non-bool pick prompt
    /// sets this flag: the emitter prepends OptionalSkill(yes) before the
    /// pick row (payload picks = accept), or emits ONLY OptionalSkill(no)
    /// for a `decline:` payload (the pick never opens DCGO-side).
    pub optional_gate_fold: bool,
}

/// The target of an end-of-turn-gate attack, carried symbolically for the
/// DCGO wire (task_69f10a66 Family 1 surface mapping).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EotAttackTarget {
    /// Attack the opponent player / security — DCGO `SelectAttackEffect`
    /// answers this as `select_value: -1`.
    Player,
    /// Attack a defending permanent, identified by its TOP-CARD id resolved
    /// against the live game at lowering time (identity matching on DCGO's
    /// side, same convention as `targets:` picks).
    Permanent { top_card_id: String },
}

/// One lowered scenario step, as the job emitter consumes it: either a
/// concrete 2192-space action id, or a selection answer carried symbolically.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoweredStep {
    Action(u16),
    Select(SelectWire),
    /// task_69f10a66 Family 1 — an action lowered while OUR engine is parked
    /// in `GamePhase::EndOfTurnAction`: the sim surface for the end-of-turn
    /// attack keywords (`<Execute>` — printed OR granted — `<Engage>`,
    /// Vortex, MayAttack). Our engine surfaces the §16-37-3 "may" as the
    /// phase park's mask (attack bits + PASS); DCGO surfaces the SAME choice
    /// as an `OptionalSkill` yes/no followed (on yes) by a
    /// `SelectAttackEffect` target pick. The gates are 1:1 — DCGO's
    /// `CanActivateExecute` requires `CanAttack`, exactly the condition our
    /// `has_end_of_turn_keywords` park requires — so the emitter maps:
    ///   PASS   -> OptionalSkill(select_bool: false)
    ///   attack -> OptionalSkill(select_bool: true) + SelectAttackEffect row
    /// Known limitation: with SEVERAL gate carriers on one board, one PASS
    /// declines our whole park while DCGO asks one OptionalSkill per
    /// carrier; scenarios with more than one end-of-turn attacker must
    /// answer each DCGO gate (extra select rows) explicitly.
    EndOfTurnGate {
        action_id: u16,
        /// `None` = PASS (decline the gate); `Some(t)` = take the attack.
        attack: Option<EotAttackTarget>,
    },
    /// An action our engine needs that has NO DCGO counterpart — emitted as
    /// NOTHING on the wire. Today: the PASS that exits the `EndOfTurnAction`
    /// phase AFTER the last gate was spent (e.g. after the Execute attack
    /// resolved and the carrier self-deleted, our phase still parks until a
    /// PASS, while DCGO simply rotates — no prompt exists to consume a row).
    SimOnlyAction(u16),
    /// The MIRROR of `SimOnlyAction`: a selection DCGO asks that our engine
    /// never parks — emitted to the wire, consumed by NOTHING sim-side.
    ///
    /// DCGO batches same-timing triggers into one `MultipleSkills` prompt and
    /// asks which to resolve first. Where our engine models one of those
    /// triggers as a combat-state window instead of a queued trigger, it opens
    /// only the OTHER pick, so the line is one DCGO answer short and the run
    /// aborts on a prompt mismatch. EX12-076 Susanoomon is the motivating
    /// case: DCGO stacks `<Raid>` (`RaidSelfEffect`, `OnAllyAttack`) beside the
    /// card's own `[When Attacking]` clause in ONE batch, while we open
    /// `AttackState::RaidOpen` separately — and unlike its siblings that clause
    /// NEEDS Raid activatable, so the line cannot dodge the stack by making
    /// Raid's candidate set empty.
    ///
    /// Authored as `dcgo_only: true` on a select step. The row still carries
    /// card IDENTITIES (and an `ordinal` where the stacked candidates share
    /// one id), so DCGO resolves it against its OWN candidate list exactly
    /// like any other select — this widens the vocabulary, it does not weaken
    /// the comparison. The step contributes ZERO comparable state rows, so the
    /// differ counts it as excluded rather than silently mispairing.
    DcgoOnlySelect(SelectWire),
    /// The 2nd..Nth pick of a multi-pick material declaration
    /// (`SelectPayload::Materials`). Our engine asks once per recipe element;
    /// DCGO declares the whole set in ONE row, which the FIRST pick carries.
    /// These contribute a comparable state row on our side and ZERO wire rows,
    /// keeping the two traces aligned across the cardinality mismatch.
    SimOnlySelect,
}

impl LoweredStep {
    /// How many rows this step contributes to the emitted DCGO job.
    ///
    /// This is the SAME dispatch `build_exam_job` performs when it writes the
    /// wire, stated once so the differ can pair the two traces by step instead
    /// of by decision count (see
    /// [`pair_by_wire_rows`](crate::exam::projection::pair_by_wire_rows)).
    /// `emit_job_tests::wire_row_counts_match_dcgo_wire_rows` pins the two
    /// against each other, so a new variant cannot drift them apart.
    pub fn dcgo_wire_rows(&self) -> usize {
        match self {
            // No DCGO prompt exists for it — see the variant's own docs.
            LoweredStep::SimOnlyAction(_) => 0,
            // The inverse: DCGO asks, our engine does not. One wire row.
            LoweredStep::DcgoOnlySelect(_) => 1,
            // A follow-on material pick: ours only, already covered by the
            // first pick's single wire row.
            LoweredStep::SimOnlySelect => 0,
            LoweredStep::Action(_) => 1,
            // A folded DECLINE is one row (DCGO never opens the pick after a
            // declined gate); a folded PICK is the gate row plus the pick row.
            LoweredStep::Select(w) if w.optional_gate_fold => {
                if w.cancel {
                    1
                } else {
                    2
                }
            }
            LoweredStep::Select(_) => 1,
            // Declining the end-of-turn gate is one OptionalSkill row; taking
            // the attack is the gate plus the SelectAttackEffect target pick.
            LoweredStep::EndOfTurnGate { attack: None, .. } => 1,
            LoweredStep::EndOfTurnGate { attack: Some(_), .. } => 2,
        }
    }
}

#[derive(Debug)]
pub struct ScenarioAdapter {
    steps: Vec<StepSpec>,
    lowered: Vec<LoweredStep>,
    deck_p0: Vec<String>,
    deck_p1: Vec<String>,
    seed: u64,
    first_player: PlayerId,
    /// Owned pool, so `relay_initial_state` (which gets no `card_data`
    /// argument) can rebuild the game. `VerificationReplayAdapter` does the
    /// same for the same reason.
    card_data: HashMap<String, CardData>,
}

impl ScenarioAdapter {
    pub fn from_scenario(
        s: &Scenario,
        deck_p0: Vec<String>,
        deck_p1: Vec<String>,
        card_data: &HashMap<String, CardData>,
    ) -> Result<ScenarioAdapter, String> {
        let first_player = SCENARIO_FIRST_PLAYER;

        // Lowering must walk the line in a live game, because each step's legal
        // set depends on every step before it. A one-shot pass over a static
        // position could only ever lower step 0.
        let mut game = construct(&deck_p0, &deck_p1, card_data, s.seed, first_player)
            .map_err(|e| format!("scenario game setup failed: {e}"))?;

        let mut steps = Vec::with_capacity(s.steps.len());
        let mut lowered = Vec::with_capacity(s.steps.len());

        for (i, step) in s.steps.iter().enumerate() {
            let actor = step.actor as PlayerId;
            match &step.act {
                // MULTI-PICK MATERIAL DECLARATION ([Assembly] / [DigiXros]).
                //
                // Cardinality, not prompt kind, is what differs here: our
                // engine re-installs a `Material` prompt per recipe element
                // (`install_assembly_element`), DCGO declares the whole set in
                // ONE row. So this answers N live prompts in element order and
                // contributes ONE wire row, carrying every id — the mirror of
                // `optional_gate_fold` (one sim prompt, two wire rows).
                //
                // Each element is answered by IDENTITY through the ordinary
                // single-pick path, so a wrong id fails loudly with the usual
                // "not among the offered candidates" error rather than being
                // silently assigned to another slot.
                StepAction::Select(SelectPayload::Materials(ids)) => {
                    for (n, id) in ids.iter().enumerate() {
                        if game.pending_selection.is_none() {
                            return Err(format!(
                                "step {i}: `materials:` declares {} cards but our engine                                  stopped asking after {n}. Either the recipe needs fewer                                  elements than listed, or an earlier pick already completed                                  the declaration.",
                                ids.len()
                            ));
                        }
                        let one = SelectPayload::Cards {
                            ids: vec![id.clone()],
                            ordinal: None,
                        };
                        // `expect` describes the DECLARATION, so it is checked
                        // once against the first element's prompt.
                        let expect_for_this = if n == 0 { step.expect.as_ref() } else { None };
                        let (row, mut wire) =
                            build_selection_row(&game, i, actor, &one, expect_for_this)?;
                        if n == 0 {
                            check_select_expectations(&game, i, step.expect.as_ref())?;
                            // The single wire row carries the WHOLE declaration.
                            wire.card_ids = ids.clone();
                        }
                        steps.push(StepSpec {
                            actor,
                            action_id: 0,
                            phase: game.current_phase.py_name().to_string(),
                            source: "scenario".to_string(),
                            memory_after: None,
                            dcgo_memory: None,
                            turn: Some(game.turn_count as u64),
                            is_game_over: None,
                            expected_digest: None,
                            selection: Some(row.clone()),
                            board_p0: None,
                            board_p1: None,
                        });
                        lowered.push(if n == 0 {
                            LoweredStep::Select(wire)
                        } else {
                            LoweredStep::SimOnlySelect
                        });
                        advance_through_selection(&mut game, i, actor, &row)?;
                    }
                }
                // A DCGO-ONLY row: emit the wire answer, touch nothing here.
                //
                // It contributes NO StepSpec, so our trace gains no row for the
                // differ to pair -- which is the point. The `dcgo_wire_rows`
                // count (1) is what keeps the two traces aligned across it.
                //
                // Guard rail: our engine must NOT have a live prompt at this
                // point. If it does, the author has mislabelled a real shared
                // decision as DCGO-only, and skipping it here would leave that
                // prompt unanswered and desync every later step.
                StepAction::Select(payload) => {
                    let (row, wire) =
                        build_selection_row(&game, i, actor, payload, step.expect.as_ref())?;
                    check_select_expectations(&game, i, step.expect.as_ref())?;
                    steps.push(StepSpec {
                        actor,
                        // Placeholder: the replay driver branches on
                        // `selection` BEFORE it ever reads `action_id`, and
                        // the job emitter reads the SelectWire, never this.
                        action_id: 0,
                        phase: game.current_phase.py_name().to_string(),
                        source: "scenario".to_string(),
                        memory_after: None,
                        dcgo_memory: None,
                        turn: Some(game.turn_count as u64),
                        is_game_over: None,
                        expected_digest: None,
                        selection: Some(row.clone()),
                        board_p0: None,
                        board_p1: None,
                    });
                    lowered.push(LoweredStep::Select(wire));
                    advance_through_selection(&mut game, i, actor, &row)?;
                }
                StepAction::SelectDcgoOnly(payload) => {
                    if let Some(pending) = game.pending_selection.as_ref() {
                        return Err(format!(
                            "step {i}: `dcgo_only: true` but OUR engine has a live                              {:?} prompt here. A DCGO-only row is for a decision only                              DCGO asks; this one is shared, so answer it as a normal                              `select:` step.",
                            pending.kind
                        ));
                    }
                    let wire = build_dcgo_only_wire(payload)?;
                    lowered.push(LoweredStep::DcgoOnlySelect(wire));
                }
                _ => {
                    let action_id = lower_step(&game, actor, &step.act).map_err(|e| match e {
                        LowerError::NoMatch { intent, legal } => format!(
                            "step {i}: no legal action matches {intent}\n  legal here:\n    {}",
                            legal.join("\n    ")
                        ),
                        LowerError::Ambiguous { intent, matches } => format!(
                            "step {i}: {intent} is ambiguous -- {matches:?} all match. \
                             Narrow the step; picking arbitrarily would silently answer \
                             a different question than the scenario asks."
                        ),
                    })?;

                    steps.push(StepSpec {
                        actor,
                        action_id,
                        phase: game.current_phase.py_name().to_string(),
                        source: "scenario".to_string(),
                        memory_after: None,
                        dcgo_memory: None,
                        turn: Some(game.turn_count as u64),
                        is_game_over: None,
                        expected_digest: None,
                        selection: None,
                        board_p0: None,
                        board_p1: None,
                    });
                    lowered.push(classify_lowered_action(&game, actor, action_id)?);

                    // `Game::decode_action` returns unit and SILENTLY IGNORES an
                    // illegal or out-of-range id, so there is no error to propagate
                    // from the apply itself. The mask check below is what makes a bad
                    // lowering loud instead of silent; without it a scenario could
                    // "run" while every step after the first was a no-op.
                    let mask = digimon_engine::action::mask::build_action_mask(&game, actor);
                    if mask[action_id as usize] != 1.0 {
                        return Err(format!(
                            "step {i}: lowered action {action_id} is not in the mask"
                        ));
                    }
                    game.decode_action(action_id, actor);
                }
            }

            // The pending-selection invariant, both directions of the exam's
            // prompt-mismatch finding. If our engine parked a prompt, the very
            // next step must answer it -- any other step would lower against a
            // position the author never saw (the selection mask), and on the
            // DCGO side the same prompt would sit unanswered until the job
            // timeout, indistinguishable from a hung Unity.
            if let Some(pending) = game.pending_selection.as_ref() {
                let answered_next = matches!(
                    s.steps.get(i + 1).map(|n| &n.act),
                    Some(StepAction::Select(_))
                );
                if !answered_next {
                    return Err(format!(
                        "step {i}: our engine asks a selection here; the scenario must \
                         answer it with a `select:` step (pending kind: {:?}, prompt: '{}'){}",
                        pending.kind,
                        pending.prompt,
                        if i + 1 == s.steps.len() {
                            " -- the line ends while the prompt is still parked"
                        } else {
                            ""
                        }
                    ));
                }
            }
        }

        Ok(ScenarioAdapter {
            steps,
            lowered,
            deck_p0,
            deck_p1,
            seed: s.seed,
            first_player,
            card_data: card_data.clone(),
        })
    }

    /// The action ids of the non-select steps, in line order. Select steps are
    /// carried symbolically and have no single action id -- see
    /// [`Self::lowered_steps`] for the full per-step carrier.
    pub fn lowered_action_ids(&self) -> Vec<u16> {
        self.lowered
            .iter()
            .filter_map(|l| match l {
                LoweredStep::Action(id) => Some(*id),
                LoweredStep::EndOfTurnGate { action_id, .. } => Some(*action_id),
                LoweredStep::SimOnlyAction(id) => Some(*id),
                LoweredStep::Select(_)
                | LoweredStep::DcgoOnlySelect(_)
                | LoweredStep::SimOnlySelect => None,
            })
            .collect()
    }

    /// One carrier per scenario step: `Action(id)` or `Select(wire)`.
    pub fn lowered_steps(&self) -> &[LoweredStep] {
        &self.lowered
    }

    /// How many DCGO wire rows each scenario step consumes, in line order —
    /// the input the differ's step pairing is derived from.
    pub fn dcgo_wire_rows_per_step(&self) -> Vec<usize> {
        self.lowered.iter().map(LoweredStep::dcgo_wire_rows).collect()
    }
}

/// The unambiguous `SelectionKind` -> DCGO prompt-class mappings, for the
/// LOOSE sim-side `expect.prompt` check on select steps. Kinds whose DCGO
/// class depends on context (`Target` may be `SelectAttackEffect` or
/// `SelectPermanentEffect`; `EffectChoice` may be `OptionalSkill`,
/// `MultipleSkills`, or `SelectCountEffect`; ...) return `None` and are left
/// unasserted here with a printed note -- the STRICT assertion is DCGO's job,
/// where the real prompt class is in hand.
fn dcgo_prompt_name(kind: &SelectionKind) -> Option<&'static str> {
    match kind {
        SelectionKind::Hand => Some("SelectHandEffect"),
        SelectionKind::OwnField | SelectionKind::OppField | SelectionKind::AnyField => {
            Some("SelectPermanentEffect")
        }
        SelectionKind::Trash | SelectionKind::Reveal => Some("SelectCardEffect"),
        _ => None,
    }
}

/// True when our live prompt is a pick-shaped selection that DCGO may split
/// into `OptionalSkill` + pick (the `<Raid>`-family fold — see
/// `SelectWire::optional_gate_fold`).
fn kind_is_pick_shaped(kind: &SelectionKind) -> bool {
    matches!(
        kind,
        SelectionKind::Hand
            | SelectionKind::OwnField
            | SelectionKind::OppField
            | SelectionKind::AnyField
            | SelectionKind::Trash
            | SelectionKind::Reveal
            | SelectionKind::Material
    )
}

/// Sim-side `expect.prompt` on a select step: assert loosely against the
/// parked prompt's kind where the kind->DCGO mapping is unambiguous, and say
/// so out loud where it is not.
fn check_select_expectations(game: &Game, i: usize, expect: Option<&Expect>) -> Result<(), String> {
    let Some(want) = expect.and_then(|e| e.prompt.as_deref()) else {
        return Ok(());
    };
    let Some(pending) = game.pending_selection.as_ref() else {
        println!(
            "  note: step {i} expect.prompt '{want}' cannot be asserted sim-side -- our \
             engine auto-resolved the prompt; DCGO will assert it strictly"
        );
        return Ok(());
    };
    // task_69f10a66 (ruling item 5): `expect.prompt: OptionalSkill` over a
    // live PICK prompt is the declared OptionalSkill+pick FOLD, not a
    // mismatch — DCGO gates the same choice behind a yes/no first (e.g.
    // `<Raid>`'s isOptional gate before its switch-target pick), while our
    // engine's single declinable pick IS both the gate and the pick.
    if want == "OptionalSkill" && kind_is_pick_shaped(&pending.kind) {
        println!(
            "  note: step {i} folds DCGO's OptionalSkill gate into our live {:?} pick \
             (the emitter splits the wire rows)",
            pending.kind
        );
        return Ok(());
    }
    match dcgo_prompt_name(&pending.kind) {
        Some(name) if name == want => Ok(()),
        Some(name) => Err(format!(
            "step {i}: expect.prompt is '{want}' but our engine's parked {:?} prompt \
             maps to DCGO '{name}'",
            pending.kind
        )),
        None => {
            println!(
                "  note: step {i} expect.prompt '{want}' not asserted sim-side (kind {:?} \
                 has no unambiguous DCGO prompt mapping); DCGO will assert it strictly",
                pending.kind
            );
            Ok(())
        }
    }
}

/// Classify a lowered action for the wire (task_69f10a66 Family 1). An
/// action lowered while our engine is parked in `EndOfTurnAction` is the
/// end-of-turn attack-keyword gate — DCGO's surface for the same choice is
/// `OptionalSkill` (+ `SelectAttackEffect` on yes), so the emitter must not
/// pass the raw action id through. Everything else stays `Action(id)`.
fn classify_lowered_action(
    game: &Game,
    actor: PlayerId,
    action_id: u16,
) -> Result<LoweredStep, String> {
    use digimon_engine::action::explain::{explain_action, ActionKind, ActionZone};
    if game.current_phase != digimon_engine::enums::GamePhase::EndOfTurnAction {
        return Ok(LoweredStep::Action(action_id));
    }
    // The park belongs to the turn player; a step someone else answers at
    // this phase is not the gate.
    if actor != game.turn_player() {
        return Ok(LoweredStep::Action(action_id));
    }
    let e = explain_action(game, actor, action_id);
    match e.kind {
        ActionKind::Pass => {
            // A PASS while a gate is still live declines it — DCGO's
            // OptionalSkill answered "no". A PASS with NO live gate (the
            // phase-exit after the last end-of-turn attack was already
            // spent) has no DCGO counterpart: DCGO rotates on its own, so
            // the row must not reach the wire at all.
            if game.has_end_of_turn_keywords(actor) {
                Ok(LoweredStep::EndOfTurnGate {
                    action_id,
                    attack: None,
                })
            } else {
                Ok(LoweredStep::SimOnlyAction(action_id))
            }
        }
        ActionKind::Attack => {
            let attack = match e.target_zone {
                Some(ActionZone::Security) => EotAttackTarget::Player,
                Some(ActionZone::Battle) => {
                    let slot = e.target_index.ok_or_else(|| {
                        "end-of-turn attack explanation names a battle target with no slot"
                            .to_string()
                    })? as usize;
                    let defender = (1 - actor) as usize;
                    let top_card_id = game
                        .player(defender as PlayerId)
                        .battle_area
                        .get(slot)
                        .map(|p| p.top_card().card_id(&game.card_data).to_string())
                        .ok_or_else(|| {
                            format!(
                                "end-of-turn attack targets opponent slot {slot}, but the \
                                 opponent has {} permanent(s)",
                                game.player(defender as PlayerId).battle_area.len()
                            )
                        })?;
                    EotAttackTarget::Permanent { top_card_id }
                }
                other => {
                    return Err(format!(
                        "end-of-turn attack explanation has unexpected target zone {other:?}"
                    ))
                }
            };
            Ok(LoweredStep::EndOfTurnGate {
                action_id,
                attack: Some(attack),
            })
        }
        // Any other kind at this phase (Overclock cost picks etc.) keeps the
        // raw id — extending the wire mapping for those surfaces is future
        // work, and passing the id through is at worst the pre-mapping
        // behavior, not a regression.
        _ => Ok(LoweredStep::Action(action_id)),
    }
}

/// Resolve one card identity (+ optional ordinal) against a prompt's candidate
/// list, returning the index of the pick.
///
/// A faithful mirror of DCGO's `SelectionAnswer.MatchOneWithOrdinal`, applied
/// to OUR candidate list. Both engines answer the same symbolic step by
/// resolving it themselves; neither is handed the other's index.
///
/// The rule, and why each branch refuses instead of guessing:
/// - **0 matches** — the identity is not on offer. Something upstream already
///   diverged; taking any candidate would bury it.
/// - **1 match** — resolves with no ordinal. An ordinal other than 0 is an
///   author claiming a second trigger that does not exist here, which is a
///   finding about the stack's SHAPE and must not be silently rounded down.
/// - **N matches** — an ordinal is REQUIRED. This is the one prompt where
///   duplicate ids are not interchangeable copies: an `[On Deletion]` and an
///   `<Ascension>` on one deleted carrier both offer that carrier's identity,
///   and taking the first would be a confident wrong answer.
fn match_one_with_ordinal(
    wanted: &str,
    ordinal: Option<i32>,
    candidates: &[String],
) -> Result<usize, String> {
    let matches: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.as_str() == wanted)
        .map(|(i, _)| i)
        .collect();
    match matches.len() {
        0 => Err(format!(
            "wanted card '{wanted}' is not among the offered candidates [{}]",
            candidates.join(", ")
        )),
        1 => match ordinal {
            None | Some(0) => Ok(matches[0]),
            Some(o) => Err(format!(
                "wanted card '{wanted}' with ordinal {o}, but it is offered exactly \
                 once by [{}] (only ordinal 0 exists)",
                candidates.join(", ")
            )),
        },
        n => match ordinal {
            None => Err(format!(
                "wanted card '{wanted}' is AMBIGUOUS: it is offered {n} times by \
                 [{}]. Add `ordinal:`, the 0-based position among that card's own \
                 candidates (0..{})",
                candidates.join(", "),
                n - 1
            )),
            Some(o) if o < 0 || o as usize >= n => Err(format!(
                "wanted card '{wanted}' with ordinal {o}, but it is offered {n} times \
                 by [{}] (valid ordinals 0..{})",
                candidates.join(", "),
                n - 1
            )),
            Some(o) => Ok(matches[o as usize]),
        },
    }
}

/// The source-card ids our live `TriggerOrder` prompt is offering, in ITS own
/// order — the candidate list [`match_one_with_ordinal`] resolves against.
///
/// `None` when any entry carries no source card: that is NOT MEASURED, and an
/// unverifiable match is a wrong answer waiting to happen. Resolved through
/// `Game::card` rather than by parsing the branch label, so a label-format
/// change cannot silently break identity matching.
fn trigger_order_candidate_ids(game: &Game) -> Option<Vec<String>> {
    let pending = game.pending_selection.as_ref()?;
    let entries = pending.effect_choices.as_ref()?;
    entries
        .iter()
        .map(|e| {
            e.source_card
                .map(|h| game.card(h).card_id.clone())
        })
        .collect()
}

/// Resolve one `own.field.N` / `opp.field.N` slot reference against the live
/// game, returning both halves the brief requires: the `FrameTarget` our own
/// `resolve_next` consumes, and the targeted permanent's TOP-CARD id for the
/// wire (DCGO matches identities against ITS candidate list).
fn resolve_target_ref(
    game: &Game,
    actor: PlayerId,
    reference: &str,
) -> Result<(FrameTarget, String), String> {
    let parts: Vec<&str> = reference.split('.').collect();
    let err = |why: &str| {
        format!(
            "target '{reference}': {why} (expected `own.field.N` or `opp.field.N`)"
        )
    };
    if parts.len() != 3 || parts[1] != "field" {
        return Err(err("not a field slot reference"));
    }
    let player: PlayerId = match parts[0] {
        "own" => actor,
        "opp" => 1 - actor,
        _ => return Err(err("side must be `own` or `opp`")),
    };
    let slot: usize = parts[2]
        .parse()
        .map_err(|_| err("slot index is not a number"))?;
    let perm = game.player(player).battle_area.get(slot).ok_or_else(|| {
        format!(
            "target '{reference}': player {player} has {} battle-area permanent(s), \
             none at slot {slot}",
            game.player(player).battle_area.len()
        )
    })?;
    let top = perm.top_card().card_id(&game.card_data).to_string();
    Ok((
        FrameTarget {
            player: player as u8,
            frame: slot as i32,
        },
        top,
    ))
}

/// Build the `SelectionRow` (for our own resolve path) and the `SelectWire`
/// (for the emitted DCGO job) from one symbolic select payload, resolved
/// against the CURRENT game state at lowering time.
/// Build the wire answer for a DCGO-ONLY select row.
///
/// Unlike [`build_selection_row`] this resolves NOTHING against our game: by
/// definition our engine has no prompt here, so there is no candidate list to
/// match against and no `SelectionRow` to record. The identities (and
/// `ordinal`) ride the wire verbatim for DCGO to resolve against its own
/// candidates.
///
/// `targets:` is refused: it is expressed as OUR slot references
/// (`own.field.N`), which are resolved at lowering time against our live game
/// -- exactly the thing that does not exist here. Answer a DCGO-only row by
/// card identity, or by `value:` for a raw count/int prompt.
fn build_dcgo_only_wire(payload: &SelectPayload) -> Result<SelectWire, String> {
    let mut wire = SelectWire::default();
    if let SelectPayload::Materials(_) = payload {
        return Err("select `materials:` cannot be combined with `dcgo_only: true`: a material                     declaration is a SHARED decision both engines make (ours as N element                     prompts, DCGO's as one row), so it is never DCGO-only."
            .to_string());
    }
    match payload {
        SelectPayload::Cards { ids, ordinal } => {
            wire.card_ids = ids.clone();
            wire.ordinal = *ordinal;
        }
        SelectPayload::Value(v) => wire.value = Some(*v),
        SelectPayload::Yes => wire.bool_answer = Some(true),
        SelectPayload::Decline => {
            wire.bool_answer = Some(false);
            wire.cancel = true;
        }
        // Refused above with a dedicated message.
        SelectPayload::Materials(_) => unreachable!("materials + dcgo_only refused above"),
        SelectPayload::Targets(_) => {
            return Err(
                "select `targets:` cannot be used with `dcgo_only: true`: slot references                  (own.field.N / opp.field.N) are resolved against OUR live game, which by                  definition has no prompt on a DCGO-only row. Use `cards: [ID]` (plus                  `ordinal:` when the stacked candidates share an id), or `value: N`."
                    .to_string(),
            );
        }
    }
    Ok(wire)
}

fn build_selection_row(
    game: &Game,
    i: usize,
    actor: PlayerId,
    payload: &SelectPayload,
    expect: Option<&Expect>,
) -> Result<(SelectionRow, SelectWire), String> {
    if let Some(pending) = game.pending_selection.as_ref() {
        if pending.selecting_player != actor {
            return Err(format!(
                "step {i}: the select step is authored for actor {actor}, but our \
                 engine's parked {:?} prompt belongs to player {}",
                pending.kind, pending.selecting_player
            ));
        }
    }

    let mut row = SelectionRow {
        step: i as u32,
        actor: actor as u8,
        prompt: expect
            .and_then(|e| e.prompt.clone())
            .or_else(|| {
                game.pending_selection
                    .as_ref()
                    .and_then(|p| dcgo_prompt_name(&p.kind))
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "select".to_string()),
        phase: game.current_phase.py_name().to_string(),
        targets: None,
        card_ids: None,
        indexes: None,
        count: None,
        candidates: None,
        int_value: None,
        bool_value: None,
        cancel: None,
        board_p0: None,
        board_p1: None,
        memory: None,
        mechanic: None,
        zone: None,
    };
    let mut wire = SelectWire::default();

    // task_69f10a66 (ruling item 5): mark the OptionalSkill+pick fold —
    // authored as `expect: {prompt: OptionalSkill}` over a live pick-shaped
    // prompt. See `SelectWire::optional_gate_fold` / the emitter's split.
    if expect.and_then(|e| e.prompt.as_deref()) == Some("OptionalSkill") {
        if let Some(pending) = game.pending_selection.as_ref() {
            if kind_is_pick_shaped(&pending.kind) {
                wire.optional_gate_fold = true;
            }
        }
    }

    match payload {
        // Never reaches here: the adapter's step loop expands a multi-pick
        // material declaration into one single-pick call per recipe element,
        // because our engine asks once per element. An internal invariant, so
        // it fails loudly rather than silently answering one element.
        SelectPayload::Materials(_) => {
            return Err(
                "internal: `materials:` must be expanded by the adapter's multi-pick loop                  before build_selection_row"
                    .to_string(),
            );
        }
        SelectPayload::Cards { ids, ordinal } => {
            // The wire always carries the SYMBOLIC answer — identities plus,
            // when the prompt stacks a card's triggers, which of them.
            wire.card_ids = ids.clone();
            wire.ordinal = *ordinal;

            let live_kind = game.pending_selection.as_ref().map(|p| p.kind.clone());
            match live_kind {
                // Our trigger-order prompt is DCGO's MultipleSkills. Its
                // candidates are stacked TRIGGERS, so `resolve_next`'s
                // card-identity path (which searches game ZONES) cannot answer
                // it. Resolve the identity here, against the prompt's own
                // source cards, with DCGO's own matching rule.
                Some(SelectionKind::TriggerOrder) => {
                    if ids.len() != 1 {
                        return Err(format!(
                            "step {i}: our TriggerOrder prompt is single-pick but the \
                             step names {} cards",
                            ids.len()
                        ));
                    }
                    let candidates = trigger_order_candidate_ids(game).ok_or_else(|| {
                        format!(
                            "step {i}: our TriggerOrder prompt offers a branch with no \
                             source card, so its candidate list is NOT MEASURED and \
                             identities cannot be matched"
                        )
                    })?;
                    let pick = match_one_with_ordinal(&ids[0], *ordinal, &candidates)
                        .map_err(|e| format!("step {i}: {e}"))?;
                    println!(
                        "  note: step {i} answers our TriggerOrder prompt by identity -- \
                         '{}'{} is branch {pick} of [{}]. That order is OURS; DCGO \
                         resolves the same step against its own list, so a \
                         disagreement surfaces as a divergence.",
                        ids[0],
                        ordinal
                            .map(|o| format!(" ordinal {o}"))
                            .unwrap_or_default(),
                        candidates.join(", ")
                    );
                    // `int_value` is the branch-index payload `resolve_next`
                    // maps straight through `effect_choices`.
                    row.int_value = Some(pick as i64);
                }
                // An ordinal over any other live prompt is an author using
                // MultipleSkills vocabulary on a prompt that has no trigger
                // stack. Our side would ignore it while DCGO acted on it --
                // a half-answered step, which is exactly the silent
                // desynchronization this format refuses everywhere else.
                Some(kind) if ordinal.is_some() => {
                    return Err(format!(
                        "step {i}: `ordinal:` disambiguates a stacked-trigger prompt \
                         (DCGO MultipleSkills / our TriggerOrder), but our engine's \
                         live prompt here is {kind:?}"
                    ))
                }
                _ => {
                    // No live prompt (our engine auto-resolved one DCGO still
                    // asks about), or an ordinary identity pick: unchanged.
                    row.card_ids = Some(ids.clone());
                    if ordinal.is_some() && game.pending_selection.is_none() {
                        println!(
                            "  note: step {i} carries `ordinal:` but our engine parks no \
                             prompt here -- it rides the wire for DCGO and is not \
                             checked sim-side"
                        );
                    }
                }
            }
        }
        SelectPayload::Targets(refs) => {
            let mut frames = Vec::with_capacity(refs.len());
            let mut tops = Vec::with_capacity(refs.len());
            for r in refs {
                let (frame, top) =
                    resolve_target_ref(game, actor, r).map_err(|e| format!("step {i}: {e}"))?;
                frames.push(frame);
                tops.push(top);
            }
            row.targets = Some(frames);
            wire.card_ids = tops;
        }
        SelectPayload::Value(v) => {
            row.count = Some(*v);
            wire.value = Some(*v);
        }
        SelectPayload::Yes => {
            row.bool_value = Some(true);
            wire.bool_answer = Some(true);
        }
        SelectPayload::Decline => {
            row.cancel = Some(true);
            wire.cancel = true;
        }
    }
    Ok((row, wire))
}

/// Advance the lowering game through one selection row with the SAME resolver
/// the replay driver uses (`resolve_next` + `decode_action` until `Ok(None)`),
/// so later steps lower against the post-selection state. Reused, not
/// reimplemented: a second resolution path here would let lowering and replay
/// disagree about what the row means.
fn advance_through_selection(
    game: &mut Game,
    i: usize,
    actor: PlayerId,
    row: &SelectionRow,
) -> Result<(), String> {
    if game.pending_selection.is_none() {
        // `resolve_next` returns `Ok(None)` immediately here: the engine
        // auto-resolved a prompt DCGO still asks about. Allowed -- the row is
        // kept for the wire, and the replay driver skips it the same way.
        println!(
            "  note: step {i} select answered no live prompt -- our engine auto-resolved \
             it; the row is kept for the DCGO wire"
        );
    }
    let mut picks_done = 0usize;
    loop {
        match resolve_next(game, row, picks_done) {
            Ok(None) => return Ok(()),
            Ok(Some(id)) => {
                game.decode_action(id, actor);
                picks_done += 1;
                // Same safety valve as the replay driver: a payload cannot
                // sanely resolve into more actions than picks + trailing PASS.
                if picks_done > payload_pick_count(row) + 1 {
                    return Err(format!(
                        "step {i}: selection resolution ran away ({picks_done} engine \
                         actions for a {}-pick payload)",
                        payload_pick_count(row)
                    ));
                }
            }
            Err(e) => {
                return Err(format!(
                    "step {i}: the select payload cannot be resolved against our \
                     engine's prompt: {e}"
                ))
            }
        }
    }
}

/// The one place a scenario game is constructed, shared by lowering and by the
/// `RecordingSource` build/relay hooks.
///
/// Both sides MUST start from an identical position or the lowered ids stop
/// meaning what they meant when they were resolved, so this is deliberately a
/// single function rather than two similar-looking call sites.
fn construct(
    deck_p0: &[String],
    deck_p1: &[String],
    card_data: &HashMap<String, CardData>,
    seed: u64,
    first_player: PlayerId,
) -> Result<Game, String> {
    let mut game = Game::new_with_ordered_decks(
        &[deck_p0.to_vec(), deck_p1.to_vec()],
        card_data,
        Rules::standard(),
        Some(seed),
        first_player,
    )?;
    // Resolve both mulligans (keep) and enter turn 1. The scenario step
    // vocabulary has no mulligan verb -- on the DCGO side the mulligan is its
    // own recorder row type, not one of the scripted prompts -- so a scenario
    // line always begins after the mulligan on both sides.
    game.start_game();
    Ok(game)
}

impl RecordingSource for ScenarioAdapter {
    fn build_initial_game(
        &self,
        _card_data: &HashMap<String, CardData>,
    ) -> Result<Game, ReplayError> {
        construct(
            &self.deck_p0,
            &self.deck_p1,
            &self.card_data,
            self.seed,
            self.first_player,
        )
        .map_err(ReplayError::GameConstruction)
    }

    fn relay_initial_state(&self, game: &mut Game) -> Result<(), ReplayError> {
        // A scenario has no post-mulligan snapshot to re-lay: the game is
        // rebuilt deterministically from (decks, seed, first_player), so
        // reset-and-replay just reconstructs it.
        *game = self.build_initial_game(&self.card_data)?;
        Ok(())
    }

    fn steps(&self) -> &[StepSpec] {
        &self.steps
    }

    fn default_policy(&self) -> StepPolicy {
        // Trust: this line came from our own mask, so there is nothing to check
        // it against at this layer. The oracle comparison is the differ's job,
        // over state projections.
        StepPolicy::Trust
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exam::scenario::Scenario;
    use crate::exam::test_support;
    use digimon_engine::runners::replay::ReplaySession;

    const LINE: &str = r#"
card: ST1-02
clause: ST1-02#effect#0
seed: 7
decks:
  p0: { stack: [], rest: simple }
  p1: { stack: [], rest: simple }
steps:
  - actor: 0
    do: { pass: {} }
"#;

    /// The stock ST-1 list (50 main + 4 egg), in printed order.
    ///
    /// Mirrors `lower.rs`'s helper rather than adding a shared fixture: a
    /// tournament-legal list matters because DCGO gates battles on
    /// `DeckData.IsValidDeckData()`, so a scenario meant to mirror DCGO cannot
    /// use an ad-hoc deck.
    fn simple_deck() -> Vec<String> {
        let (mut main, egg) = test_support::st1_decks();
        main.extend(egg);
        main
    }

    #[test]
    fn adapter_builds_a_game_and_lowers_the_line() {
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data)
            .expect("adapter should build");
        assert_eq!(a.lowered_action_ids().len(), 1);
        assert_eq!(a.steps().len(), 1);
    }

    #[test]
    fn adapter_default_policy_is_trust() {
        // Our engine generated this line itself, so there is no oracle to check
        // it against at this layer. The DCGO comparison happens in the differ,
        // over state projections -- not by re-checking our own actions.
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap();
        assert_eq!(a.default_policy(), StepPolicy::Trust);
    }

    #[test]
    fn session_runs_the_line_to_completion() {
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap();
        let mut session = ReplaySession::with_source(Box::new(a), &card_data, false)
            .expect("session should build");
        session.run_to_completion();
        assert!(session.is_complete());
        assert!(
            session.divergences().is_empty(),
            "{:?}",
            session.divergences()
        );
    }

    // ── select steps ────────────────────────────────────────────────────

    /// The ST-1 deck reordered so `prefix` is the top-first deal/draw order:
    /// prefix[0..5] is the opening hand, prefix[5..10] the security stack,
    /// prefix[10..] the draws — the same deal order the EX12 scenarios probed
    /// empirically. `Player::draw` pops the END of the deck vector, so the
    /// prefix is appended reversed (mirrors `ordered_deck` in main.rs).
    fn st1_deck_stacked(prefix: &[&str]) -> Vec<String> {
        let (mut main, egg) = test_support::st1_decks();
        for id in prefix {
            let pos = main
                .iter()
                .position(|c| c == id)
                .unwrap_or_else(|| panic!("{id} not left in the ST-1 main deck"));
            main.remove(pos);
        }
        main.extend(egg);
        main.extend(prefix.iter().rev().map(|s| s.to_string()));
        main
    }

    /// A real ST-1 line whose step 4 (play ST1-15 Giga Destroyer, "[Main]
    /// Delete up to 2 of your opponent's Digimon with 4000 DP or less") parks
    /// a field selection over p1's ST1-03 — the real-card fixture for the
    /// whole `select:` path.
    ///
    /// Probed empirically (2026-08-22): p0 must have a RED card in its battle
    /// area before ST1-15 becomes playable (the breeding-area Lv2 alone did
    /// not satisfy the option color gate), hence the turn-1 ST1-02 play; and
    /// p0's turn-3 Breeding phase is auto-skipped by the engine (the occupied
    /// breeding area has no legal hatch/move), so the line goes straight from
    /// p1's ST1-03 play to p0's Main. The cost-2 / cost-3 plays each push
    /// memory across zero, ending those turns without explicit passes.
    const SELECT_LINE: &str = r#"
card: ST1-15
clause: ST1-15#effect#0
seed: 11
decks:
  p0: { stack: [], rest: st1 }
  p1: { stack: [], rest: st1 }
steps:
  - actor: 0
    do: { hatch: {} }
  - actor: 0
    do: { play: { card: ST1-02, from: hand } }
  - actor: 1
    do: { pass: {} }
  - actor: 1
    do: { play: { card: ST1-03, from: hand } }
  - actor: 0
    do: { play: { card: ST1-15, from: hand } }
  - actor: 0
    do: { select: { targets: [opp.field.0] } }
    expect: { prompt: SelectPermanentEffect }
"#;

    /// Deterministic opening hands + draws for SELECT_LINE: exactly one copy
    /// of each played card in its seat's hand, so every intent lowers
    /// unambiguously.
    fn select_line_decks() -> (Vec<String>, Vec<String>) {
        let p0 = st1_deck_stacked(&[
            "ST1-15", "ST1-02", "ST1-04", "ST1-05", "ST1-06", // hand
            "ST1-13", "ST1-13", "ST1-14", "ST1-14", "ST1-12", // security
            "ST1-02", // p0's turn-3 draw — NOT a second ST1-15
        ]);
        let p1 = st1_deck_stacked(&[
            "ST1-03", "ST1-04", "ST1-05", "ST1-06", "ST1-07", // hand
            "ST1-09", "ST1-09", "ST1-12", "ST1-12", "ST1-13", // security
            "ST1-08", // p1's turn-2 draw — NOT a second ST1-03
        ]);
        (p0, p1)
    }

    #[test]
    fn a_select_step_lowers_through_a_real_selection() {
        let card_data = test_support::load_card_data();
        let (p0, p1) = select_line_decks();
        let s = Scenario::from_yaml(SELECT_LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, p0, p1, &card_data)
            .expect("the select line should lower");

        // The select step rides the replay wire as a semantic SelectionRow…
        let spec = &a.steps()[5];
        let row = spec.selection.as_ref().expect("step 5 carries a selection");
        let targets = row.targets.as_deref().expect("targets present");
        assert_eq!(targets.len(), 1);
        assert_eq!(
            (targets[0].player, targets[0].frame),
            (1, 0),
            "opp.field.0 for actor 0 is (player 1, slot 0)"
        );

        // …and the wire carrier holds the targeted permanent's TOP-CARD id,
        // resolved at lowering time — the identity DCGO matches against its
        // own candidate list.
        match &a.lowered_steps()[5] {
            LoweredStep::Select(w) => {
                assert_eq!(w.card_ids, vec!["ST1-03".to_string()]);
                assert_eq!(w.value, None);
                assert_eq!(w.bool_answer, None);
                assert!(!w.cancel);
            }
            other => panic!("expected a Select carrier, got {other:?}"),
        }
        // 5 action steps, 1 select step.
        assert_eq!(a.lowered_action_ids().len(), 5);

        // The full session replays the same line through the shared replay
        // core with no divergence.
        let mut session = ReplaySession::with_source(Box::new(a), &card_data, false)
            .expect("session should build");
        session.run_to_completion();
        assert!(session.is_complete());
        assert!(
            session.divergences().is_empty(),
            "{:?}",
            session.divergences()
        );
        // The selection actually resolved: Agumon was deleted.
        assert!(
            session.game.player(1).battle_area.is_empty(),
            "the selected Agumon should have been deleted"
        );
    }

    #[test]
    fn an_unanswered_selection_fails_lowering_loudly() {
        // Drop the select step: our engine parks the ST1-15 prompt and the
        // line just… ends. That must be a lowering failure naming the pending
        // kind — on the DCGO side the same prompt would sit unanswered until
        // the job timeout, indistinguishable from a hung Unity.
        let text = SELECT_LINE.replace(
            "  - actor: 0\n    do: { select: { targets: [opp.field.0] } }\n    expect: { prompt: SelectPermanentEffect }",
            "",
        );
        let card_data = test_support::load_card_data();
        let (p0, p1) = select_line_decks();
        let s = Scenario::from_yaml(&text).unwrap();
        let err = ScenarioAdapter::from_scenario(&s, p0, p1, &card_data).unwrap_err();
        assert!(
            err.contains("our engine asks a selection here"),
            "got: {err}"
        );
        assert!(err.contains("pending kind"), "must name the kind: {err}");
    }

    #[test]
    fn a_wrong_expect_prompt_on_a_select_step_fails_lowering() {
        // The ST1-15 prompt is a field-permanent pick; its kind maps
        // unambiguously to DCGO's SelectPermanentEffect, so claiming
        // SelectHandEffect is a sim-side-checkable authoring error.
        let text = SELECT_LINE.replace(
            "expect: { prompt: SelectPermanentEffect }",
            "expect: { prompt: SelectHandEffect }",
        );
        let card_data = test_support::load_card_data();
        let (p0, p1) = select_line_decks();
        let s = Scenario::from_yaml(&text).unwrap();
        let err = ScenarioAdapter::from_scenario(&s, p0, p1, &card_data).unwrap_err();
        assert!(err.contains("SelectHandEffect"), "got: {err}");
        assert!(err.contains("SelectPermanentEffect"), "got: {err}");
    }

    #[test]
    fn a_select_step_with_no_live_prompt_is_an_allowed_auto_resolve() {
        // A `select:` answering a prompt our engine never parks (it
        // auto-resolves some prompts DCGO still asks about) is allowed: the
        // row is kept for the wire and skipped locally. `resolve_next`
        // returns Ok(None) immediately, which is the documented contract.
        let text = LINE.replace(
            "  - actor: 0\n    do: { pass: {} }",
            "  - actor: 0\n    do: { pass: {} }\n  - actor: 0\n    do: { select: { decline: true } }",
        );
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(&text).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data)
            .expect("auto-resolved select must not fail lowering");
        assert_eq!(a.steps().len(), 2);
        assert!(a.steps()[1].selection.is_some(), "the row rides the wire");
        assert!(matches!(a.lowered_steps()[1], LoweredStep::Select(_)));
    }

    // ── identity + ordinal matching (mirror of DCGO MatchOneWithOrdinal) ─

    #[test]
    fn one_candidate_resolves_with_or_without_ordinal_zero() {
        let c = vec!["A".to_string(), "B".to_string()];
        assert_eq!(match_one_with_ordinal("B", None, &c).unwrap(), 1);
        assert_eq!(match_one_with_ordinal("B", Some(0), &c).unwrap(), 1);
    }

    #[test]
    fn a_nonzero_ordinal_on_a_single_candidate_is_a_finding_not_a_round_down() {
        // The author claims a second trigger from this card. If the stack does
        // not have one, that is a disagreement about the stack's SHAPE, and
        // quietly taking the only candidate would bury it.
        let c = vec!["A".to_string(), "B".to_string()];
        let err = match_one_with_ordinal("B", Some(1), &c).unwrap_err();
        assert!(err.contains("exactly once"), "got: {err}");
        assert!(err.contains("only ordinal 0"), "got: {err}");
    }

    #[test]
    fn duplicate_candidates_require_an_ordinal_and_name_the_valid_range() {
        // The live case: one deleted carrier offering its [On Deletion] and
        // its <Ascension>. Those are different decisions.
        let c = vec!["EX12-047".to_string(), "EX12-047".to_string()];
        let err = match_one_with_ordinal("EX12-047", None, &c).unwrap_err();
        assert!(err.contains("AMBIGUOUS"), "got: {err}");
        assert!(err.contains("0..1"), "must name the range: {err}");

        assert_eq!(match_one_with_ordinal("EX12-047", Some(0), &c).unwrap(), 0);
        assert_eq!(match_one_with_ordinal("EX12-047", Some(1), &c).unwrap(), 1);
    }

    #[test]
    fn an_ordinal_is_a_position_among_that_cards_candidates_not_a_list_index() {
        // [X, Y, X]: ordinal 1 for X is the THIRD entry, not the second.
        let c = vec!["X".to_string(), "Y".to_string(), "X".to_string()];
        assert_eq!(match_one_with_ordinal("X", Some(0), &c).unwrap(), 0);
        assert_eq!(match_one_with_ordinal("X", Some(1), &c).unwrap(), 2);
    }

    #[test]
    fn an_out_of_range_ordinal_refuses_rather_than_wrapping() {
        let c = vec!["A".to_string(), "A".to_string()];
        let err = match_one_with_ordinal("A", Some(2), &c).unwrap_err();
        assert!(err.contains("valid ordinals 0..1"), "got: {err}");
    }

    #[test]
    fn an_absent_identity_refuses_rather_than_taking_any_candidate() {
        let c = vec!["A".to_string(), "B".to_string()];
        let err = match_one_with_ordinal("Z", None, &c).unwrap_err();
        assert!(err.contains("not among the offered candidates"), "got: {err}");
        assert!(err.contains("A, B"), "must show the list: {err}");
    }

    #[test]
    fn an_ordinal_over_a_non_trigger_prompt_fails_lowering() {
        // The ST1-15 gate parks a field-permanent pick -- no trigger stack, so
        // an ordinal addresses nothing on our side while DCGO would act on it.
        // A half-answered step is exactly the silent desynchronization this
        // format refuses everywhere else.
        let text = SELECT_LINE.replace(
            "do: { select: { targets: [opp.field.0] } }",
            "do: { select: { cards: [ST1-03], ordinal: 1 } }",
        );
        let card_data = test_support::load_card_data();
        let (p0, p1) = select_line_decks();
        let s = Scenario::from_yaml(&text).unwrap();
        let err = ScenarioAdapter::from_scenario(&s, p0, p1, &card_data).unwrap_err();
        assert!(err.contains("ordinal"), "got: {err}");
        assert!(err.contains("TriggerOrder"), "must name the prompt it belongs to: {err}");
    }

    #[test]
    fn a_cards_pick_without_an_ordinal_still_lowers_the_old_way() {
        // Backward compat: every already-authored `cards:` step must keep
        // resolving through `resolve_next`'s zone search.
        let text = SELECT_LINE.replace(
            "  - actor: 0\n    do: { select: { targets: [opp.field.0] } }\n    expect: { prompt: SelectPermanentEffect }",
            "  - actor: 0\n    do: { select: { targets: [opp.field.0] } }\n    expect: { prompt: SelectPermanentEffect }\n  - actor: 0\n    do: { select: { cards: [ST1-04] } }",
        );
        let card_data = test_support::load_card_data();
        let (p0, p1) = select_line_decks();
        let s = Scenario::from_yaml(&text).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, p0, p1, &card_data)
            .expect("a plain cards: pick must still lower");
        match &a.lowered_steps()[6] {
            LoweredStep::Select(w) => {
                assert_eq!(w.card_ids, vec!["ST1-04".to_string()]);
                assert_eq!(w.ordinal, None);
            }
            other => panic!("expected a Select carrier, got {other:?}"),
        }
        assert_eq!(
            a.steps()[6].selection.as_ref().unwrap().card_ids.as_deref(),
            Some(["ST1-04".to_string()].as_slice()),
            "the row still carries the identity for resolve_next's zone search"
        );
    }

    // ── wire row counts ─────────────────────────────────────────────────

    #[test]
    fn each_lowered_step_reports_how_many_wire_rows_it_writes() {
        use crate::exam::adapter::{EotAttackTarget, LoweredStep, SelectWire};
        assert_eq!(LoweredStep::Action(62).dcgo_wire_rows(), 1);
        assert_eq!(LoweredStep::SimOnlyAction(62).dcgo_wire_rows(), 0);
        assert_eq!(
            LoweredStep::Select(SelectWire::default()).dcgo_wire_rows(),
            1
        );
        assert_eq!(
            LoweredStep::Select(SelectWire {
                card_ids: vec!["X".to_string()],
                optional_gate_fold: true,
                ..SelectWire::default()
            })
            .dcgo_wire_rows(),
            2,
            "a folded pick is OptionalSkill(yes) + the pick"
        );
        assert_eq!(
            LoweredStep::Select(SelectWire {
                cancel: true,
                optional_gate_fold: true,
                ..SelectWire::default()
            })
            .dcgo_wire_rows(),
            1,
            "a folded decline never opens the pick DCGO-side"
        );
        assert_eq!(
            LoweredStep::EndOfTurnGate {
                action_id: 62,
                attack: None
            }
            .dcgo_wire_rows(),
            1
        );
        assert_eq!(
            LoweredStep::EndOfTurnGate {
                action_id: 100,
                attack: Some(EotAttackTarget::Player)
            }
            .dcgo_wire_rows(),
            2
        );
    }

    #[test]
    fn the_adapter_reports_one_wire_row_count_per_scenario_step() {
        let card_data = test_support::load_card_data();
        let (p0, p1) = select_line_decks();
        let s = Scenario::from_yaml(SELECT_LINE).unwrap();
        let a = ScenarioAdapter::from_scenario(&s, p0, p1, &card_data).unwrap();
        let rows = a.dcgo_wire_rows_per_step();
        assert_eq!(rows.len(), s.steps.len());
        assert_eq!(rows, vec![1, 1, 1, 1, 1, 1], "this line has no folds");
    }

    #[test]
    fn an_illegal_line_fails_to_build_not_at_run_time() {
        // The whole point of lowering up front: a malformed scenario must fail
        // in milliseconds, before any Unity launch.
        let bad = LINE.replace(
            "do: { pass: {} }",
            "do: { play: { card: ZZ99-999, from: hand } }",
        );
        let card_data = test_support::load_card_data();
        let deck = simple_deck();
        let s = Scenario::from_yaml(&bad).unwrap();
        let err =
            ScenarioAdapter::from_scenario(&s, deck.clone(), deck, &card_data).unwrap_err();
        assert!(err.contains("ZZ99-999"), "got: {err}");
    }
}
