# The card loop — operating manual

`python -m tools.card_loop` drives `claude -p` and `codex exec` as stateless
workers to implement cards (YAML DSL), author exam scenarios, and adjudicate
every printed clause **and every gating interaction** of a card pool against
the DCGO oracle. Code orchestrates; a model is called only for a judgment
(implement, review, author, triage, classify a ruling, fix). Design:
`openspec/changes/add-card-authoring-loop/design.md`; the exam it drives:
`docs/DCGO_EXAM.md`; the readiness gate it feeds: CLAUDE.md rule 34 and
`docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md`.

Run everything from the repo root with `PYTHONPATH=code`.

## 1. Prerequisites

| Need | Check |
|---|---|
| Claude Code CLI, logged in | `claude auth status` |
| Codex CLI (bundled with the Store app) | preflight resolves it; the loop always runs it `-s workspace-write` and refuses `danger-full-access` |
| A DCGO oracle player build whose action space matches the engine | `dcgo-harness --root <harness root> node status --build <player dir>` is GO |
| The player running while the loop runs | `dcgo-harness --root <root> node up --build <player dir>` (one node, at most 2 players: `docs/runbooks/oracle-node.md`) |
| A config file with the oracle paths and a USD cap | copy `qa/card-loop/config.example.toml` and edit `harness_root`, `player_dir`, `budget_usd` |

Preflight (`plan` runs it; `preflight` runs it alone) refuses or downgrades,
never silently proceeds: the official mirror and `cards.json` must cover the
pool, the keyword gate must pass, each card's DCGO effect class is resolved
(a card DCGO cannot play is planned `unavailable`), both CLIs must answer, and
the node must be GO. A failed check names its remedy.

## 2. Choose what to run

```bash
python -m tools.card_loop candidates --top 10          # qa/card-loop/candidates.md + .json
```

ranks the meta archetypes (`data/deck_library.json`) by meta weight × closeness
to fully adjudicated, in tiers: **A** implemented but unverdicted, **B** a few
cards missing in a coherent era, **C** large implementation gaps. Each row says
how many cards, clauses and gating interactions are left and prints the `plan`
command to run.

## 3. Plan

```bash
python -m tools.card_loop plan --archetype "Toho Braves"                     # deck-library archetype (alias ok)
python -m tools.card_loop plan --set BT27                                    # a release set
python -m tools.card_loop plan --decklists decks/*.txt --core-fraction 0.7   # any format the engine parses
python -m tools.card_loop plan --cards BT26-065,BT26-072 --run-id my-run     # explicit ids (or @file)
```

Inputs combine. The plan is frozen at `runs/card-loop/<run-id>/plan.json`
(git-ignored): the work set (`pool`, `core`, `ranking`), the base sha, the
config, DCGO script presence and the preflight report. `--skip-preflight`
records the plan without the gate.

## 4. Run, resume, status

```bash
export PYTHONPATH=code                        # the CLI imports `tools.*` from code/
python -m tools.card_loop run --plan runs/card-loop/<run-id>/plan.json --config my-loop.toml \
    --worktree-root D:/cl                     # short path: worker pool + run tree live here
python -m tools.card_loop status --run <run-id>
python -m tools.card_loop resume --run <run-id> --config my-loop.toml --worktree-root D:/cl
```

A real run checks out the run branch `card-loop/<run-id>/run` from the plan's
base sha in its own tree (`--run-tree`), extends the clause-text book for the
pool, and commits every tree write (scenarios, backfilled asserts, verdicts,
ledgers) with `Loop-Attempt:` / `Loop-Model:` / `Loop-Run:` trailers. Engine
fixes land on `card-loop/<run-id>/engine-<gap>` branches for human merge. The
loop never pushes and never touches `main`. `--budget-usd` overrides the
config's cap; `--max-attempts N` stops after N worker calls; `--serial` runs
one task at a time; `--fake <canned.json>` replays canned worker results (no
model, ledgers under `<run-dir>/fake/`) — the way to smoke a plan for nothing.

### What a run does to each item

