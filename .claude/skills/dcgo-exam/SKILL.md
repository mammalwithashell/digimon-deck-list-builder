---
name: dcgo-exam
description: Cross-examine an implemented Digimon card against the DCGO oracle, clause by clause. Binds the card's printed text to the `clause_coverage` denominator, authors/runs scripted scenarios in both engines, and reports a per-clause verdict — `confirmed | diverged | unreachable | unavailable | unmeasured` — with the full denominator ALWAYS printed. Triggers on "exam <CARD>", "cross-examine <card> against DCGO", "which clauses of <card> are actually verified", authoring or running scenarios under `qa/dcgo-exams/`, reading `qa/qa-reports/dcgo_exam_verdicts.json`, or triaging an exam divergence. Runs AFTER a card is implemented and its per-card tests are green. Does NOT fix the engine.
argument-hint: <CARD_ID | SET | --suite> [--sim-only]
---

# DCGO Card-Clause Exam

You take **one implemented card**, enumerate **every clause of its printed text**, and
ask the DCGO oracle whether our engine resolves each one the same way. The output is a
per-clause verdict table over the full denominator — never a pass/fail for the card.

This is the third bug-discovery mode, downstream of the card pipeline:
`/batch-implement-cards-rust-dsl` implements → per-card `cards_behavioral` tests go
green → **`/dcgo-exam` cross-examines against the oracle** → confirmed clauses backfill
scenario assertions and can draft a `cards_behavioral` test.

You do **not** fix the engine here. An exam ends at a reported verdict table plus
triaged findings. Fixes stay a decision — **ask before fixing**.

## Non-negotiables — read before acting

- **`diverged` is a finding to TRIAGE, not proof our engine is wrong.** Source priority
  puts `general_rule.pdf` **above** DCGO. Read the printed card text and the rule before
  concluding anything. Rank, diagnose, report; then ask.
- **`unavailable` and `unmeasured` are REAL outcomes** and must be reported alongside
  `confirmed`. Never report a card as verified on a partial denominator. A card reads as
  `EX12-035: 8 clauses — 5 confirmed, 1 diverged, 2 unmeasured`, never `EX12-035: passed`.
- **`unavailable` is determined PER CARD, not per set.** DCGO's `Assets/Scripts/CardEffect/`
  spans AD1, BT1–BT26, EX1–EX12, ST1–ST24, LM, P and RB1 — so "newer than DCGO" is the
  **wrong test**. Check for the specific `<SET>/<COLOR>/<CARD_ID>.cs`, **underscored**
  (`BT17-001` → `BT17_001.cs`). A set directory can exist while an individual card has none.
- **25 cards route through `SetIsBackgroundProcess(true)`** and bypass the
  `effect_activation` hook entirely (CLAUDE.md rule 27; `docs/DCGO_RECORDING_SCHEMA.md`).
  Their clauses are **structurally unmeasurable** by activation matching — they get
  `unreachable` carrying *that specific reason*, never a silent pass.
- **The drafter's output is EVIDENCE, NOT TRUTH.** A draft `#[test]` is never committed
  without a human reading it. An auto-generated test asserting a behavior nobody read
  would launder a DCGO quirk into a permanent guard — under the no-approximations policy
  that is worse than no test.
- **DCGO work happens in the BASE repo, never a worktree** (rule 29). The worktree's
  `./DCGO` is an intentionally-empty placeholder; **never** `git submodule update --init`
  it. Resolve the real checkout:
  ```bash
  BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"
  ```
  The base DCGO repo carries **~8349 pre-existing dirty asset files**. Never
  `git add -A` / `git add .` / `git commit -a` there — stage only explicit paths you edited.

## The five verdict classes

| Verdict | Meaning |
|---|---|
| `confirmed` | A scenario exercised this clause and both engines agreed for the whole line |
| `diverged` | Both engines ran it and disagreed — **a finding to triage** |
| `unreachable` | Scenario exists but the line could not legally reach the clause (incl. background-process cards) |
| `unavailable` | DCGO has no script for this card, so no oracle exists |
| `unmeasured` | No scenario authored yet — **the default, and the point of the table** |

