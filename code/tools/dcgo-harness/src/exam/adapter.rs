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
                    lowered.push(classify_lowered_action(&game, actor, action_id, &step.act)?);

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
                LoweredStep::Select(_) => None,
            })
            .collect()
    }

    /// One carrier per scenario step: `Action(id)` or `Select(wire)`.
    pub fn lowered_steps(&self) -> &[LoweredStep] {
        &self.lowered
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
    act: &StepAction,
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
        SelectPayload::Cards(ids) => {
            row.card_ids = Some(ids.clone());
            wire.card_ids = ids.clone();
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
