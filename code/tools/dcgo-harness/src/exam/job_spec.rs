//! The DCGO scripted job for a lowered exam scenario -- what `exam --emit-job`
//! writes and what `exam --oracle` submits. Moved out of the `dcgo-harness`
//! binary so the CLI and the MCP build jobs through one definition
//! (oracle-readiness Plan 3, Task 1).

use serde::Serialize;

use crate::exam::deckbook::DeckEntry;
use crate::job::JobLimits;

/// A DCGO scripted harness job, field-for-field the shape the modded client's
/// scripted driver reads (see `qa/dcgo-harness/golden-scripted-job.json`).
/// Same core fields as `crate::job::JobSpec` plus the scripted-only
/// `deck_order` / `inputs`; phase-1 readers tolerate the extras.
#[derive(Debug, Serialize)]
pub struct ExamJobSpec {
    pub job_id: String,
    pub policy: String,
    pub decks: crate::job::JobDecks,
    pub deck_order: ExamDeckOrder,
    pub inputs: Vec<ScriptedInput>,
    pub first_player: u8,
    pub seed: u64,
    pub limits: JobLimits,
}

/// The scenario's stack per seat, in DCGO's TOP-FIRST convention — the order
/// the author wrote it in.
#[derive(Debug, Serialize)]
pub struct ExamDeckOrder {
    pub p0: Vec<String>,
    pub p1: Vec<String>,
}

/// One scripted answer: which seat, which lowered action id, and which prompt
/// it expects to be answering (asserted BEFORE answering — see the exam doc).
/// Our prompt vocabulary maps 1:1 onto DCGO's, so the name passes through.
///
/// A `select:` step carries no action id — it answers a selection RPC, not a
/// main-phase/breeding prompt — and instead fills the `select_*` fields, whose
/// names are the C# `HarnessJobStep` contract verbatim (`select_card_ids` /
/// `select_value` / `select_has_bool` + `select_bool` / `select_cancel`; the
/// C# side treats an absent `select_value` as `int.MinValue` via its field
/// initializer, so absence is expressed by OMITTING the key). The values are
/// the scenario's SYMBOLIC identities (card ids, counts, bools), never engine
/// action ids: DCGO resolves them against its own candidate lists.
#[derive(Debug, Default, Serialize)]
pub struct ScriptedInput {
    pub actor: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_prompt: Option<String>,
    /// Expected number of picks. The C# initializer is `-1` = "do not
    /// assert", so absence is expressed by OMITTING the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_count: Option<u16>,
    /// Expected candidate card ids, order-insensitive. Empty = "do not
    /// assert". On a `MultipleSkills` row these are the stacked triggers'
    /// SOURCE-CARD ids, which is the cheapest way to catch "DCGO did not stack
    /// what we stacked".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub expect_candidates: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub select_card_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select_value: Option<i32>,
    /// Which of the SAME identity's candidates to take, 0-based — DCGO's
    /// `select_ordinal`. Deliberately NOT `select_value` reused: that field
    /// stays the raw DCGO-index fallback, and one field meaning "an index into
    /// DCGO's list" in one step and "an index within one card's own triggers"
    /// in the next is the value-space confusion the payload exists to end.
    /// Same `int.MinValue` absent sentinel, so absence OMITS the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select_ordinal: Option<i32>,
    /// Which of ONE card's simultaneous triggers to resolve, named by its
    /// KEYWORD -- the semantic sibling of `select_ordinal`, and the preferred
    /// one. Carries the NORMALIZED name (lowercased, `<`/`>`/whitespace
    /// stripped) so our `<Armor Purge>` and DCGO's `"Armor Purge"` compare
    /// equal; DCGO normalizes its own `ICardEffect.EffectName` the same way.
    ///
    /// Emitted even though no DCGO build reads it yet, and that is deliberate.
    /// Without it a `trigger:` scenario passes `--sim-only` and then aborts on
    /// the oracle: DCGO refuses an ambiguous same-identity stack by telling the
    /// author to "Add select_ordinal" (SelectionAnswer.cs:175-181) -- exactly
    /// the key our own parser refuses to accept alongside `trigger:`. A green
    /// sim gate that hands the author advice they are forbidden to follow is
    /// worse than no key at all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select_trigger: Option<String>,
    /// The branch to EXCLUDE, for a wanted branch with no keyword of its
    /// own -- the complement of `select_trigger`, normalized identically.
    ///
    /// EX12-047 Amaterasumon is the case: its deletion stack is
    /// [Ascension, the printed On Deletion] and only the first is nameable
    /// by keyword. Neither engine can answer "which branch is not a
    /// keyword" without a registry of what counts as one, and DCGO has
    /// none -- no IsKeywordEffect flag, no keyword enum, and Decode's
    /// effect name is parameterized. Both sides CAN drop a named branch
    /// and check that exactly one survives.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select_trigger_not: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub select_has_bool: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub select_bool: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub select_cancel: bool,
    /// The two materials of a DNA digivolution, as their permanents' TOP-CARD
    /// ids in declaration order. Only set on a `main_phase` row whose
    /// `action_id` is in the DNA_DIGIVOLVE range.
    ///
    /// Deliberately NOT `select_card_ids`: DCGO's `HarnessJobStep.IsSelection`
    /// is true whenever that field is non-empty, and a step that reads as a
    /// selection answer arriving at an action-id prompt aborts the job as a
    /// prompt mismatch (`InputDriver.TryAnswer`) -- correctly, for every other
    /// row. A DNA digivolution is ONE action carrying both materials on that
    /// side (`PlayCardAction.JogressEvoRootsFrameIDs`) while our engine asks
    /// for them as two `Material` prompts, so the pair needs a channel of its
    /// own. `HarnessJob.cs`'s `dna_materials` is that channel.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dna_materials: Vec<String>,
    /// The battle-area permanents a main-phase action names, as TOP-CARD ids
    /// -- `HarnessJob.cs`'s `attacker_card_id` / `attack_target_card_id` /
    /// `permanent_card_id` / `digivolve_target_card_id`. Set only on a
    /// `LoweredStep::FieldAction` row: DCGO's compact field is frame-ordered
    /// and ours is play-ordered, so the raw slot in `action_id` can name a
    /// different permanent there (add-card-authoring-loop 10.1).
    /// `InputDriver.BuildMainPhaseAction` resolves these against DCGO's own
    /// field; an older player ignores the unknown keys (`JsonUtility`) and
    /// falls back to the slot. Like `dna_materials`, deliberately NOT
    /// selection fields, so `IsSelection` stays false on the row.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attacker_card_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_target_card_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent_card_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digivolve_target_card_id: Option<String>,
}

