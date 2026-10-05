## Context

**What exists.** The DCGO exam works end to end: `dcgo-harness` (scenario → lower → job queue → headless DCGO player → state sidecar → differ → verdict), a ten-tool exam MCP registered for both Claude (`.mcp.json`) and Codex (`~/.codex/config.toml`), a per-card verdict ledger (`qa/qa-reports/exam-verdicts/`), advisory claims, the `/archetype-campaign` and `/dcgo-exam` skills, and an oracle node runbook (`-batchmode -nographics`, two concurrent players measured clean, ≤15 s per job). Three campaigns (Toho Braves, Three Musketeers, Rocks Track A) ran it as human-attended LLM sessions.

**What is approved but not built.** The oracle-readiness design (`docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` and five plans, branch `worktree-dcgo-effect-translation`, b2a3f914d / 6cba79462): harness defect fixes, structured `triage` on diverged rows, one-call `exam --oracle` + `exam_probe(sim_only:false)`, step introspection, lints, a per-archetype scenario library, a lean `/dcgo-exam`, `/readiness-batch`, a cost report, the committed readiness artifact `data/oracle_readiness.json`, and an always-on training gate. This change builds on those and does not re-specify them.

**Measured economics.** Track A: ~$1.72/clause on Sonnet 5.5 (Fable runs ~$8), ~70% of spend is cache reads of a ~200k context re-read every turn, oracle plumbing ~27% of spend, 2.5 oracle round trips per confirmed clause. Campaign orchestration: $704 of $4,210. Turns × context is the cost lever; a cheaper model is a smaller one.

**State of the denominator.** ~3,620 printed clauses across ~1,035 YAML cards, ~370 verdicts (347 confirmed). 211 authored scenarios have never been run against the oracle. Official Q&A: the old mirror held one answer per card for 2,590 cards with no questions; the real data is several rulings per card (BT7-056: 4), each with a stable Q-number.

**Constraints.** No-approximations policy (CLAUDE.md rules 17, 28). DCGO and the rules PDFs live in the base repo (rules 29, 32). Per-worktree `CARGO_TARGET_DIR` (rule 31). `general_rule.pdf` outranks DCGO for rules; the official Bandai DB outranks DCGO for printed data. Engine fixes land on their own branch for human review (fleet-design fix gate). The user's Codex config defaults to `danger-full-access` + `approval_policy = "never"`.

## Goals / Non-Goals

**Goals:**
- One durable command that takes cards, a set, or decklists and drives them to adjudication, resumably, on any future set.
- Code orchestrates; models do one judgment task per call with a small, schema-validated packet.
- Interaction coverage that measurably raises confidence beyond one happy path per clause, with a deterministic, gating denominator.
- Use Claude and Codex where each is best, decided by measured correction rates rather than assumption.
- Cut per-clause cost below Track A's $1.72 by removing orchestration turns.

**Non-Goals:**
- Re-implementing anything the readiness plans deliver (one-call oracle, triage schema, library, readiness artifact, cost report).
- A mechanical C#→YAML transpiler, a behaviour-cloning emitter, or a prefix planner (deferred by the readiness design §9).
- Running DCGO in CI, a distributed job queue, or a lease server (fleet-design non-goals).
- Auto-merging engine fixes. The loop opens a branch and flags it; a human merges.
- Replacing the interactive skills. `/dcgo-exam`, `/archetype-campaign` and `/readiness-batch` remain for attended work.

## Decisions

### D1 — A Python driver orchestrates; models are stateless workers

The driver owns the per-item state machine, the oracle queue, test gates, merges, the ledger and the reports. It calls a model only for authoring, implementation, triage, review and Q&A classification. Each call is a fresh process with a task packet of references plus the minimum inline context, and returns JSON validated against a stage schema.

*Alternatives.* (a) An LLM main thread running `/archetype-campaign`: this is today's shape and the source of the $704 orchestration line and the ~200k-context turns. (b) A Claude Code `Workflow` script: it runs inside a Claude session, cannot drive Codex except via Bash inside an agent, and its structured-output step aborted 7/16 slices in `author-set`. (c) Rust inside `dcgo-harness`: the resolvers, `clause_coverage`, the campaign planner and the readiness generator are Python, so the driver lives beside them and calls the harness as a CLI.

