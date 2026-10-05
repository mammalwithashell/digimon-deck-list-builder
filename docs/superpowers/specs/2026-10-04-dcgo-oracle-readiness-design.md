# DCGO Oracle Readiness — design

**Status:** approved design, 2026-10-04. Next step: implementation plan.
**Scope:** make "compared against DCGO" the bar a card must clear before RL training
draws it, and make the agentic exam loop cheap enough to clear that bar at pool scale.

## 1. Goal and decisions

The engine admits a card to training today when it is registered and its DSL ledger
verdict is not one of `PARTIAL / BLOCKED / AUDITED-DRIFT / AUDITED-MISSING-TESTS`
(`code/digimon_gym/agents/gauntlet.py`). That is a sim-only bar: a card whose own
behavioral tests pass can still disagree with DCGO, and the oracle exams keep finding
cases where it does (Three Musketeers: 15 engine fixes; Rocks Track A below: a sim test
that encodes a rules-wrong order).

Decisions made with the user:

1. **Coverage first, then recordings.** Grow the trainable pool; DCGO-recording
   usability (replay parity, behavior cloning) follows from prompt-sequence alignment.
2. **Raise the bar.** A card must be adjudicated against the DCGO oracle, clause by
   clause, before training uses it.
3. **The gate is ON by default, now.** Accept the pool reduction and work to meet the
   gate. No opt-in phase.
4. **Make the agentic loop cheap** with skills and MCP tools, measured, rather than
   replacing it with an autonomous generator. The earlier ~$8/clause figure came from
   running the loop on Fable.
5. **Not now:** a mechanical C#→YAML transpiler (the prior one died with the Python
   engine), the behavior-cloning emitter, a static C# extractor, a prefix planner.
   See §9 for the conditions that would bring them back.

## 2. Evidence (Track A, 2026-10-04)

Three Rocks core cards, the unmodified `/dcgo-exam` skill, Claude Sonnet 5.5 subagents,
oracle DCGO build `scripted-v17`, harness built from main `0dfab9924`. Token cost from
the subagent transcripts, split by phase markers, at Sonnet 5.5 list price.

| Card | Clauses | Confirmed | Diverged | Unmeasured | Turns | Cost |
|---|---|---|---|---|---|---|
| Sunarizamon (EX10-025) | 2 | 1 | 1 | 0 | 75 | $4.38 |
| Landramon (EX10-028) | 2 | 1 | 1 | 0 | 56 | $2.76 |
| Close (EX8-067) | 3 | 2 | 0 | 1 | 88 | $4.90 |
| **Total** | **7** | **4** | **2** | **1** | **219** | **$12.04** |

What it established:

- **~$1.72 per clause** on Sonnet 5.5. Reusing the first card's deck book and line shapes
  cut the second card's cost by ~37%.
- **Cost is turns × context.** ~70% of spend is cache reads of a ~200k-token context
  re-read every turn. A tool that removes turns saves more than a cheaper model.
- **Spend by phase (all three):** oracle plumbing (submit, poll, find sidecar, diff) ~27%,
  sim retries ~22%, research ~16%, recording ~12%, triage ~10%, extraction ~10%,
  authoring ~3%. The oracle itself answered every job in ≤15 s; it is not the bottleneck.
- **10 oracle round-trips for 4 confirmed clauses** (2.5 each); three of them went to the
  one unmeasured clause.
- **Both divergences are one engine defect.** Source-trash triggers resolve mid-effect,
  violating `general_rule.pdf` §15-8-3-2 (p.25); DCGO stacks them. The engine inline-drains
  on purpose so `ex10_036_clause_a_after_source_trash_prompts_opp_field_delete` passes.
  Recorded in `docs/RUST_ENGINE_GAPS.md` under `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS`
  (update 2026-10-04). Until fixed, every Rocks inherited clause diverges.
- **The unmeasured clause is a harness defect** (§4.1, item 1), which consumed all three of
  that clause's oracle round-trips.
- **Data discrepancy:** `card_official.json` prints Close (EX8-067) at play cost 3; the card
  image, `cards.json`, the YAML, and the oracle agree on 4.

Artifacts: `qa/dcgo-exams/EX10/EX10-025-*.yaml`, `EX10-028-*.yaml`, `EX8/EX8-067-*.yaml`,
their `NOTES-*.md`, `qa/dcgo-exams/EX10/rocks_pool.json`, and verdict files under
`qa/qa-reports/exam-verdicts/`.

Pool impact, estimated against main at the time of writing (playable = registry ∪ vanilla):

| Gate | Decklists | Archetypes |
|---|---|---|
| Current (registry + ledger deny-list) | 1519 | 72 |
| Oracle bar, upper bound | ≤101 | ≤3 |