/// One seat's flat job deck in DCGO's top-first convention: full main deck
/// (stack first, then the remainder), with the egg deck appended.
///
/// This inverts `decks_from_recording`'s reversal: our engine's deck vector is
/// draw-from-back (`ordered_deck` builds `remainder + stack.rev()`), while a
/// DCGO job lists the deck top-first. Reversing the main portion gives
/// `stack + remainder.rev()` — `stack[0]` on top, exactly what `deck_order`
/// then re-imposes after DCGO's own seeded shuffle.
pub fn job_deck_top_first(
    seat: &crate::exam::scenario::ScenarioSeat,
    entry: &DeckEntry,
    deck_name: &str,
) -> Result<Vec<String>, String> {
    let mut remainder = entry.main.clone();
    for id in &seat.stack {
        match remainder.iter().position(|c| c == id) {
            Some(i) => {
                remainder.remove(i);
            }
            None => {
                return Err(format!(
                    "stacked card {id} is not in the main deck of `{deck_name}` -- \
                     the job would claim a deck the scenario does not use"
                ))
            }
        }
    }
    let mut out: Vec<String> = seat.stack.clone();
    out.extend(remainder.into_iter().rev());
    out.extend(entry.eggs.iter().cloned());
    Ok(out)
}

/// Build the DCGO scripted job for a lowered scenario.
///
/// Refuses a seat whose deck resolves with no egg cards: `Game::new` would
/// play on regardless, but DCGO's breeding phase would be answering over a
/// different game than the one the line was lowered against — the job must
/// say so instead of silently emitting.
pub fn build_exam_job(
    stem: &str,
    s: &crate::exam::scenario::Scenario,
    entry_p0: &DeckEntry,
    entry_p1: &DeckEntry,
    lowered: &[crate::exam::adapter::LoweredStep],
    owners: &[usize],
) -> Result<ExamJobSpec, String> {
    // Rows are paired to scenario steps through `owners`, not positionally: a
    // step whose declaration our engine splits into several decisions
    // (`materials:`, `dna:`) contributes several lowered entries, and a
    // positional zip would silently shift every later row onto the wrong step.
    if owners.len() != lowered.len() {
        return Err(format!(
            "lowered {} row(s) but {} owner(s) -- refusing to emit a \
             desynchronized job",
            lowered.len(),
            owners.len()
        ));
    }
    if let Some(bad) = owners.iter().find(|o| **o >= s.steps.len()) {
        return Err(format!(
            "lowered row claims scenario step {bad}, but the line has {} step(s)",
            s.steps.len()
        ));
    }
    if owners.windows(2).any(|w| w[1] < w[0]) {
        return Err("lowered rows are not in scenario order -- refusing to emit a \
                    desynchronized job"
            .to_string());
    }
    if owners.first().copied().unwrap_or(0) != 0 || owners.last().copied().map(|o| o + 1) != Some(s.steps.len())
    {
        // Every step must have produced at least one row up to the last one;
        // a gap means a step lowered to nothing and the wire is short.
        return Err(format!(
            "lowered rows cover scenario steps {:?}..={:?} of {} -- refusing to \
             emit a desynchronized job",
            owners.first(),
            owners.last(),
            s.steps.len()
        ));
    }
    for (seat_name, seat, entry) in [
        ("p0", &s.decks.p0, entry_p0),
        ("p1", &s.decks.p1, entry_p1),
    ] {
        if entry.eggs.is_empty() {
            return Err(format!(
                "deck `{}` ({seat_name}) has no egg cards resolvable from the deck \
                 book -- refusing to emit a job with no eggs, which would run a \
                 different game than the scenario lowered against",
                seat.rest
            ));
        }
    }

    let inputs = lowered
        .iter()
        .zip(owners)
        .enumerate()
        .flat_map(|(row, (l, owner))| {
            use crate::exam::adapter::{EotAttackTarget, LoweredStep};
            let step = &s.steps[*owner];
            // `expect:` describes the step's FIRST decision. On a step the
            // adapter split into several rows the later ones are follow-on
            // picks with prompts of their own, so asserting the authored
            // `expect` against them would fail on a line that is correct.
            let first_of_step = row == 0 || owners[row - 1] != *owner;
            let expect_prompt = if first_of_step {
                step.expect.as_ref().and_then(|e| e.prompt.clone())
            } else {
                None
            };
            // `expect.count` / `expect.candidates` describe the PICK, so on a
            // step the emitter splits they ride the pick row, never the
            // OptionalSkill gate that precedes it.
            let expect_count = if first_of_step {
                step.expect.as_ref().and_then(|e| e.count)
            } else {
                None
            };
            let expect_candidates = if first_of_step {
                step.expect
                    .as_ref()
                    .map(|e| e.candidates.clone())
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            match l {
                LoweredStep::Action(id) => vec![ScriptedInput {
                    actor: step.actor,
                    action_id: Some(*id),
                    expect_prompt,
                    expect_count,
                    expect_candidates,
                    ..ScriptedInput::default()
                }],
                // The same one `main_phase` row, plus the identities of the
                // battle-area permanents the action names, which DCGO resolves
                // against its own frame-ordered field (see `ScriptedInput`).
                LoweredStep::FieldAction { action_id, refs } => vec![ScriptedInput {
                    actor: step.actor,
                    action_id: Some(*action_id),
                    expect_prompt,
                    expect_count,
                    expect_candidates,
                    attacker_card_id: refs.attacker.clone(),
                    attack_target_card_id: refs.attack_target.clone(),
                    permanent_card_id: refs.permanent.clone(),
                    digivolve_target_card_id: refs.digivolve_target.clone(),
                    ..ScriptedInput::default()
                }],
                // A DNA digivolution is ONE `main_phase` row on DCGO's side --
                // a single `PlayCardAction` carrying `JogressEvoRootsFrameIDs`
                // -- so the action id rides it together with both materials'
                // identities, which `InputDriver.BuildMainPhaseAction` resolves
                // against the actor's live field. Our own two material prompts
                // ride the following `SimOnlySelect` row (zero wire rows).
                LoweredStep::DnaDeclaration {
                    action_id,
                    material_ids,
                } => vec![ScriptedInput {
                    actor: step.actor,
                    action_id: Some(*action_id),
                    expect_prompt,
                    expect_count,
                    expect_candidates,
                    dna_materials: material_ids.clone(),
                    ..ScriptedInput::default()
                }],
                // task_69f10a66 (ruling item 5) — the OptionalSkill+pick
                // FOLD: DCGO gates some optional keyword windows (<Raid>,
                // …) behind an OptionalSkill yes/no BEFORE the pick, while
                // our engine surfaces one declinable pick. A folded row
                // (authored `expect: {prompt: OptionalSkill}` over the live
                // pick) splits on the wire:
                //   picks   -> OptionalSkill(yes) + the pick row
                //   decline -> OptionalSkill(no) only (DCGO never opens the
                //              pick after a declined gate).
                LoweredStep::Select(w) if w.optional_gate_fold => {
                    if w.cancel {
                        vec![ScriptedInput {
                            actor: step.actor,
                            action_id: None,
                            expect_prompt: Some("OptionalSkill".to_string()),
                            select_has_bool: true,
                            select_bool: !w.cancel,
                            ..ScriptedInput::default()
                        }]
                    } else {
                        vec![
                            ScriptedInput {
                                actor: step.actor,
                                action_id: None,
                                expect_prompt: Some("OptionalSkill".to_string()),
                                select_has_bool: true,
                                select_bool: true,
                                ..ScriptedInput::default()
                            },
                            ScriptedInput {
                                actor: step.actor,
                                action_id: None,
                                // The pick's own DCGO prompt class varies
                                // (SelectPermanentEffect / SelectHandEffect /
                                // SelectCardEffect …) — leave it unasserted
                                // rather than guess wrong.
                                expect_prompt: None,
                                expect_count,
                                expect_candidates,
                                select_card_ids: w.card_ids.clone(),
                                select_value: w.value,
                                select_ordinal: w.ordinal,
                                select_trigger: w.trigger.clone(),
                                select_trigger_not: w.trigger_not.clone(),
                                ..ScriptedInput::default()
                            },
                        ]
                    }
                }
                // A DCGO-ONLY row emits exactly like a plain select — DCGO
                // cannot tell the difference, and must not: it resolves the
                // identities against its own candidate list as usual. The
                // asymmetry is entirely on our side, where the step consumes
                // no live prompt.
                LoweredStep::Select(w) | LoweredStep::DcgoOnlySelect(w) => vec![ScriptedInput {
                    actor: step.actor,
                    action_id: None,
                    expect_prompt,
                    expect_count,
                    expect_candidates,
                    select_card_ids: w.card_ids.clone(),
                    select_value: w.value,
                    select_ordinal: w.ordinal,
                    select_trigger: w.trigger.clone(),
                    select_trigger_not: w.trigger_not.clone(),
                    select_has_bool: w.bool_answer.is_some(),
                    select_bool: w.bool_answer.unwrap_or(false),
                    select_cancel: w.cancel,
                    dna_materials: Vec::new(),
                    attacker_card_id: None,
                    attack_target_card_id: None,
                    permanent_card_id: None,
                    digivolve_target_card_id: None,
                }],
                // task_69f10a66 Family 1 surface mapping: our EndOfTurnAction
                // phase park (the §16-37-3 "may attack at end of turn" for
                // printed/granted <Execute>, <Engage>, Vortex, MayAttack) is
                // DCGO's OptionalSkill gate (+ SelectAttackEffect on yes).
                //   PASS   -> one row: OptionalSkill answered "no".
                //   attack -> two rows: OptionalSkill "yes", then the
                //             SelectAttackEffect target pick (permanent
                //             targets by top-card id, the player as
                //             select_value -1 — the SelectAttackEffect.cs
                //             harness contract).
                // A sim-side-only action (e.g. the PASS that exits our
                // EndOfTurnAction park after the last gate was spent) —
                // DCGO has no prompt for it, so it contributes NO wire row.
                LoweredStep::SimOnlyAction(_) => Vec::new(),
                // A follow-on material pick: ours only. The FIRST pick's row
                // already carries every declared id for DCGO.
                LoweredStep::SimOnlySelect => vec![],
                LoweredStep::EndOfTurnGate { attack, .. } => {
                    let gate_prompt =
                        Some(expect_prompt.unwrap_or_else(|| "OptionalSkill".to_string()));
                    match attack {
                        None => vec![ScriptedInput {
                            actor: step.actor,
                            action_id: None,
                            expect_prompt: gate_prompt,
                            select_has_bool: true,
                            ..ScriptedInput::default()
                        }],
                        Some(target) => {
                            let (ids, value) = match target {
                                EotAttackTarget::Player => (Vec::new(), Some(-1)),
                                EotAttackTarget::Permanent { top_card_id } => {
                                    (vec![top_card_id.clone()], None)
                                }
                            };
                            vec![
                                ScriptedInput {
                                    actor: step.actor,
                                    action_id: None,
                                    expect_prompt: gate_prompt,
                                    select_has_bool: true,
                                    select_bool: true,
                                    ..ScriptedInput::default()
                                },
                                ScriptedInput {
                                    actor: step.actor,
                                    action_id: None,
                                    expect_prompt: Some("SelectAttackEffect".to_string()),
                                    expect_count,
                                    expect_candidates,
                                    select_card_ids: ids,
                                    select_value: value,
                                    ..ScriptedInput::default()
                                },
                            ]
                        }
                    }
                }
            }
        })
        .collect();

    Ok(ExamJobSpec {
        job_id: format!("exam-{stem}"),
        policy: "scripted".to_string(),
        decks: crate::job::JobDecks {
            p0: job_deck_top_first(&s.decks.p0, entry_p0, &s.decks.p0.rest)?,
            p1: job_deck_top_first(&s.decks.p1, entry_p1, &s.decks.p1.rest)?,
        },
        deck_order: ExamDeckOrder {
            p0: s.decks.p0.stack.clone(),
            p1: s.decks.p1.stack.clone(),
        },
        inputs,
        // The adapter lowers every scenario with seat 0 acting first
        // (`SCENARIO_FIRST_PLAYER`), and DCGO honours the job's first_player.
        first_player: 0,
        seed: s.seed,
        limits: JobLimits {
            max_turns: 40,
            // The player fails a job at this limit itself (DCGO fork,
            // 2026-10-09), so it has to clear every healthy exam: of 355
            // completed since 2026-10-01 the slowest took 209 s, and 240 s is
            // what the host already tolerated (the old 180 s plus its stall
            // grace). A line wedged on a prompt fails in ~10 s regardless.
            timeout_seconds: 240,
        },
    })
}

#[cfg(test)]
mod emit_job_tests {
    use super::*;
    use crate::exam::adapter::{EotAttackTarget, FieldRefs, LoweredStep, SelectWire};
    use crate::exam::scenario::Scenario;

    /// Shorthand: a lowered line of plain action ids.
    fn actions(ids: &[u16]) -> Vec<LoweredStep> {
        ids.iter().map(|id| LoweredStep::Action(*id)).collect()
    }

    const LINE: &str = r#"
card: ST1-12
clause: ST1-12#effect#0
seed: 424242
decks:
  p0: { stack: [B, D], rest: tiny }
  p1: { stack: [], rest: tiny }
steps:
  - actor: 0
    do: { pass: {} }
    expect: { prompt: breeding_action }
  - actor: 0
    do: { pass: {} }
    expect: { prompt: main_phase }
  - actor: 1
    do: { pass: {} }
"#;

    fn entry() -> DeckEntry {
        DeckEntry {
            main: vec!["A", "B", "C", "D"].into_iter().map(String::from).collect(),
            eggs: vec!["E1".to_string()],
        }
    }

    /// `build_exam_job` for the ordinary 1-row-per-step case: owners are the
    /// identity mapping. The expanding steps (`materials:`, `dna:`) are the
    /// exception and carry their own owner vectors.
    fn build_job_1to1(
        stem: &str,
        s: &crate::exam::scenario::Scenario,
        e0: &DeckEntry,
        e1: &DeckEntry,
        lowered: &[crate::exam::adapter::LoweredStep],
    ) -> Result<ExamJobSpec, String> {
        let owners: Vec<usize> = (0..lowered.len()).collect();
        build_exam_job(stem, s, e0, e1, lowered, &owners)
    }

    #[test]
    fn deck_is_reversed_back_to_top_first_with_eggs_appended() {
        // Our draw-from-back vector for this seat would be
        // [A, C, D, B] (remainder [A, C] + stack [B, D] reversed), so the
        // top-first job deck must be [B, D, C, A] -- stack first, remainder
        // reversed -- with the egg deck appended after the main deck.
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_job_1to1("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        assert_eq!(job.decks.p0, vec!["B", "D", "C", "A", "E1"]);
        // No stack: the whole main deck reversed, then eggs.
        assert_eq!(job.decks.p1, vec!["D", "C", "B", "A", "E1"]);
        assert_eq!(job.deck_order.p0, vec!["B", "D"]);
        assert!(job.deck_order.p1.is_empty());
    }

    #[test]
    fn inputs_carry_actor_action_id_and_expect_prompt() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_job_1to1("ST1-12", &s, &entry(), &entry(), &actions(&[62, 63, 64])).unwrap();
        assert_eq!(job.inputs.len(), 3);
        assert_eq!(job.inputs[0].actor, 0);
        assert_eq!(job.inputs[0].action_id, Some(62));
        assert_eq!(job.inputs[0].expect_prompt.as_deref(), Some("breeding_action"));
        assert_eq!(job.inputs[1].expect_prompt.as_deref(), Some("main_phase"));
        assert_eq!(job.inputs[2].actor, 1);
        assert_eq!(job.inputs[2].expect_prompt, None);
    }

    #[test]
    fn job_identity_fields_come_from_the_scenario() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_job_1to1("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        assert_eq!(job.job_id, "exam-ST1-12");
        assert_eq!(job.policy, "scripted");
        assert_eq!(job.seed, 424242);
        assert_eq!(job.first_player, 0);
    }

    #[test]
    fn a_deck_with_no_resolvable_eggs_is_refused() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let eggless = DeckEntry {
            main: entry().main,
            eggs: vec![],
        };
        let err = build_job_1to1("ST1-12", &s, &eggless, &entry(), &actions(&[62, 62, 62])).unwrap_err();
        assert!(err.contains("no egg cards"), "got: {err}");
        assert!(err.contains("tiny"), "must name the deck: {err}");
    }

    #[test]
    fn a_desynchronized_lowering_is_refused() {
        // 3 steps but only 2 lowered ids: emitting would hand DCGO a line that
        // answers the wrong prompts from the first mismatch onward.
        let s = Scenario::from_yaml(LINE).unwrap();
        let err = build_job_1to1("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62])).unwrap_err();
        assert!(err.contains("desynchronized"), "got: {err}");
    }

    #[test]
    fn serialized_job_matches_the_golden_field_names() {
        // The DCGO reader is the consumer; these exact key names are the
        // contract (see qa/dcgo-harness/golden-scripted-job.json).
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_job_1to1("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        for key in [
            "job_id",
            "policy",
            "decks",
            "deck_order",
            "inputs",
            "first_player",
            "seed",
            "limits",
        ] {
            assert!(v.get(key).is_some(), "missing top-level key {key}");
        }
        assert_eq!(v["inputs"][0]["expect_prompt"], "breeding_action");
        assert_eq!(v["inputs"][0]["action_id"], 62);
        assert_eq!(v["limits"]["max_turns"], 40);
        // A step without `expect:` must OMIT the key, not write null -- the
        // driver treats presence as "assert this prompt".
        assert!(v["inputs"][2].get("expect_prompt").is_none());
        // A non-select step must not wear ANY selection field: on the C# side
        // `IsSelection` keys off their presence, and a stray one would turn an
        // action step into a selection answer.
        for key in [
            "select_card_ids",
            "select_value",
            "select_has_bool",
            "select_bool",
            "select_cancel",
        ] {
            assert!(
                v["inputs"][0].get(key).is_none(),
                "action step must omit {key}"
            );
        }
    }

    // ── select steps on the wire ────────────────────────────────────────
    //
    // The field names below are the C# `HarnessJobStep` contract verbatim
    // (read from `Assets/Scripts/Script/Harness/HarnessJob.cs`, DCGO
    // 9bbc7e5f3): `select_card_ids` / `select_value` (absent = int.MinValue
    // via the field initializer) / `select_has_bool` + `select_bool` /
    // `select_cancel`. The values are symbolic identities from the scenario,
    // never engine action ids.

    /// LINE with its final pass replaced by a select step carrying `args`.
    fn select_line(args: &str) -> Scenario {
        let text = LINE.replace(
            "  - actor: 1\n    do: { pass: {} }",
            &format!("  - actor: 1\n    do: {{ select: {args} }}"),
        );
        Scenario::from_yaml(&text).expect("select line parses")
    }

    fn job_json(s: &Scenario, lowered: &[LoweredStep]) -> serde_json::Value {
        let job = build_job_1to1("ST1-12", s, &entry(), &entry(), lowered).unwrap();
        serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap()
    }

    #[test]
    fn select_card_ids_ride_the_wire_and_action_id_is_omitted() {
        let s = select_line("{ cards: [ST1-03, ST1-03] }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["ST1-03".to_string(), "ST1-03".to_string()],
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(
            v["inputs"][2]["select_card_ids"],
            serde_json::json!(["ST1-03", "ST1-03"])
        );
        // A selection step answers a selection RPC, not a 2192-space prompt;
        // an action id here would be an engine-internal leak.
        assert!(v["inputs"][2].get("action_id").is_none());
        assert_eq!(v["inputs"][2]["actor"], 1);
    }

    #[test]
    fn select_value_and_bool_and_cancel_use_the_csharp_field_names() {
        let s = select_line("{ value: 3 }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            value: Some(3),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_value"], 3);
        assert!(v["inputs"][2].get("select_card_ids").is_none());
        assert!(v["inputs"][2].get("select_has_bool").is_none());

        let s = select_line("{ yes: true }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            bool_answer: Some(true),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_has_bool"], true);
        assert_eq!(v["inputs"][2]["select_bool"], true);
        // Absent select_value must be OMITTED (the C# initializer, not 0, is
        // the absent sentinel -- writing 0 would claim a count answer of 0).
        assert!(v["inputs"][2].get("select_value").is_none());

        let s = select_line("{ decline: true }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            cancel: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_cancel"], true);
        assert!(v["inputs"][2].get("select_bool").is_none());
    }

    // ── select_ordinal + the expect_* assertions on the wire ────────────

    #[test]
    fn select_ordinal_rides_the_wire_under_the_csharp_field_name() {
        let s = select_line("{ cards: [EX12-047], ordinal: 1 }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ordinal: Some(1),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_card_ids"], serde_json::json!(["EX12-047"]));
        assert_eq!(v["inputs"][2]["select_ordinal"], 1);
        // `select_value` stays the raw DCGO-index fallback and must NOT be
        // written: the C# hook aborts on value+card_ids by design.
        assert!(v["inputs"][2].get("select_value").is_none());
    }

    /// The whole point of `trigger:` is that it survives the trip to DCGO. It
    /// did NOT at first: `SelectWire::trigger` existed and `--sim-only` went
    /// green, but `ScriptedInput` had no `select_trigger`, so the key was
    /// dropped at the job boundary and the scenario aborted on the oracle --
    /// DCGO refuses an ambiguous same-identity stack with "Add select_ordinal"
    /// (SelectionAnswer.cs:175-181), the one key our parser forbids next to
    /// `trigger:`. A sim gate that passes and then strands the author on the
    /// oracle is worse than no key, so pin the serialization itself.
    #[test]
    fn select_trigger_rides_the_wire_under_the_csharp_field_name() {
        let s = select_line("{ cards: [EX12-065], trigger: fortitude }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-065".to_string()],
            trigger: Some("fortitude".to_string()),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["select_card_ids"], serde_json::json!(["EX12-065"]));
        assert_eq!(v["inputs"][2]["select_trigger"], "fortitude");
        // `trigger:` and `ordinal:` are mutually exclusive by construction, so
        // a trigger row must never also carry the positional disambiguator.
        assert!(v["inputs"][2].get("select_ordinal").is_none());
    }

    /// An absent trigger OMITS the key -- the C# reads absence as "not given",
    /// so writing `null` would be a different statement.
    #[test]
    fn an_absent_trigger_is_omitted_from_the_wire() {
        let s = select_line("{ cards: [EX12-047] }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert!(v["inputs"][2].get("select_trigger").is_none());
    }

    #[test]
    fn ordinal_zero_is_written_and_an_absent_ordinal_is_omitted() {
        // 0 is a real answer ("the FIRST of that card's triggers"), and the
        // C# absent sentinel is int.MinValue -- so 0 must serialize while
        // absent must omit the key entirely.
        let s = select_line("{ cards: [EX12-047], ordinal: 0 }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ordinal: Some(0),
            ..SelectWire::default()
        }));
        assert_eq!(job_json(&s, &lowered)["inputs"][2]["select_ordinal"], 0);

        let s = select_line("{ cards: [EX12-047] }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ..SelectWire::default()
        }));
        assert!(job_json(&s, &lowered)["inputs"][2]
            .get("select_ordinal")
            .is_none());
    }

    /// add-card-authoring-loop 10.1: a `FieldAction` row carries its action id
    /// AND the named battle-area permanents' identities under the C#
    /// `HarnessJobStep` field names, so DCGO resolves them against its own
    /// frame-ordered field instead of trusting the raw slot.
    #[test]
    fn a_field_action_carries_identities_beside_its_action_id() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::FieldAction {
            action_id: 100,
            refs: FieldRefs {
                attacker: Some("ST1-03".to_string()),
                attack_target: Some("ST1-02".to_string()),
                ..FieldRefs::default()
            },
        });
        let v = job_json(&s, &lowered);
        let row = &v["inputs"][2];
        assert_eq!(row["action_id"], 100);
        assert_eq!(row["attacker_card_id"], "ST1-03");
        assert_eq!(row["attack_target_card_id"], "ST1-02");
        assert!(row.get("permanent_card_id").is_none());
        assert!(row.get("digivolve_target_card_id").is_none());
        // `HarnessJobStep.IsSelection` keys off the select_* fields: a field
        // action must still read as an action-id answer on the C# side.
        assert!(row.get("select_card_ids").is_none());
        assert!(row.get("dna_materials").is_none());
    }

    #[test]
    fn a_plain_action_wears_no_field_identities() {
        let s = Scenario::from_yaml(LINE).unwrap();
        let v = job_json(&s, &actions(&[62, 62, 62]));
        for key in ["attacker_card_id", "attack_target_card_id", "permanent_card_id", "digivolve_target_card_id"] {
            assert!(v["inputs"][0].get(key).is_none(), "{key} leaked onto a plain action row");
        }
    }

    #[test]
    fn an_action_step_wears_no_ordinal() {
        // On the C# side `IsSelection` keys off the selection fields'
        // presence, and `select_ordinal` is one of them -- a stray one would
        // turn an action step into a selection answer.
        let s = Scenario::from_yaml(LINE).unwrap();
        let v: serde_json::Value = serde_json::from_str(
            &serde_json::to_string(
                &build_job_1to1("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(v["inputs"][0].get("select_ordinal").is_none());
    }

    #[test]
    fn expect_count_and_expect_candidates_ride_the_wire() {
        // Previously parsed and then DROPPED. On a MultipleSkills row the
        // candidate set is the stacked TRIGGERS' source-card ids, which is the
        // cheapest available check that DCGO stacked what we stacked.
        let text = LINE.replace(
            "  - actor: 1\n    do: { pass: {} }",
            "  - actor: 1\n    do: { select: { cards: [EX12-047], ordinal: 1 } }\n    expect: { prompt: MultipleSkills, count: 1, candidates: [EX12-047, EX12-047] }",
        );
        let s = Scenario::from_yaml(&text).expect("parses");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["EX12-047".to_string()],
            ordinal: Some(1),
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["expect_prompt"], "MultipleSkills");
        assert_eq!(v["inputs"][2]["expect_count"], 1);
        assert_eq!(
            v["inputs"][2]["expect_candidates"],
            serde_json::json!(["EX12-047", "EX12-047"])
        );
    }

    #[test]
    fn absent_expectations_omit_their_keys() {
        // The C# initializers (`expect_count = -1`, `expect_candidates =
        // new string[0]`) ARE the "do not assert" defaults, so writing 0 / []
        // would turn "no opinion" into a live assertion.
        let s = Scenario::from_yaml(LINE).unwrap();
        let job = build_job_1to1("ST1-12", &s, &entry(), &entry(), &actions(&[62, 62, 62])).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&job).unwrap()).unwrap();
        assert!(v["inputs"][0].get("expect_count").is_none());
        assert!(v["inputs"][0].get("expect_candidates").is_none());
    }

    #[test]
    fn a_folded_pick_carries_the_expectations_on_the_pick_row_not_the_gate() {
        // `expect.count` / `expect.candidates` describe the PICK. Putting them
        // on the OptionalSkill gate would assert a candidate list against a
        // yes/no prompt that has none.
        let text = LINE.replace(
            "  - actor: 1\n    do: { pass: {} }",
            "  - actor: 1\n    do: { select: { targets: [opp.field.0] } }\n    expect: { prompt: OptionalSkill, count: 1, candidates: [ST1-07] }",
        );
        let s = Scenario::from_yaml(&text).expect("parses");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["ST1-07".to_string()],
            optional_gate_fold: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 4);
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert!(v["inputs"][2].get("expect_count").is_none(), "gate row must not assert the pick");
        assert!(v["inputs"][2].get("expect_candidates").is_none());
        assert_eq!(v["inputs"][3]["expect_count"], 1);
        assert_eq!(v["inputs"][3]["expect_candidates"], serde_json::json!(["ST1-07"]));
    }

    // ── task_69f10a66 surface mappings on the wire ──────────────────────

    /// The end-of-turn attack-keyword gate (our `EndOfTurnAction` park; DCGO
    /// OptionalSkill + SelectAttackEffect). PASS = one OptionalSkill "no"
    /// row; an attack = OptionalSkill "yes" + the SelectAttackEffect answer
    /// (permanent by top-card id / the player as select_value -1); a
    /// sim-only phase-exit pass = NO wire row at all.
    #[test]
    fn end_of_turn_gate_maps_to_optional_skill_rows() {
        // Decline: one OptionalSkill(no) row.
        let s = select_line("{ cards: [ST1-03] }"); // 3-step line; payloads below drive the shape
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::EndOfTurnGate {
            action_id: 62,
            attack: None,
        });
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_has_bool"], true);
        assert!(v["inputs"][2].get("select_bool").is_none()); // false is skip-serialized
        assert!(v["inputs"][2].get("action_id").is_none());

        // Accept + attack a permanent: OptionalSkill(yes) then the pick.
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::EndOfTurnGate {
            action_id: 100,
            attack: Some(EotAttackTarget::Permanent {
                top_card_id: "ST1-07".to_string(),
            }),
        });
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 4, "one step -> two rows");
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_bool"], true);
        assert_eq!(v["inputs"][3]["expect_prompt"], "SelectAttackEffect");
        assert_eq!(v["inputs"][3]["select_card_ids"], serde_json::json!(["ST1-07"]));

        // Accept + attack the player: select_value -1 on the pick row.
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::EndOfTurnGate {
            action_id: 113,
            attack: Some(EotAttackTarget::Player),
        });
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"][3]["select_value"], -1);

        // Sim-only phase exit: contributes NOTHING to the wire.
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::SimOnlyAction(62));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 2, "no third row");
    }

    /// The differ pairs the two traces by LOWERED STEP, using
    /// `LoweredStep::dcgo_wire_rows()` to know how many rows each step
    /// consumes. That number is only trustworthy if it agrees with what this
    /// emitter actually writes -- if they ever drift, the pairing slides and
    /// manufactures divergences out of an offset, which is precisely the bug
    /// the pairing exists to fix. So: pin them against each other, per
    /// variant, over the real emitter.
    #[test]
    fn wire_row_counts_match_dcgo_wire_rows() {
        let s = select_line("{ cards: [ST1-03] }");
        let variants: Vec<LoweredStep> = vec![
            LoweredStep::Action(62),
            // A `link:` step is a plain main-phase action on the wire: the
            // FIELD_EFFECT link sub-slot (slot 0 -> 1003). Its host pick is
            // the NEXT scenario step's own row, never folded into this one,
            // so it must count exactly like any other Action.
            LoweredStep::Action(
                digimon_engine::action::space::FIELD_EFFECT_START
                    + digimon_engine::action::space::FIELD_EFFECT_SLOT_FOR_LINK,
            ),
            LoweredStep::SimOnlyAction(62),
            // A main-phase action naming battle-area permanents: still ONE
            // `main_phase` row, the identities riding beside the action id.
            LoweredStep::FieldAction {
                action_id: 100,
                refs: FieldRefs {
                    attacker: Some("ST1-03".to_string()),
                    ..FieldRefs::default()
                },
            },
            // A `dna:` step: ONE main_phase row carrying the DNA_DIGIVOLVE
            // action id AND both materials' identities (DCGO takes them in the
            // same `PlayCardAction`), never a second row for the material
            // picks -- those are ours alone and ride a `SimOnlySelect`.
            LoweredStep::DnaDeclaration {
                action_id: digimon_engine::action::space::DNA_DIGIVOLVE_START,
                material_ids: vec!["ST1-03".to_string(), "ST1-07".to_string()],
            },
            // The mirror of SimOnlyAction: one wire row, no sim-side row.
            LoweredStep::DcgoOnlySelect(SelectWire {
                card_ids: vec!["ST1-03".to_string()],
                ordinal: Some(1),
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                card_ids: vec!["ST1-03".to_string()],
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                value: Some(3),
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                bool_answer: Some(true),
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                cancel: true,
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                card_ids: vec!["ST1-03".to_string()],
                optional_gate_fold: true,
                ..SelectWire::default()
            }),
            LoweredStep::Select(SelectWire {
                cancel: true,
                optional_gate_fold: true,
                ..SelectWire::default()
            }),
            LoweredStep::EndOfTurnGate {
                action_id: 62,
                attack: None,
            },
            LoweredStep::EndOfTurnGate {
                action_id: 100,
                attack: Some(EotAttackTarget::Player),
            },
            LoweredStep::EndOfTurnGate {
                action_id: 100,
                attack: Some(EotAttackTarget::Permanent {
                    top_card_id: "ST1-07".to_string(),
                }),
            },
        ];
        for v in variants {
            // Two known-1-row steps plus the variant under test, so the
            // baseline is fixed and the delta is the variant's own count.
            let mut lowered = actions(&[62, 62]);
            lowered.push(v.clone());
            let job = build_job_1to1("ST1-12", &s, &entry(), &entry(), &lowered).unwrap();
            assert_eq!(
                job.inputs.len() - 2,
                v.dcgo_wire_rows(),
                "{v:?} claims {} wire row(s) but the emitter wrote {}",
                v.dcgo_wire_rows(),
                job.inputs.len() - 2
            );
        }
    }

    /// The OptionalSkill+pick FOLD (ruling item 5, `<Raid>`-family): a
    /// folded pick splits into OptionalSkill(yes) + the pick row; a folded
    /// decline emits ONLY OptionalSkill(no).
    #[test]
    fn optional_gate_fold_splits_the_wire_rows() {
        let s = select_line("{ targets: [opp.field.0] }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            card_ids: vec!["ST1-07".to_string()],
            optional_gate_fold: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 4, "one step -> two rows");
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_bool"], true);
        assert!(v["inputs"][3].get("expect_prompt").is_none());
        assert_eq!(v["inputs"][3]["select_card_ids"], serde_json::json!(["ST1-07"]));

        let s = select_line("{ decline: true }");
        let mut lowered = actions(&[62, 62]);
        lowered.push(LoweredStep::Select(SelectWire {
            cancel: true,
            optional_gate_fold: true,
            ..SelectWire::default()
        }));
        let v = job_json(&s, &lowered);
        assert_eq!(v["inputs"].as_array().unwrap().len(), 3, "decline folds to one row");
        assert_eq!(v["inputs"][2]["expect_prompt"], "OptionalSkill");
        assert_eq!(v["inputs"][2]["select_has_bool"], true);
        assert!(v["inputs"][2].get("select_bool").is_none()); // "no" skip-serializes
        assert!(v["inputs"][2].get("select_cancel").is_none());
    }
}