### D2 — Package layout

`code/tools/card_loop/`: `workset.py` (resolution + preflight), `state.py` (item state machine, event log), `stages/` (one module per stage), `workers/` (`base.py`, `claude.py`, `codex.py`, `fake.py`), `ledger.py` (attempts, provenance, corrections), `scorecard.py` (metrics, router), `interactions/` (Q&A join, probe generator, denominator), `prompts/<stage>.md` (versioned templates), `schemas/<stage>.json`, `__main__.py` (CLI). Run directories are `runs/card-loop/<run-id>/` (git-ignored): the frozen `plan.json`, `events.jsonl`, worker transcripts and diffs. Cross-run truth stays in committed ledgers.

### D3 — Work-set resolution

| Input | Pool | Core | Ranking |
|---|---|---|---|
| `--cards a,b` / `--cards @file` | as given | all | given order |
| `--decklists f…` | union of list cards | cards in ≥ `core_fraction` (0.7) of lists | greedy decklist completion (readiness §4.8) |
| `--archetype NAME` | lists from `deck_library.json` via `archetype_aliases.json` | as decklists | as decklists |
| `--set PREFIX` | `author_set.set_resolver.resolve_set` | set cards in any known list; all if none | decklist completion if lists exist, else card id |

Forms combine (`--set BT27 --decklists …` takes the pool from the set and core/ranking from the lists). Decklists are parsed with the engine's `deck_tools::parse_deck` through the existing PyO3 binding, so every format the desktop app accepts works. `tools.clause_coverage.campaign`'s archetype-only planner is generalised to take a resolved work set rather than duplicated.

**Preflight** (refuses or downgrades, never silently proceeds): official mirror and `cards.json` contain every pool card; the author-set keyword gate passes; per-card DCGO `.cs` presence (absent → that card's clauses are `unavailable`, re-checked on every `resume`); the player manifest's `action_space_hash` matches the engine; both CLIs resolve and run their version command; the harness root is `node status` GO. A failed check names its remedy.

### D4 — Item state machine and terminal states

Items are `card` (implement), `clause` (clause exam) and `interaction` (interaction exam). The states:

```
card:        PENDING → IMPLEMENTING → REVIEW → (PARKED{gap} | IMPLEMENTED)
clause/int:  PENDING → AUTHORING → SIM → ORACLE → (CONFIRMED | DIVERGED → TRIAGE)
TRIAGE:      ours_wrong → FIX → GATE → ORACLE
             dcgo_quirk | unreachable → TERMINATION_CHECK → (TERMINAL | ESCALATED)
             undetermined → ESCALATED
any:         attempt cap or budget → ESCALATED
```

A prompt-sequence failure is split by who disagreed with whom. If the scenario's `expect:` is contradicted by both engines, it goes back to AUTHORING. If our engine and DCGO disagree, it goes to TRIAGE, because missing-decline bugs present exactly as prompt mismatches. The readiness plan-3 introspection supplies which actor resolved each prompt.

Terminal is **adjudicated**: `confirmed`, `dcgo_quirk` with a citation, `unreachable` with a measured reason, `unavailable`, or `escalated` (a human queue entry, which is not adjudicated and blocks readiness). `events.jsonl` records every transition. `resume` rebuilds state from the committed ledgers first and the run's events second, so a crashed run resumes outstanding work only.

### D5 — Two-family termination rule

A call that ends an item without our engine changing (`dcgo_quirk`, `unreachable`, a Q&A classified `textual` or `not_examinable`) needs independent agreement from both model families. Each receives the same packet and does not see the other's answer. If they disagree, or either cannot cite a source, the item is escalated to `qa/card-loop/escalations/<item>.md` with both arguments. `ours_wrong` needs no second opinion: the fix gate (a citation, a test that fails before and passes after, `cards_behavioral` green) is the arbiter. If only one family is available, every terminating call escalates. The loop never degrades to single-model judgment on terminating calls.

*Why.* Terminating calls are where one model rationalises: it turns a divergence into "DCGO quirk", or calls a hard line "unreachable". These are also the calls that remove an item from future scrutiny.

### D6 — The interaction denominator is deterministic

Gating interactions per card are:
1. **Every official Q&A ruling that prints the card** (`data/card_qa.json`), with id `qa:<Q-number>`. One ruling naming several cards is one interaction, adjudicated once and counted for every card.
2. **Generated risk probes**: a rule-based generator over the card's extracted clause text emits `probe:<clause-id>:<family>[:neg]`. The families are versioned (`families@1`):
   - `optional_decline` ("you may", "up to", Opt-cost→Mand keywords) — the decline path is offered and honoured;
   - `scope` ("this Digimon" vs "1 of your Digimon" vs "all") — a negative probe asserts that a non-target permanent is unaffected;
   - `once_per_turn_multi` (`[Once Per Turn]` with two copies in play);
   - `would_replacement` ("when … would");
   - `granted_keyword` (granted keywords carry their triggers);
   - `leave_play` (`[On Deletion]`, `<Decode>`, `<Fortitude>`; post-trash reads, rule 25);
   - `immunity` ("isn't affected by", `<Progress>`);
   - `timing_gate` (`[Your Turn]` / `[Opponent's Turn]` — a negative probe on the wrong turn).

Both inputs are pure functions of committed data, so the denominator is reproducible. `python -m tools.card_loop.interactions --check` fails CI on drift, mirroring the readiness generator.

Model-authored combo interactions from `qa/archetype-qa/*-model.md` are examined and reported but **do not gate**: a model-written list is not a stable denominator.

**Q&A classification precedes examination.** A worker classifies each ruling as:
- `behavioral` — exam it;
- `textual` — a printed-data clarification checked against card data, adjudicated without the oracle;
- `not_examinable` — about tournament procedure, or a state no legal line can reach.

`textual` and `not_examinable` are terminating calls, so D5 applies.

**Adding a family is a promotion, not an edit.** A new family version gates only after it is explicitly promoted, because promotion drops affected cards out of readiness.

*Alternatives.* Gate on model-authored "critical interactions": rejected, because the gate would move whenever a prompt changed. Gate on Q&A only: rejected, because most past bugs (scope, decline) have no ruling.

### D7 — Three-way comparison for Q&A interactions

A Q&A-sourced scenario carries the ruling as an `expect_ruling:` block: an assertion encoding the publisher's answer, with the Q-number. One family authors it and the other must agree that it encodes the answer (D5's packet discipline), because this assertion is the expected value.