The top blockers are cross-archetype staples, not archetype cores: Ukkomon (BT16-082) in
270 admitted decklists, In-Between Theater (BT24-100) in 214, Aegiomon (BT24-034) in 206.

## 3. Architecture

Three loops, one artifact between them.

```
  exam loop (agents + oracle)         readiness generator            training
  ───────────────────────────         ───────────────────           ────────
  /dcgo-exam, /readiness-batch  ──▶   verdict store           ──▶   gauntlet / deck pool
  dcgo-harness exam --oracle          + clause denominator          admit decklist iff
  MCP: exam_probe(oracle) …           ──▶ data/oracle_readiness.json every card is ready
          ▲                                     │
          └──────── readiness plan (ranked card queue) ◀──┘
```

- The **verdict store** (`qa/qa-reports/exam-verdicts/`) stays the source of truth per
  `(card, clause)`.
- The **readiness generator** joins the printed clause denominator with the store and
  writes a committed, deterministic artifact. Training reads only that artifact.
- The **readiness plan** ranks unexamined cards by how many decklists they unblock and
  feeds the exam loop.

## 4. Components

### 4.1 Tool-defect fixes (first; each burned whole oracle round-trips)

1. **Trailing PASS eats a fresh "up to N" prompt.** `resolve_next`
   (`code/digimon-engine/src/runners/selection_resolve.rs`) treats any
   `CountCappedMultiSelect` as the current row's still-open prompt and sends PASS. Guard it
   like `SourceMulti`: `CountCappedMultiSelect { picked, .. } => picked > 0`. Same defect
   family as `G-TOOLING-EXAM-TRAILING-PASS-EATS-NEXT-PROMPT`. Driver:
   `qa/dcgo-exams/EX8/EX8-067-effect1.yaml`.
2. **Digivolution source unauthorable inside a `UnionZone` prompt.** The resolver's
   UnionZone arm matches hand and trash only. Add identity matching for digivolution
   sources, with the carrier named by `targets:`. Driver: the abandoned EX11-038 line in
   `qa/dcgo-exams/EX10/NOTES-EX10-028.md`.
3. **Verdict writes rewrite unrelated files.** `--verdicts` rewrites line endings of every
   card file (~100 files per run). Write only the touched card's file and preserve its
   existing line endings.
4. **No backfill entry point.** `exam/backfill.rs` is not wired to the CLI. Add
   `--backfill` so a confirmed run writes the observed state into the scenario's `assert:`
   block. Without it a confirmed scenario guards nothing in CI.
5. **MCP plan/status silently empty for unextracted cards.** `exam_plan` / `exam_status`
   read a committed clause book holding 98 cards and report "0 outstanding" for any other
   card. Extract on demand for cards not in the book; never report zero for an unknown card.
6. **MCP `node_health` has no root.** It checks `.` and reports NO-GO. Resolve the harness
   root the same way the CLI does, or accept it as an argument.
7. **Unhelpful sim notes.** "our engine auto-resolved" cannot distinguish "the engine never
   prompts" from "the driver already answered it". Name the actor that resolved the prompt.

### 4.2 One-call oracle loop

`dcgo-harness exam --oracle --scenario <file>` and MCP `exam_probe(sim_only: false)`:

1. Lower the scenario (as `--sim-only --emit-job` does) under a deterministic job id
   `exam-<CARD>-<zone><idx>`.
2. Submit to the harness root's `jobs/`.
3. Wait on `done/<job-id>.result.json` (configurable timeout; quarantine and failure are
   terminal results, not hangs). Never locate recordings by newest file.
4. Read `recording_path`, derive the `.state.jsonl` sidecar, run the existing differ.
5. Write the verdict for that card only; on `confirmed`, backfill asserts.
6. Return one structured result: verdict, first divergence (field, ours, dcgo, step),
   compared-row counts, sidecar path, job id.

Preflight runs `node_health` first and refuses on NO-GO.

### 4.3 State and prompt introspection

`exam_probe(..., inspect_step: N)` and `exam --sim-only --inspect N` return, after step N:
the pending selection (kind, min/max/picked, optional, legal candidates by card identity),
each player's hand/trash/field/sources/memory, and which prompt the driver answered at
each step. Agents built throwaway `assert:` blocks to get this.

### 4.4 Lints in `exam_validate`

- **Deck budget:** per player, stacked copies plus the `rest:` deck book never exceed the
  copy limit (`stacked card X is not in deck` was really a copy-count error).
- **Ambiguous play:** a `play {card}` with several matching hand cards gets a
  `hand.<i>` suggestion computed from the stacked draw order.
- **Known-unauthorable shapes:** warn before running when a row targets a shape the
  resolver cannot answer.

### 4.5 Per-archetype scenario library