```
card:        PENDING → IMPLEMENTING → REVIEW → IMPLEMENTED   (or PARKED on a gap)
clause/int:  PENDING → [CLASSIFY] → AUTHORING → [ENCODE] → SIM → ORACLE → CONFIRMED
                                         └── DIVERGED → TRIAGE → ours_wrong → FIX → GATE → ORACLE
                                                               → dcgo_quirk | unreachable → TERMINATION_CHECK → TERMINAL
                                                               → undetermined → ESCALATED
```

- **No model** runs the sim check, the oracle round trip (`dcgo-harness exam
  --oracle`: submit, wait for that job's own result, diff, record, backfill),
  the merge, the fix gate (citation + a test that fails before and passes
  after + the scoped `cards_behavioral` suite) and the termination check's
  bookkeeping.
- **Two families** must agree on any call that ends an item without our
  engine changing (`dcgo_quirk`, `unreachable`, a ruling classified `textual`
  or `not_examinable`, the `expect_ruling` encoding). Each gets the same
  packet and never sees the other's answer. Disagreement, a missing citation,
  or only one vendor available → `ESCALATED`.
- **A prompt mismatch** on the oracle is routed by who disagreed: both engines
  contradict the scenario → back to AUTHORING; ours matches the scenario and
  DCGO does not → TRIAGE (a missing decline looks exactly like this).
- **A stalled player** (DCGO enforces no job timeout: a scripted selection
  that cannot complete its prompt holds the player forever, heartbeat fresh)
  is reported by the harness once the claim outlives the job's own
  `timeout_seconds` + 60 s while the heartbeat still names it, with the newest
  recording's last row as evidence. The item goes to TRIAGE as `undetermined`
  and the stage restarts the player (`node down` / `node up`) — only while
  the heartbeat still names that job, so two runs sharing a player do not
  restart it twice. A retry whose identical job is still on the player waits
  for it; a claim the heartbeat no longer names is set aside under `aside/`.
- **An interaction waits** (`waits for <clause> (FIX)` in the blocked report)
  while any clause of its card is DIVERGED / TRIAGE / FIX / GATE: the engine
  under the exam is about to change, and an exam authored now fails the very
  assertion the fix targets.
- **Interactions** start from the gating denominator
  (`data/interaction_denominator.json`): official rulings `qa:<Q>` and
  generated probes `probe:<clause>:<family>[:neg]`. A ruling is first
  classified (behavioral / textual / not_examinable), then a line that
  exercises it is authored by a family other than the card's implementer, then
  its publisher answer is encoded as `expect_ruling:` against that line by one
  family and verified blind by the other; the sim step writes the agreed block
  into the scenario. A line that never reaches the ruling's situation goes
  back to its author with the objection. A legal line on which our engine
  contradicts the ruling goes on to the oracle and triage (the engine's
  finding), not back to the author.

### Stopping

The USD cap, the wall-clock cap, per-stage attempt caps (`attempt_caps` in the
config: author 3, fix 2, one triage per family …), a plateau of
`plateau_attempts` worker calls with no new adjudication, or every vendor
exhausted. Every stop writes `state.json` and `report.md`; the report's first
line names the **unmeasured, escalated and unavailable** counts before the
confirmed count. `resume` rebuilds from the committed ledgers first and the
run's `events.jsonl` second, so a crashed run re-does only outstanding work.

There is no graceful stop yet: to stop a running driver early, kill its
process tree (`taskkill /PID <driver pid> /T /F` — the tree holds the worker
CLIs and any `exam --oracle` waiting on the player), then `resume`. Work in
flight at the kill is lost (its cost is real but unledgered). Run the CLI
with `PYTHONPATH=code` (`pyproject.toml` sets that path for pytest only).

## 5. What lands where

| Where | What |
|---|---|
| `runs/card-loop/<run-id>/` (git-ignored) | `plan.json`, `events.jsonl`, `state.json`, `report.md`, worker transcripts and diffs |
| `qa/card-loop/attempts.jsonl` | one row per worker call (family, model, prompt version, assignment, outcome, usage); union-merged |
| `qa/card-loop/corrections.jsonl` | every correction of an earlier attempt: immediate (review reject, gate fail, schema, family disagreement) and late (blame, overturned verdict, …) |
| `qa/card-loop/audits.jsonl` | human verdicts on the 5% audit sample |
| `qa/card-loop/escalations/<item>.md` + `index.jsonl` | the human queue: both families' arguments, citations, attempt history |
| `qa/qa-reports/exam-verdicts/<card>.json` | per-clause and per-interaction verdicts (v2), written by the harness |
| `qa/dcgo-exams/<SET>/` | the scenarios the loop authored, backfilled with oracle-confirmed asserts |
| `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md` | gap records a parked card raised (orchestrator-only writes) |

## 6. Handling escalations

An escalation is **not adjudicated** and blocks readiness. Read the file: it
holds each family's call, citation and reasoning, and the attempt ids. Decide
with the sources in CLAUDE.md's priority order (`general_rule.pdf`, then
DCGO's C#, then the card data), then either

- record the verdict yourself (`dcgo-harness verdict-triage --clause <id>
  --triage ours_wrong|dcgo_quirk --citation <ref>`, or `dcgo-harness
  verdict-set --clause|--interaction <id> --verdict unreachable|unavailable
  --reason "…"`) and delete the escalation file, or
- fix the card / scenario and delete the file; `resume` re-enters the item.

Deleting the file alone withdraws the escalation: `resume` puts the item back
at its ledger state with its attempt counters reset. Do that for escalations
the tooling caused (an `oracle_retry 3/3` behind a stalled player, a merge the
transport refused) rather than re-planning the run. The files live in the run
tree (`<run tree>/qa/card-loop/escalations/`) and are committed there.

A worker's edit to a gap tracker (`qa/dsl-vocab-gaps.md`,
`docs/RUST_ENGINE_GAPS.md`) is dropped at merge, not refused — the rest of
its diff lands, and the drop is listed in the merge result's `skipped`.