| ours = DCGO | ours = ruling | Outcome |
|---|---|---|
| yes | yes | `confirmed` |
| yes | no | `ours_wrong` (fix gate); DCGO divergence from the ruling logged |
| no | yes | `dcgo_quirk`, citation `qa:<Q>`; still two-family checked |
| no | no | `ours_wrong` |

Parity with DCGO is the aspiration: every `dcgo_quirk` and "DCGO disagrees with the ruling" row is also exported as a candidate patch to our DCGO fork. Fixing it there, where it is our call, restores parity and makes those recordings usable for replay and behaviour cloning. This change only produces the list; the mod work is separate and needs a Unity rebuild.

### D8 — Scenario and verdict schema extension

`Scenario` gains optional `covers: [clause ids]` (defaults to `[clause]`) and `interaction: {id, source, kind: positive|negative}`. Existing scenarios stay valid unchanged. `exam_validate` rejects an `interaction.id` absent from the denominator, the same orphan rule clauses have.

Per-card verdict files move to `version: 2` with an `interactions` map beside `clauses`. A shared Q&A verdict is written into every card file that prints it in the same commit. The loader accepts v1. *Alternative.* One file per interaction: rejected, because ~10k small files lose the "a card is the unit of a file" merge property the fleet ledger relies on. Revisit only if shared-ruling writes conflict in practice.

### D9 — Readiness extension

After readiness plan 2 lands, its generator additionally requires every gating interaction of the card to be adjudicated (D4 terminal states minus `escalated`). Vanilla cards with no rulings and no probes stay `ready` by construction. The training gate itself is unchanged: it reads the artifact.

### D10 — Worker adapter contract