`qa/dcgo-exams/_library/<archetype-slug>/` holds the archetype's deck book(s) and a
`LINES.md` of proven line shapes (how to fill the trash, reach a given level by turn N, put
a card at the top of security, the `dcgo_only`/`sim_only` folds the archetype needs), each
citing the scenario that proved it. The exam skill reads it first. A run that discovers a
new shape appends it. The Rocks entries seed from the Track A scenarios.

### 4.6 Verdict schema: structured triage

Add to diverged rows: `triage: ours_wrong | dcgo_quirk | undetermined` and
`citation: <general_rule.pdf §… | DCGO file:line>`. `dcgo_quirk` requires a citation that
puts the rules on our side. Backfill the six existing diverged rows by reading their
`reason` and NOTES; any that cannot be classified become `undetermined`. Track A's two rows
are `ours_wrong` (§15-8-3-2).

### 4.7 Readiness artifact and the training gate (default ON)

**Generator:** `PYTHONPATH=code python -m tools.clause_coverage.readiness` writes
`data/oracle_readiness.json`, with `--check` for CI drift (mirrors
`code/tools/build_tested_cards.py`). Per card:

- `ready` — every printed clause, individually, has an adjudicated verdict: `confirmed`;
  `unreachable` or `unavailable` with a stated `reason`; or `diverged` with
  `triage: dcgo_quirk` and a `citation`. A clause with no verdict row counts as
  `unmeasured` and blocks.
- `not_ready` — otherwise, with counts by verdict class and the blocking clause ids.
- Vanilla cards (no effect, inherited, or security text) are `ready` by construction.

A card's verdicts are invalidated by the existing `text_sha256` drift rule, so a text
change drops it back to `not_ready` automatically.

**Gate:** `gauntlet.py` admits a decklist only if, in addition to today's checks, every
card in it is `ready`. The rejection reason is `not_oracle_ready`, counted alongside
`unregistered` and `not_ready`. The gate applies to **every** library file training loads,
not only the default `data/deck_library.json` (today's ledger gate runs only when the
default path is loaded; the oracle gate must not inherit that hole). The starter
curriculum's ST-1..6 decklists live in the deck library, so they are covered. There is no
CLI override. Tests keep injecting their own ready set, as they do for the ledger gate
today.

**Empty pool fails fast.** If no decklist survives, training stops at startup with the
top blocking cards (card, name, decklists blocked) instead of training on nothing. This
will happen for the starter curriculum until ST-1..6 are examined; that is accepted.

**Applies to:** every library load (`MetaGauntlet.load`, and through it
`load_generalist_deck_pool`, which `pilot_training`, the starter curriculum's phase 1,
`run_training_job`, the specialist league, and the eval CLIs all use) and every pool
snapshot load (`GeneralistDeckPool.from_snapshot`, used by `--curriculum-pool` and the
anchored-eval/Elo/exploiter CLIs), so a snapshot taken before the gate cannot smuggle
non-ready decks back in. `DeckPoolWrapper` builds variants from explicit decks and never
reads the library, so it is out of scope. The desktop deck builder and hosted API keep
using `data/tested_cards.json`; they are not training.

**Determinism:** the generator runs clause extraction with DCGO disabled
(`DIGIMON_DCGO_ROOT=""`) so the artifact is identical on a machine with the base-repo DCGO
checkout and in CI without it. A card with neither official text sections nor an image
cache entry therefore carries an `image-required` clause and stays `not_ready` — the
conservative outcome.

### 4.8 Readiness plan and the dispatch unit

`python -m tools.clause_coverage.readiness --plan [--limit N]` ranks `not_ready` cards by
greedy decklist completion: repeatedly choose the card whose examination completes the
most decklists that are otherwise ready, tie-broken by the number of decklists containing
the card. Output: card, archetype(s), outstanding clause ids, decklists unblocked.

**`/readiness-batch`** (new skill, thin) takes the top N cards from the plan, groups them
by archetype so each group shares a library entry, claims them through the existing ledger
(`claim` / `release`), and runs `/dcgo-exam` per card with Sonnet 5.5 subagents. It does
not re-implement the exam. Divergences go through the existing campaign fix gate
(citation, a test that fails before and passes after, `cards_behavioral` green; engine
fixes on their own branch for human review). `/archetype-campaign` stays for
archetype-scoped work.

### 4.9 Lean `/dcgo-exam` skill

Rewrite the skill around the tools: extraction via `exam_status`, library first, author,
`exam_validate`, `exam_probe` sim with `inspect_step`, `exam_probe` oracle, triage with a
structured `triage` field. Default worker model Sonnet 5.5. Keep the honesty rules
(`confirmed` only from an oracle diff; the full denominator always printed).

### 4.10 Cost measurement

Promote the Track A transcript parser to `code/tools/exam_cost_report.py`: given a
subagent transcript, report turns, tool calls, and tokens per phase marker, plus dollars
at a configurable price table. Every `/readiness-batch` run records per-card cost in its
ledger entry so the effect of each tool is measured, not assumed.

