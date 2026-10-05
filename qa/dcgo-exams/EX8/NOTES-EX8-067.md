# NOTES -- Close (EX8-067)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX8-067`):
**3 clauses** -- `effect#0` ([Start of Your Turn]), `effect#1` ([Your Turn] on-digivolve),
`effect#2` (Security Effect). DCGO script exists (`EX8/Black/EX8_067.cs`); not `unavailable`.

EX8-067: 3 clauses: 2 confirmed, 0 diverged, 0 unreachable, 0 unavailable, 1 unmeasured

Book: reused `qa/dcgo-exams/EX10/rocks_pool.json` (deck `rocks-exam`) via `--decks`; no new book.

| Clause | Scenario | Oracle round-trips | Verdict |
|---|---|---|---|
| `EX8-067#effect#0` | `EX8-067-effect0.yaml` | 1 | confirmed (CLEAN 7/7) |
| `EX8-067#effect#1` | `EX8-067-effect1.yaml` | 3 | unmeasured (tooling blocker, below) |
| `EX8-067#effect#2` | `EX8-067-effect2.yaml` | 1 | confirmed (CLEAN 10/10) |

## effect#0 -- start-of-turn memory
Close played T1 (cost 4 -> -4); P1 plays two Ukkomon (BT16-082, cost 3 each, 4 -> -2) so P0
starts T3 on exactly 2 memory (the "2 or less" boundary). Both engines set memory to 3 with no
prompt (DCGO `SetMemoryTo3TamerEffect` is non-optional, EX8_067.cs:14 / CardEffectFactory.cs:15).
The "memory > 2: no change" side was not exercised.

## effect#2 -- Security
Close stacked at P0's `stack[9]` (top security); P1's Ukkomon attacks on T4. Close plays free
onto P0's field (security 5 -> 4, memory untouched). No prompt in either engine.

## effect#1 -- UNMEASURED (tooling blocker, not an engine finding)
Line: EX11-038 x2 fill the trash with EX8-047 + BT21-055 (hand trash cost), Close played T3,
T5 digivolve EX11-038 into EX8-048 Landramon (cost 2). Oracle round-trips:
1. Aborted: `prompt mismatch: step 15 expected prompt 'OptionalSkill' but DCGO asked
   'MultipleSkills'` -- Landramon's [When Digivolving] (EX8_048.cs:46) activates on the Tamer
   count alone, so DCGO stacks it beside Close's observer. Fixed with a `dcgo_only` MultipleSkills
   row (pick Close) and a trailing `dcgo_only` decline of Landramon's OptionalSkill. Our engine
   queues no Landramon trigger when no [Close] is in hand (cardinality gap, outcome-neutral).
2. Diverged -- MY authoring error: the trash pick was `dcgo_only`, so our side never answered it.
3. Diverged: `p0.trash ours=[BT21-055, EX8-047] dcgo=[]`, `p0.field[1].sources
   ours=[EX11-038] dcgo=[BT21-055, EX11-038, EX8-047]`.

DCGO behaviour (recording `..._8b93322a...state.jsonl`, last row): Close suspended, trash empty,
Landramon sources `[EX11-038, EX8-047, BT21-055]`, memory 1 -- it matches the printed text.

Why round-trip 3 is NOT recorded as `diverged`: our side placed nothing because the harness answered
our prompt, not because the engine declined to offer it. `resolve_next`
(`code/digimon-engine/src/runners/selection_resolve.rs:504-570`) sends a trailing PASS after the
gate row's last pick whenever the NEXT parked prompt is a `CountCappedMultiSelect` with PASS legal
(`multiselectish`, lines 546-549 -- the `SourceMulti { picked > 0 }` guard from
`G-TOOLING-EXAM-TRAILING-PASS-EATS-NEXT-PROMPT` was not applied to `CountCappedMultiSelect`).
That PASS is "up to 2: choose 0". The sim confirms the symptom (`step 18 select answered no live
prompt -- our engine auto-resolved it`) but cannot distinguish "engine never prompts" from "tool
already passed it"; the code reading says the latter (selections.rs:3797-3839 installs a
`CountCappedMultiSelect` when >= 1 candidate exists, and 2 do). Not observed directly -- inferred.
No row shape avoids it: `cards:` cannot resolve against our `OwnField` gate prompt
("card pick 'EX8-067' not found in OwnField prompt"), and `targets` / `yes` / `decline` payloads
all hit the same trailing PASS. Budget (3 round-trips) spent; recorded `unmeasured`.
Fix to unblock: in `resolve_next`, treat a fresh `CountCappedMultiSelect { picked: 0 }` like
`SourceMulti { picked: 0 }` (PASS only if `picked > 0` or the row asked to stop).

## Data note (not a clause)
`data/card_official.json` / the bundle print Close's play cost as 3; the card image and
`cards.json` / `EX8-067.yaml` say 4. The line ran with 4 and both engines agreed with the
oracle (DCGO `action_detail` for Close is not logged here; T1 memory -4 was consistent across both).

## 2026-10-04 -- re-measure attempt after the trailing-PASS guard (0 oracle round-trips; still unmeasured)
Sim-only lowering (`exam --sim-only`, job emitted to a scratch dir, NOT to the oracle queue) still prints
`step 17 the DRIVER sent a trailing PASS to close Some(CountCappedMultiSelect { min: 0, max: 2, picked: 0, ... })
after the row's 1 pick(s)` and `step 18 select answered no live prompt`. The 190cf276c guard
(`picked > 0` for `CountCappedMultiSelect`) is bypassed here: `resolve_next` computes
`open_field_multi_pick = picks_done > 0 && resume stack has MultiPickStep/NonDslCountCappedStep/
CountCappedPermanentsStep` BEFORE the kind match (selection_resolve.rs:557-573), and the DSL "up to N"
trash pick installs a `MultiPickStep` frame (dsl_cards/step/selections.rs:3301), so the fresh prompt is
still PASSed. The guard's unit tests park the prompt with no resume frame, so they did not catch it.
No oracle job was emitted: the diff would only re-report the tool declining our prompt. The trash-pick
row is already `cards: [EX8-047, BT21-055]`, so the brief's scenario fix does not apply.
Fix to unblock (code, out of scope for the acceptance task): in `resolve_next`, require the resume-frame
branch to also see `picked > 0` (or no fresh-prompt signal) before spending the trailing PASS.