Resolving against a worker's call is a late correction the scorecard counts.

## 7. Judge the models

```bash
python -m tools.card_loop audit next                 # the next sampled accepted output to check
python -m tools.card_loop audit record <attempt_id> agree|disagree --note "…"
python -m tools.card_loop report models [--since 30d] [--stage triage] [--json]
```

`report models` shows, per stage × model × prompt version: attempts,
first-pass acceptance, immediate and late correction rates (late weighted ×3),
cost per accepted-and-uncorrected unit, Wilson intervals, audit precision and
reviewer precision. Routing changes only when both families have ≥
`routing_min_samples` (30) per stage and their intervals separate; until then
the config's `[routes]` apply, and a seeded 20% of each stage is cross-assigned
so the comparison is not confounded by item difficulty. Workers never see the
scorecard.

## 8. Readiness

```bash
python -m tools.clause_coverage.readiness --out data/oracle_readiness.json   # regenerate (deterministic)
python -m tools.clause_coverage.readiness --check                             # CI drift check
python -m tools.clause_coverage.readiness --plan --limit 20                   # cards that unblock the most decklists
python -m tools.card_loop interactions --check                                # the gating denominator has not drifted
```

Training admits only decklists whose every card is `ready` (rule 34). Today
readiness is clause-level; the interaction join (change task 7.x) will add
"every gating interaction adjudicated". Adding a probe family is a promotion
(`code/tools/card_loop/interactions/promotion.json`), never an edit: a new
family gates only once promoted, because promotion drops cards from readiness.

## 9. Known limits

- One oracle node, at most two players; the oracle is the throughput ceiling
  (~15–60 s per scenario).
- DCGO itself enforces no per-job timeout (`HarnessJobLimits.timeout_seconds`
  is read by nobody in Unity): a wedged job is detected and cured from this
  side (stall report + player restart, above), at the cost of the job's
  limit + 60 s per stall. Teaching the DCGO mod to abort a selection its
  script cannot complete is the proper fix (base-repo DCGO work, rule 29).
- A card whose official-mirror entry is a failed lookup (no `colors`) is not
  `ready` until the mirror covers it; EX12 is the known hole.
- `exam_probe` (MCP) never records a verdict; only a committed scenario's run
  does.
- Engine fixes wait for a human to merge their branch; the item sits in GATE
  with `engine_wait` until that branch is an ancestor of the run tree.
- Group 7 (interaction readiness) and the first pilot's cost measurement
  (change task 8.2) are still open.