### 4.11 Engine fix that unblocks Rocks

Fix `G-ENGINE-INLINE-DRAIN-SIBLING-TRIGGERS` as the first engine work under the fix gate:
route source-trash observers through the deferring drain (§15-8-3-2), mirroring the
`G-ENGINE-SECURITY-REMOVED-OBSERVER-MID-EFFECT` fix (`drain_effect_queue` →
`maybe_drain_effect_queue` at the source-trash fire sites in `game_actions/mod.rs`). The
EX10-036 test that motivated the inline drain pins Magneticdramon EX10-036's sibling
When Digivolving clause running in the middle of the first clause; rewrite it to the
rules order and confirm against the oracle. Remove the
`G-DSL-TAIL-CLOBBERS-INLINE-OBSERVER-SELECTION` park once nothing drains inline from inside
an effect. Re-run `EX10-025#inherited#0` and `EX10-028#inherited#0` against the oracle as
the acceptance check.

## 5. Order of work

1. Tool-defect fixes (§4.1) and the triage schema (§4.6).
2. Readiness generator, gate ON, fail-fast, plan (§4.7, §4.8 plan only).
3. One-call oracle loop and introspection (§4.2, §4.3).
4. Lints, library, lean skill, `/readiness-batch`, cost report (§4.4, §4.5, §4.9, §4.8, §4.10).
5. Engine fix for Rocks (§4.11); can run in parallel with 3–4.
6. Run readiness batches from the plan; re-measure after each tooling step.

The gate turns on in step 2. The pool shrinks immediately; that is the accepted cost.

## 6. Success criteria

- **Gate:** training refuses any decklist containing a card that is not `ready`, across
  every deck source, and fails fast with blockers when nothing survives.
- **Cost:** median ≤ **$0.75 per adjudicated clause** on Sonnet 5.5 over the first ten
  cards after step 4, measured with `exam_cost_report` (Track A baseline: $1.72).
- **Oracle efficiency:** ≤ **1.3 oracle round-trips per confirmed clause** (Track A
  baseline: 2.5).
- **Pool:** the plan's top-ranked decklists become admissible in plan order; the readiness
  artifact reports the admitted decklist count after every batch.
- **Honesty:** zero clauses reported `confirmed` without an oracle sidecar diff; every
  `dcgo_quirk` carries a citation.

## 7. Error handling

- Oracle NO-GO: `exam --oracle` and `/readiness-batch` refuse before authoring.
- Job timeout, quarantine, or failure: returned as a terminal result with the job id; the
  clause stays `unmeasured` with the reason; no retry loop beyond the skill's budget
  (3 oracle round-trips per clause).
- Readiness artifact missing: training fails at startup with the regeneration command
  (no silent fallback to the old gate). Readiness artifact stale: CI `--check` fails the PR
  (recomputing at every training start would mean re-extracting the whole pool).
- Clause text drift: existing `text_sha256` invalidation returns the card to `not_ready`.
- Concurrent nodes: existing advisory claims; duplicate verdicts are detectable at merge.

## 8. Testing

- **Harness fixes:** a regression test per §4.1 item, driven by the named Track A
  scenarios, in the `dcgo-harness` and engine test suites.
- **Oracle loop:** unit tests with a fake harness root (pre-written `result.json` and
  sidecar) for success, divergence, timeout, quarantine; no Unity required.
- **Readiness generator:** fixture verdict stores covering each status rule (confirmed,
  reasoned unreachable, unavailable, `dcgo_quirk`, `ours_wrong`, `undetermined`, missing
  clause, text drift, vanilla); `--check` drift test.
- **Gate:** extend `code/tests/rl/` gauntlet tests with a ready-set fixture: rejection
  reason counts, a non-default library path still gated, starter archetypes gated,
  empty-pool fail-fast message.
- **Plan:** deterministic ranking on a small synthetic library.
- **Engine fix:** failing-then-passing DebugRunner tests for the deferred trigger order;
  the two Rocks inherited scenarios re-confirmed on the oracle.
- CI runs everything Unity-free; only oracle submission needs a node.

## 9. Deferred, and what would bring each back

| Deferred | Revisit when |
|---|---|
| C#-derived prompt-sequence prediction | Prompt-translation cycles stay above ~15% of spend after §4.3–4.5 |
| Prefix planner (search our engine for a legal line) | Line authoring stays the top friction after the library has entries for the target archetype |
| Mechanical C#→YAML transpiler | Implementation, not examination, becomes the readiness bottleneck |
| Behavior-cloning emitter | Replay parity of the DCGO corpus is high enough to trust recorded actions (today 246/327 games) |
| Replay-parity demotion signal | Triage can attribute a first divergence to a card, not an action slot |
