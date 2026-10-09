## Why

RL training is about to admit a decklist only when every card in it is adjudicated against the DCGO oracle (the approved oracle-readiness design, 2026-10-04). That gate collapses the trainable pool (est. 1519 decklists → ≤101), so exam throughput is now the bottleneck — and today every campaign is a long, human-attended LLM session that orchestrates itself (orchestration alone was $704 of a $4,210 campaign; Track A measured ~70% of exam spend as re-read context). Clause-level "confirmed" is also too weak a bar: every faithfulness bug the Toho campaign found was an *interaction* (cross-permanent firing, a decline never offered, `<Delay>` windows, granted-keyword triggers), which a single happy-path line per clause cannot see. We need a durable, resumable tool that turns a card list, a set, or a set of decklists into adjudicated cards — at scale, across future sets — and that uses both available model families (Claude, Codex) where each is measurably best.

## What Changes

- **New `card-loop` tool** (`python -m tools.card_loop plan|run|resume|status|report`): a Python driver that owns the state machine, the oracle queue, the gates and the ledger, and calls `claude -p` / `codex exec` only for steps that need judgment, each with a small schema-validated task packet in a fresh worktree.
- **One work set from three input forms**: `--cards`, `--set`, `--decklists` / `--archetype` (combinable) resolve to `{pool, core, ranking}`, with a new-set preflight (data freshness, keyword gate, per-card DCGO script presence, player action-space hash) so the tool keeps working on future sets.
- **Adjudication, not confirmation, is the finish line**: per clause and per interaction the loop ends in `confirmed`, a cited `dcgo_quirk` triage, a measured `unreachable`, `unavailable`, or escalation to a human. Terminating calls (`dcgo_quirk`, `unreachable`) require independent agreement from both model families.
- **Interaction exams (new stage)**: adversarially authored, multi-clause / multi-card exams drawn from official Q&A rulings, generated risk-family probes and negative probes (the clause must *not* fire). Q&A-sourced exams carry the publisher's answer as a third oracle. **Gating interactions become part of training readiness** — a card is `ready` only when its clauses *and* its gating interactions are adjudicated.
- **Complete official Q&A mirror**: `data/card_qa.json` keyed by Q-number (question, answer, date, card ids), scraped by `code/tools/scrape_official_qa.py`. The existing `card_official.json` `qa` field kept one answer per card and no questions.
- **Model scorecard**: every worker attempt is ledgered with provenance (`produced_by`, `Loop-Attempt:` commit trailers) so later corrections are attributed; per-stage correction rates, a randomized cross-assignment share and a human audit sample drive which model each stage routes to.
- **Exam scenario schema extension**: scenarios gain `covers: [clause ids]` and an `interaction: {id, source, kind}` block (today a scenario binds exactly one card and one clause).

## Capabilities

### New Capabilities
- `card-loop-work-set`: resolving cards / sets / decklists / archetypes into a work set with core and ranking; new-set preflight; resumable run lifecycle and run directories.
- `card-loop-orchestration`: the driver's per-item state machine, stage order, adjudicated terminal states, two-family termination rule, gap lane, fix gate, budgets, attempt caps and plateau stop, human escalation queue.
- `agent-cli-workers`: the worker adapter contract for `claude -p` and `codex exec` — task packets in, schema-validated results out, cost/usage capture, sandboxing, worktree isolation, retry/backoff.
- `interaction-exams`: the gating interaction denominator (Q&A rulings + generated risk/negative probes), scenario schema extension, three-way comparison, adversarial authorship, and the extension of training readiness to interactions.
- `model-scorecard`: attempt ledger, artifact provenance, correction signals per stage, audit sample, randomized cross-assignment and per-stage routing.

### Modified Capabilities
- `official-card-data-sourcing`: adds a requirement that official Q&A rulings are mirrored completely (Q-number, question, answer, date, every card that prints them) rather than as a single answer string per card.

## Impact

- **New code**: `code/tools/card_loop/` (driver, resolver, workers, ledger, scorecard, generators, prompts), `code/tools/scrape_official_qa.py`; tests under `code/tests/tools/`.
- **New data**: `data/card_qa.json` (committed); interaction specs + verdicts alongside the existing exam ledger (`qa/qa-reports/exam-verdicts/`); attempt ledger; run dirs under `runs/card-loop/` (git-ignored).
- **Modified**: `code/tools/dcgo-harness/src/exam/scenario.rs` (+ binding / validate / differ) for `covers` and `interaction`; `tools.clause_coverage` binding and the readiness generator for interaction verdicts; `tools.clause_coverage.campaign` generalised beyond `--archetype`.
- **Depends on** the oracle-readiness plans (`docs/superpowers/specs/2026-10-04-dcgo-oracle-readiness-design.md` + `docs/superpowers/plans/2026-10-04-oracle-readiness-*.md`; Plans 1 and 3 built on this change's branch on 2026-10-05): one-call `exam --oracle`, structured triage, scenario library, readiness artifact + training gate, readiness plan, cost report. This change references them and does not re-specify them; the driver's oracle integration waits for them, while the resolver, workers, ledger/scorecard and interaction schema/generators are built in parallel.
- **Aligns with** `harden-card-authoring-pipeline` (worker diffs + manifest, deterministic `merge_wave.py`, retry with fast abort on credit exhaustion, orchestrator-only tracker writes) — reused, not duplicated.
- **External**: two CLIs on the operator's machine (Claude Code; Codex bundled with the Store app); the DCGO oracle node per `docs/runbooks/oracle-node.md`. No hosted surface, DB, or production build is touched.
- **Pool impact**: gating on interactions shrinks the trainable pool further until interactions are examined — accepted (accuracy over pool recovery).
