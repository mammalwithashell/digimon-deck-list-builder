# NOTES — Mimi Tachikawa (BT3-096)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT3-096`):
**2 clauses** — `effect#0`, `effect#1`.
DCGO script: `BT3/Purple/BT3_096.cs` exists — the card is **not** `unavailable`.
Both clauses have a scenario; neither is `unreachable`.
Book: `qa/dcgo-exams/BT3/tm_bt3_096_pool.json` (decks `tm-mimi-purple`, `quiet-opponent`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT3-096#effect#0` | `[All Turns] When a player uses an Option card, you may suspend this Tamer to gain 1 memory.` | `BT3-096-effect0.yaml` | lowers, asserts pass |
| `BT3-096#effect#1` | `[Security] Play this card without paying its memory cost.` | `BT3-096-effect1.yaml` | lowers, asserts pass |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

Both lowered and asserted as found. `effect#0` (`OnUseOption`,
`SetUpActivateClass(..., -1, TRUE, ...)`): one OptionalSkill after the used
Option's body (DCGO stacks the OnUseOption trigger in
`UseOptionClass.UseOption` and runs `OptionSkill` inline first; ours drains
OnUseOption after the body), then suspend + `AddMemory(1)`, no pick. The
Option used is Purple Scramble LM-032 with **no purple Digimon on the field
and none in hand** — deliberately: with a purple Digimon in hand our engine
parks a non-optional Hand pick from LM-032's own `[Main]` that DCGO never
asks (card-YAML finding recorded in `../LM/NOTES-LM-032.md`). Keeping the
hand red makes Mimi's gate the only prompt after the use. `effect#1` is the
standard `PlaySelfTamerSecurityEffect` skeleton, mandatory, no prompt.

## Second resume (2026-09-18): re-lowered; slot hygiene checked

Every file for this card re-lowered unchanged against the current harness
(no digivolve into BT25-085, no `<De-Digivolve>`, no `<Link>` on this card).
Slot hygiene (the campaign's centre-out rule, `../BT25/NOTES-BT25-083.md`:
`field.N` in `attack:` / `digivolve:` / `main:` is a COMPACT frame-order index
on DCGO, reversed against ours on a two-Digimon board): every slot-addressed
step here acts on a seat holding exactly ONE Digimon, with any Tamer / Option
played AFTER it (back-row frames sort after every Digimon on DCGO), so
`field.0` names the same permanent on both engines. `select: { targets: }`
rows ride the wire by top-card identity and are unaffected.

## `effect#0` — verdict RE-EARNED 2026-09-21 (it had been standing on stale evidence)

`BT3-096#effect#0` was stored `confirmed` on 2026-09-19, BEFORE `5aef07fa8`
inserted our 9-1-5 `TriggerOrder` trash-order prompt and before this scenario
gained its `sim_only` `choice: "Trash the used Option"` row. Re-diffed against the
preserved sidecar `20260918T131227Z_d89e270c562949ca86c373b7791e3354.state.jsonl`
on the harness as it stood, the line was **DIVERGED**: `memory: ours=2 dcgo=1` at
the `select: { yes: true }` row.

That was not an engine divergence. `ReplayDriver::auto_answer_option_trash_order`
answered the engine-only trash-order prompt on DCGO's behalf AND the scenario's own
sim-only row answered it too, so the row landed one decision late — on Mimi's
"you may suspend this Tamer" decline gate (`general_rule.pdf` §15-7-1 / §15-7-4,
p.24) — and spent it. The memory gain had therefore already applied at the row
that was written to answer the gate. This is the same defect found on
`BT25-091#effect#2`; BT3-096 was its silent second instance, and an instrumented
sweep of all 369 exam scenarios confirms these two are the ONLY lines that reach
the prompt.

Fixed by `RecordingSource::answers_engine_only_prompts()` (default `false`;
`ScenarioAdapter` → `true`), which suppresses the auto-answer for exam scenarios
and leaves the DCGO-corpus replay path untouched. Re-diff post-fix: **CLEAN**,
compared 8 of 9 ours / 8 dcgo steps (1 sim-only row with no DCGO prompt). Verdict
re-recorded `confirmed` 2026-09-21. Full write-up:
`G-TOOLING-EXAM-AUTO-ANSWER-EATS-NEXT-PROMPT` in `qa/resolved-gaps.md`.