A worker takes a packet and returns `{status, result, artifacts, usage}`:
- the packet holds stage, item, attempt_id, schema path, worktree path, references, a budget and the model/effort;
- `artifacts` is the worktree diff plus a manifest, per `harden-card-authoring-pipeline` D1;
- `usage` holds tokens, USD (Claude) or tokens priced from a config table (Codex), and wall time.

| | Claude | Codex |
|---|---|---|
| Invocation | `claude -p --output-format json --json-schema <file> --model <m> --max-budget-usd <b> --permission-mode <non-interactive mode> --add-dir <read-only bases>` | `codex exec -C <worktree> -s workspace-write --add-dir <cargo target> --add-dir <sccache> --output-schema <file> -o <result.json> --json -m <m> -c model_reasoning_effort=<e> -c approval_policy=never` |
| Locating the binary | `claude` on PATH | `codex.exe` inside `(Get-AppxPackage OpenAI.Codex*).InstallLocation`, resolved at runtime because the path changes per Store update |

- The sandbox is always explicit, and `danger-full-access` is refused.
- The driver creates worktrees itself (`git worktree add` at a pinned base sha), with a per-worktree `CARGO_TARGET_DIR` under `D:\cargo-target` (rule 31), and keeps them warm in a pool. It does not use either CLI's `--worktree`, so base pinning and target-dir isolation are identical across vendors.
- Retry policy: transient errors back off; schema-invalid output retries once with the validation error appended; quota or credit exhaustion stops that vendor for the run and the router re-routes.
- A `fake` worker replays canned results for driver tests.

### D11 — Attempt ledger, provenance and corrections

`qa/card-loop/attempts.jsonl` is committed and append-only with a `merge=union` driver. One row per worker call: `{attempt_id, run_id, ts, stage, item, model, effort, prompt_version, assignment: routed|explore, outcome, usage}`.

Provenance:
- Every record a worker produces (YAML header comment, scenario `meta.produced_by`, verdict/triage `produced_by`) carries its `attempt_id`.
- Every driver commit carries `Loop-Attempt:` and `Loop-Model:` trailers.

A correction event `{corrected_attempt, by: attempt|human, kind, latency}` is written when:
- a reviewer rejects or a gate fails (immediate);
- a later attempt or a human edits lines or records whose blame resolves to an earlier attempt (late);
- a terminating verdict is overturned (late);
- an escalation resolves against a worker's call (late).

| Stage | Immediate | Late |
|---|---|---|
| implement | review reject, `cards_behavioral` red | `ours_wrong` blamed on its YAML |
| author | validate/sim fail, scenario-caused oracle round trips | an interaction exam shows the "confirmed" line never exercised the clause |
| triage / classify | other family disagrees | human override, `dcgo_quirk`→`ours_wrong` flip |
| fix | gate fail, review changes requested | revert, rejected branch, later regression |
| interaction | — | false alarms (claimed finding was a scenario error); yield = confirmed bugs per $ |

### D12 — Scorecard and routing

`card-loop report models` gives, per (stage × model × prompt_version): attempts, first-pass acceptance, immediate and late correction rates (late weighted ×3 by default), cost per accepted-and-not-later-corrected unit, and Wilson intervals.

**Judge the judge.** A random sample of accepted outputs per stage (rate is an open question) goes to `card-loop audit next`, where a human marks agree or disagree. That gives each model's precision as a reviewer and corrects for a lenient reviewer flattering its author.

**Fair comparison.** A seeded hash of the item id sends ~20% of each stage's items to the non-routed family (`assignment: explore`). That keeps comparisons unconfounded by item difficulty.

**Routing.** Per stage, the cheapest model by corrected-cost wins once both cells have ≥30 samples and their intervals separate. Until then the config default applies:
- Claude authors, implements and triages;
- Codex reviews and gives the second termination opinion, and takes engine-gap fixes;
- each stage cross-assigns 20% to the other family.

Workers never see the scorecard.

### D13 — Concurrency, merges and stopping

- **Concurrency.** Authoring workers run in parallel (default 3). The oracle is serialised by the node's queue (≤2 players).
- **Merges.** The driver merges worker diffs one at a time onto a run branch, through `merge_wave.py` semantics: apply, assert module registration, run the scoped suite via the verification ladder's `impact_scope`. Engine-touching diffs go on a separate `card-loop/<run>/engine-<gap>` branch. The driver never pushes to `main`. A run ends with a PR or branch for human merge.
- **Stopping.**
  - per-run USD cap;
  - per-item attempt caps (author 3, fix 2, one triage per family);
  - wall-clock cap;
  - plateau rule: no new adjudication in K = 10 consecutive completed attempts;
  - a stop writes state and a report whose first line names `unmeasured`, `escalated` and `unavailable` counts.

