Groups 1–5 do not depend on the oracle-readiness plans and can proceed in parallel (group 1 first). Groups 6–7 wait on readiness plans 1–3 and 2 respectively (branch `worktree-dcgo-effect-translation`).

## 1. Official Q&A mirror (first)

- [x] 1.1 `code/tools/scrape_official_qa.py` parsing `cardFaqListItem` (Q-number, date, question, answer), per-printing dedupe, exact card-number segmentation, not-found → `failed`, conflict recording, resume + checkpoint, sorted output; tests in `code/tests/tools/test_scrape_official_qa.py`
- [ ] 1.2 Run the full scrape (`--all`) into `data/card_qa.json`; review `failed` and `conflicts`; commit the dataset
- [ ] 1.3 Report the real ruling count and per-card distribution vs the old `card_official.json` `qa` field (one answer per card) in the change notes
- [ ] 1.4 Document the tool and dataset in `docs/TOOLS.md` and note in CLAUDE.md "Printed card data" that `data/card_qa.json` (not `card_official.json`'s `qa`) is the ruling source

## 2. Work-set resolution and preflight (independent)

- [x] 2.1 Scaffold `code/tools/card_loop/` (package, `__main__.py` lazy CLI dispatcher, `config.py` defaults + TOML overrides, `contracts.py` shared stage/family/item/packet/result types); tests in `test_card_loop_skeleton.py`
- [x] 2.2 `workset.py`: `--cards` (ids, `@file`), `--set` via `author_set.set_resolver`, `--decklists` via the PyO3 `parse_deck`, `--archetype` via `deck_library.json` + `archetype_aliases.json` with near-miss errors; combinable; tests per spec scenario
- [x] 2.3 Core by `core_fraction` and ranking by greedy decklist completion (shared with, or ported from, readiness §4.8 so there is one implementation); tests including the staple-outranks-one-of scenario
- [x] 2.4 Generalise `tools.clause_coverage.campaign` to plan from a resolved work set (keep `--archetype` as a thin wrapper); existing campaign tests stay green
- [x] 2.5 Preflight checks (mirror and `cards.json` coverage, keyword gate, per-card DCGO `.cs` presence → `unavailable`, action-space hash, CLI resolution, `node status`) each with a named remedy; NO-GO blocks worker spend; tests with fixtures
- [x] 2.6 `plan` writes `runs/card-loop/<run-id>/plan.json` (frozen inputs, resolved pool/core/ranking, config hash); add `runs/card-loop/` to `.gitignore`

## 3. Agent CLI workers (independent)

- [ ] 3.1 `workers/base.py`: packet and result dataclasses, stage JSON schemas under `schemas/`, schema validation with one retry carrying the validation error
- [ ] 3.2 `workers/fake.py` replaying canned results, used by all driver tests
- [ ] 3.3 `workers/claude.py`: `claude -p --output-format json --json-schema … --max-budget-usd …` with non-interactive permissions and read-only `--add-dir` for base DCGO / rules; parse result, `total_cost_usd`, usage
- [ ] 3.4 `workers/codex.py`: resolve `codex.exe` from the installed package at runtime; `codex exec -C <wt> -s workspace-write -c approval_policy=never --add-dir <cargo target> --add-dir <sccache> --output-schema … -o … --json`; refuse an effective `danger-full-access`; token usage priced from a config table and marked derived
- [ ] 3.5 Worktree pool: `git worktree add` at a pinned base, per-worktree `CARGO_TARGET_DIR` under `D:\cargo-target`, reset-to-base before each item, bounded size, cleanup
- [ ] 3.6 Retry classes: transient backoff, schema retry, quota/credit exhaustion disables the vendor for the run and notifies the router
- [ ] 3.7 Smoke test each real adapter on a trivial schema'd task (manual, recorded in the change notes with observed cost and latency)

## 4. Attempt ledger, provenance and scorecard (independent)

- [x] 4.1 `qa/card-loop/attempts.jsonl` schema and writer; `merge=union` in `.gitattributes`; reader tolerant of unknown keys
- [x] 4.2 Provenance: `produced_by` on worker-produced records, `Loop-Attempt:` / `Loop-Model:` trailers on driver commits; blame-to-attempt resolver with tests
- [x] 4.3 Correction detectors (review reject, gate fail, blame-attributed later edit, overturned terminating verdict, escalation resolved against a call) emitting immediate/late correction events
- [x] 4.4 Router: per-stage defaults from config, seeded-hash exploration share (default 20%), implementer-excluded routing for interaction exams, minimum-sample + interval-separation rule before switching
- [x] 4.5 `card-loop report models`: per stage × model × prompt version — attempts, first-pass acceptance, immediate/late correction rates, corrected cost per unit, Wilson intervals
- [x] 4.6 `card-loop audit next|record`: random accepted-output sampling at a configurable rate, human verdicts stored as ground truth, reviewer-precision estimate in the report

## 5. Interaction schema, generators and denominator (independent)

- [ ] 5.1 `scenario.rs`: optional `covers` (default `[clause]`) and `interaction {id, source, kind}`; `expect_ruling` block carrying a Q-number; legacy scenarios unchanged; Rust unit tests
- [ ] 5.2 `exam_validate` / binding: orphan-interaction rejection; `covers` ids must all be known clauses
- [ ] 5.3 Verdict store v2: `interactions` map beside `clauses` in per-card files; v1 loader compatibility; shared-ruling writes to every listed card in one commit
- [ ] 5.4 Probe generator `families@1` (optional_decline, scope, once_per_turn_multi, would_replacement, granted_keyword, leave_play, immunity, timing_gate) over extracted clause text, with negative probes; deterministic ids; tests per family on real clause text
- [ ] 5.5 Denominator builder joining `card_qa.json` + probes per card; committed artifact plus `--check` drift mode; family promotion flag and a promotion report listing cards that would drop out of readiness
- [ ] 5.6 Q&A classification packet and schema (`behavioral|textual|not_examinable`) and the `expect_ruling` authoring packet, both marked as terminating calls requiring two families
- [ ] 5.7 Three-way outcome function (ours / DCGO / ruling → verdict per the D7 table) with exhaustive unit tests; DCGO fork-candidate export

## 6. Driver orchestration (after readiness plans 1 and 3)

- [ ] 6.1 Item state machine (`state.py`) with `events.jsonl`, rebuild-from-ledger `resume`, and terminal-state accounting where `escalated` is not adjudicated
- [ ] 6.2 Stage executors: implement (DSL-first per rule 28, reviewer = other family), clause author, interaction author, triage, fix, Q&A classify; prompts under `prompts/` with `version:` headers that reference the exam MCP, scenario library and rules docs instead of embedding them
- [ ] 6.3 Oracle stage via readiness's one-call `exam --oracle` (no model); prompt-failure routing by who disagreed, using readiness's step introspection
- [ ] 6.4 Two-family termination check and the escalation queue `qa/card-loop/escalations/`
- [ ] 6.5 Fix gate (citation, failing-then-passing test, `cards_behavioral` green via the verification ladder's `impact_scope`); engine fixes on `card-loop/<run>/engine-<gap>` branches; never push to `main`
- [ ] 6.6 Gap lane: park on gap id, rank gaps by core clauses blocked, unpark on merge; route gap records through the orchestrator only (align with `harden-card-authoring-pipeline` tracker hygiene)
- [ ] 6.7 Merge integration reusing `harden-card-authoring-pipeline`'s diff + manifest transport and `merge_wave.py` semantics (build it there if still unbuilt; do not fork it)
- [ ] 6.8 Budgets and stopping: USD cap, wall-clock cap, per-item attempt caps, plateau rule; report whose first line names unmeasured / escalated / unavailable counts
- [ ] 6.9 End-to-end driver test on the fake worker covering confirm, diverge→fix, diverge→quirk (agree), diverge→escalate (disagree), park→unpark, resume after a simulated crash

## 7. Readiness extension (after readiness plan 2)

- [ ] 7.1 Readiness generator joins gating interactions: `ready` requires every clause and every gating interaction adjudicated; blockers list interaction ids; vanilla/no-interaction cards unchanged
- [ ] 7.2 Tests for the readiness scenarios (unexamined ruling blocks, shared ruling counts for both cards, escalated does not count)
- [ ] 7.3 Regenerate `data/oracle_readiness.json` and report the pool delta vs clause-only readiness

## 8. Pilot and documentation

- [ ] 8.1 Iteration 0: drain the 211 authored-but-never-oracled scenarios through the driver; record outcomes and any harness issues
- [ ] 8.2 Pilot run on one archetype (`--archetype`) end to end, including Q&A interactions; compare per-clause cost with Track A's $1.72 and record the first scorecard
- [ ] 8.3 `docs/CARD_LOOP.md` operating manual (inputs, preflight, run/resume, escalation handling, audit, scorecard, adding a risk family); link from `docs/INDEX.md` and CLAUDE.md
- [ ] 8.4 Resolve the design's open questions (audit rate, ruling-only adjudication without DCGO, default concurrency/budget, gating switch-on timing) and record the decisions in `design.md`

## 9. Follow-ups found while building groups 2–5

- [ ] 9.1 Keyword-gate false positives make every real pool NO-GO: `link` (still classed a subsystem in the DCGO keyword manifest — blocks Rocks via ST22-09), `delay`, `digi-burst up to`, `sistermon noir` (BT7). Fix the `tools.author_set` lexicon/manifest; preflight meanwhile fails the gate only for cards with no YAML spec (warn otherwise)
- [ ] 9.2 177 cards with printed effects have no per-card DCGO `.cs` (e.g. keyword-only cards such as one carrying only `<Jamming>`). Establish how DCGO implements keyword-only cards before preflight's `.cs` presence check marks them `unavailable`
- [ ] 9.3 Promote module-level defaults proposed by the groups into `LoopConfig`: `dcgo_root`, `node_status_timeout_s` (group 2); `corrections_path`, `audits_path` (group 4); plus any from groups 3 and 5
- [ ] 9.4 The router's scorecard override matches the configured model alias by exact string (`sonnet`; Codex `None` pools all models) — the driver must record the configured alias in attempts, or the scorecard must normalise resolved model names
- [ ] 9.5 Stale-on-main test `test_real_library_reproduces_the_published_toho_figures` (deck library refreshed: Toho Braves now 114 lists, test expects 45) — fails at `ad68542ee` before any card-loop change; fix in its own commit (CLAUDE.md rule 33)