Store: `qa/qa-reports/dcgo_exam_verdicts.json`, one row per `(card, clause)`.
Clause identity is **not invented here** — it is `clause_coverage.models.Clause.id`,
formatted `{card_id}#{zone}#{idx}` (e.g. `EX12-073#security#0`). A verdict also stores the
clause `label` and a `text_sha256`; if the card's text drifts, the id re-points at a
different clause and the stale verdict is **invalidated back to `unmeasured`**.

---

## Phase 0 — Bind the denominator FIRST

Never author a scenario before you know how many clauses the card has.

```bash
PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX12-035
```

Record the clause count and **every** clause id. **This number is the denominator every
later report must print.** Then join it against authored scenarios and stored verdicts
via `tools.clause_coverage.exam_binding.bind(card_ids, scenarios_dir, verdicts_path)`,
and read three fields before anything else:

- `denominator.by_verdict` — always contains all five classes and always sums to `total_clauses`.
- `orphan_scenarios` — a scenario naming a clause id the extractor does not produce (typo,
  stale id after a re-scrape). **Fix it; never ignore it** — an orphan passes its own
  assertions while covering nothing in the denominator, an invisible sixth verdict class.
- `invalidated_clause_ids` — verdicts whose clause text drifted. Re-run those clauses.

Also settle `unavailable` up front — is there a DCGO script at all?

```bash
ls "$BASE_DCGO/Assets/Scripts/CardEffect/EX12/"*/EX12_035.cs
```

No file → every clause of that card is `unavailable`. Report it and stop; do not spend
Unity time.

## Phase 1 — Author one scenario per clause

`qa/dcgo-exams/<SET>/<CARD-ID>.yaml` (one file per clause):

```yaml
card: EX12-035
clause: EX12-035#effect#0        # a clause_coverage Clause.id — rejected if unknown
seed: 424242
decks:
  p0: { stack: [ST1-02, EX12-035], rest: <deck-name> }
  p1: { stack: [],                 rest: <deck-name> }
steps:
  - actor: 0
    do:     { play: {card: EX12-035, from: hand} }
    expect: { prompt: main_phase }
  - actor: 0
    do:     { select: {targets: [opp.field.0]} }
    expect: { prompt: select_permanent, count: 1 }
assert:
  - at: 2
    that: { opp.field.0.dp: 3000, p0.memory: -2 }
```

Rules that are structural, not stylistic:

- **DCGO reaches a position only by legally playing to it.** No board-materialize cheat.
  Existing `cards_behavioral` tests use `DebugRunner` staging and are therefore **not
  exam-able** — exam scenarios are a new artifact: a legal line from game start over a
  stacked deck.
- **Both seats are fully scripted.** If DCGO's AI plays the opponent, every game diverges
  for reasons that are not findings.
- **`stack` is a PREFIX and applies to the initial shuffle only.** Mid-game shuffles fall
  through to seeded RNG — still deterministic, and honest.
- **`expect` is asserted BEFORE the step is answered.** "Our engine expected a choice here
  and DCGO never asked" is a divergence class that never surfaces as an illegal action —
  a prompt mismatch aborts the job and **reports itself as a finding**.
- **`do` is symbolic** (`hatch` / `pass` / `play` / `digivolve` / `attack` / `select`) and
  lowered to a 2192-space action id against our engine's live mask. Never hand-write ids.
- A clause no legal line can reach is **not skipped silently** — record `unreachable`
  with the reason.

## Phase 2 — Run: sim first, oracle second

```bash
# 1. Lowering + assertion check in our engine only. Milliseconds. No Unity.
cargo run -p dcgo-harness -- exam --scenario qa/dcgo-exams/EX12/ --sim-only --cards-json data/cards.json

# 2. The oracle pass — deliberate, local, ~40s of Unity per scenario.
cargo run -p dcgo-harness -- exam --card EX12-035 --cards-json data/cards.json

# 3. Regression replay of the whole authored suite against the oracle.
cargo run -p dcgo-harness -- exam --suite
```

**Every scenario must lower and pass `--sim-only` before any Unity time is spent.** A
malformed line then fails in milliseconds instead of after sixty seconds of Unity.