### D14 — Prompts reuse contracts, not skills

Stage prompts are narrow templates under `code/tools/card_loop/prompts/`, each with a `version:` header. They point at the exam MCP (`exam_authoring_guide`, `exam_validate`, `exam_keyword_brief`), the scenario library, `docs/digimon-rules/` and the DSL test API, instead of embedding those contracts. Workers do not invoke orchestrator skills (`/batch-implement-cards-rust-dsl`, `/archetype-campaign`), because those spawn their own sub-agents and re-create LLM orchestration inside a worker.

## Risks / Trade-offs

- **[Both families agree on a wrong terminating call]** → a citation is required (PDF §, DCGO file:line, or `qa:<Q>`); the human audit sample; late overturns are recorded and penalise both.
- **[Gating on interactions shrinks the trainable pool further]** → accepted by the user (accuracy first). The ranking prefers interactions on cards that complete the most decklists; vanilla and no-ruling cards are unaffected.
- **[The Q&A volume is large (likely several thousand rulings)]** → only work-set cards are examined; dedupe by Q-number across reprints; `textual` / `not_examinable` classification avoids oracle spend; costs are tracked per interaction.
- **[A model-authored `expect_ruling:` misencodes the publisher's answer]** → a second family must agree it encodes the answer; human audit; Q-number cited for spot checks.
- **[Promoting a new risk family silently drops cards from readiness]** → families are versioned and gate only on explicit promotion; the promotion report lists cards that would drop.
- **[Codex runs unsandboxed because of the user's global config]** → explicit `-s workspace-write` and `-c approval_policy` on every call; preflight rejects a resolved `danger-full-access`; base DCGO is passed only as read-only context.
- **[Disk and build pressure from N warm worktrees each with a cargo target]** → bounded pool (default 3), reuse across items, sccache, target dirs on D:.
- **[The readiness plans slip]** → the independent parts are useful on their own: the resolver feeds `/readiness-batch`, workers and scorecard measure any run, and the Q&A mirror and probe generator serve attended exams. Only driver oracle integration waits.
- **[The scorecard is gamed or confounded]** → randomized cross-assignment, prompt-version partitioning, minimum samples before routing changes, and workers blind to metrics.
- **[A new set ships before DCGO supports it]** → its clauses and interactions are `unavailable` (not adjudicated); implementation and sim checks still run; `resume` after a DCGO bump picks them up.

## Migration Plan

1. Land the Q&A mirror (`scrape_official_qa.py` + `data/card_qa.json`). It is additive, and nothing reads it yet.
2. Land the independent parts behind the CLI (resolver/`plan`, workers, ledger/scorecard, interaction schema + generator + denominator `--check`). The v2 verdict loader accepts v1, and existing scenarios are unchanged.
3. After readiness plans 1 and 3: driver oracle integration, `run`/`resume`.
4. After readiness plan 2: the readiness interaction join. This turns the stricter gate on, and the pool drop is expected and reported.
5. Pilot:
   - **Iteration 0:** drain the 211 unrun scenarios (near-zero authoring spend) to prove the oracle path.
   - **Pilot campaign:** one archetype run end to end.
   - Compare per-clause cost with Track A.

**Rollback.** Revert the readiness join to return to clause-only readiness. Every other part is additive tooling.

## Open Questions

- ~~Human audit sample rate~~ — **decided 2026-10-04: 5%** of accepted outputs per stage (`LoopConfig.audit_rate`).
- **Q&A interactions on cards with no DCGO script.** Should a two-way (ours vs ruling) comparison count as adjudicated, or stay `unavailable` until DCGO supports the card?
- **Default concurrency and per-run budget** for unattended overnight runs on the single local node.
- **When interaction gating turns on relative to clause-only readiness.** Same release as readiness plan 2, or a follow-up once the first Q&A pass has run?