`--sim-only` **cannot find a new divergence** — it asserts our engine still agrees with
what the oracle *previously* confirmed. That is the regression half, and the only half CI
runs (`.github/workflows/dcgo-exam-sim.yml`); DCGO cannot gate PRs (Photon, multi-GB
licensed Unity checkout, no headless play, card-art redistribution). Never describe the CI
gate as if it could catch a new divergence.

Oracle runs go through the existing harness (`docs/DCGO_HARNESS.md`): `exam` submits jobs;
`dcgo-harness up --build <dir>` / `watch` keep the player draining them.

Agent surface, once registered: `dcgo-harness mcp` exposes `exam_status(card_id)` (the
`VerdictSummary`, **always including `unmeasured`**), `run_scenario(path, sim_only)` (the
`DiffReport`), and `exam_card(card_id)` (bind + run + per-clause verdicts).

## Phase 3 — Report the full denominator

Print the table, never a verdict for the card:

```
EX12-035 — 8 clauses
  confirmed:   5
  diverged:    1   EX12-035#effect#1  (scenario qa/dcgo-exams/EX12/EX12-035-effect-1.yaml)
  unreachable: 0
  unavailable: 0
  unmeasured:  2   EX12-035#security#0, EX12-035#inherited#0
```

If any clause is `unmeasured`, `unavailable`, or `unreachable`, say so in the first
sentence of your summary. Do not lead with the confirmed count.

## Phase 4 — Triage divergences (do not auto-fix)

For each `diverged`, in this order:

1. Read the **printed card text** — the card image / official bundle (`/digimon-card-lookup`).
2. Read the governing rule in `general_rule.pdf` (`/digimon-rules`; keyword semantics §16).
   **The PDF outranks DCGO.**
3. Read the DCGO C# at `$BASE_DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID>.cs`
   for how it actually resolves.
4. Classify: **our bug** / **DCGO quirk** / **rules-ambiguous**. Report the classification
   with its evidence; **ask before changing engine or card code**.

Confirmed engine findings route to the existing trackers — card-effect faithfulness to
`qa/archetype-qa/engine-gaps.md`, missing primitives to `docs/RUST_ENGINE_GAPS.md`.

## Phase 5 — Backfill and (optionally) draft

- On a `confirmed` verdict the exam writes the observed DCGO state into that scenario's
  `assert:` block. That is what makes `assert:` load-bearing rather than decorative — the
  permanent guard that survives into CI after the oracle has gone home. **Backfill refuses
  to run on a diverged line**; forcing it would bake DCGO's disagreement in as our expected
  value and make the scenario pass forever.
- The test drafter emits a draft `#[test]` for `code/digimon-engine/tests/cards_behavioral/<set>/`
  headed `DCGO build <hash>, job <id>, scenario <path> observed:` — behind an explicit
  `--write-draft`, never auto-committed. **A human reads it before it lands.**

---

## Red flags — STOP

- About to report a card without printing `unmeasured` → STOP. The denominator is the point.
- About to call a `diverged` clause "an engine bug" before reading `general_rule.pdf` → STOP.
- About to mark a card `unavailable` because its set is "newer than DCGO" → STOP. Check the
  per-card `.cs` file.
- About to `git submodule update --init DCGO` in this worktree → STOP (rule 29). Use `$BASE_DCGO`.
- About to `git add -A` in the base DCGO repo → STOP. ~8349 unrelated dirty files.
- About to commit a drafted `cards_behavioral` test unread → STOP. Evidence, not truth.
- About to launch Unity before `--sim-only` passes → STOP. Lower first.
- About to fix the engine as part of the exam → STOP. Triage and report; ask before fixing.

## Reference

- Operating manual: `docs/DCGO_EXAM.md` — scenario format, run modes, CI split, known gaps.
- Harness ops: `docs/DCGO_HARNESS.md`; recording schema: `docs/DCGO_RECORDING_SCHEMA.md`.
- Design: `docs/superpowers/specs/2026-08-21-dcgo-scripted-scenario-exam-design.md`.
- Single-game microscope (complementary, not this skill): `/replay-bug-hunt`.
