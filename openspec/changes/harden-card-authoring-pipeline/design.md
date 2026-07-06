# Design: Harden the card-authoring pipeline

## Context

The pipeline's current shape (per the store-champs campaign and `reference_card_impl_workflow_lessons` memory): Sonnet-class implementers in isolated worktrees return FULL yaml/test bodies via structured output; Opus reviewers judge the embedded bodies; the orchestrator hand-merges (entity-unescape → write → register → verdict → suite) and hand-fixes small review directives. Failure modes observed at rate: HTML-entity-escaped bodies, prose-instead-of-code returns (EX7-048), placeholder/false-green outputs, stale worktree bases (5×), ingest override regressions, missed module registration (dead never-compiled test files), full relaunches after transient API failures, and a gap tracker that has duplicated itself wholesale.

EX12 adds: no DCGO C#, no official bundles, paraphrased API text — the card scan is the only printed authority, and today every agent reads it independently.

## Goals / Non-Goals

**Goals:**
- Remove the transport-class failure modes structurally (not by prompt discipline).
- Make wave merging a single deterministic, asserting command.
- Establish one canonical printed-text authority per card that all agents cite.
- Make ingest override-preserving by construction.
- Cut review/fix agent spend without weakening the omission-catching review.

**Non-Goals:**
- Changing the TDD/no-approximations authoring standards or the reviewer's mandate.
- The testing/verification ladder (separate change: `establish-verification-ladder`).
- Rebuilding `card_official.json`/bundles from the official DB (blocked on the site restructure; the printed.md corpus is the interim authority).
- Retiring the structured-output path for *metadata* (verdicts, notes, manifests stay structured — only card/test bodies move to files).

## Decisions

**D1 — Transport: worktree diffs + manifest, not embedded bodies.**
Workers keep isolation and keep the honesty contract, but the deliverable becomes files at canonical paths in their worktree plus a structured manifest `{card, verdict, files: [{path, sha256}], test_result_lines, notes, gaps}`. Orchestrator merge = `git -C <wt> add -A && git -C <wt> diff <base> --binary` filtered to manifest paths, applied 3-way. Reviewers get the worktree path read-only and diff it themselves — "file not on disk" rejections stay impossible because the files demonstrably exist where the reviewer looks. Alternative considered: keep embedded bodies + harden unescaping — rejected; it patches one symptom of a lossy channel that has now failed three different ways.

**D2 — `merge_wave.py` owns the merge invariants.**
Inputs: results JSON + approved-card list + expected base sha. Steps (each asserted, failing loudly): apply per-card diffs; verify YAML parses via the pack build; verify `mod <card>;` registration and set registration in `main.rs` (add if missing); verify the test module contributes >0 tests to the binary (kills the dead-module class); update `validated_cards_dsl.json`; run the scoped behavioral filter; emit a wave summary table. Idempotent — re-running on a merged wave is a no-op.

**D3 — Printed-text extraction is a corpus build, not an agent habit.**
One extraction wave per set: a vision agent per card reads the scan and emits `<ID>.printed.md` (verbatim effect/inherited/security text, keywords, digivolve circles with colors/levels/costs, DUAL faces, alt-play boxes), plus a rulings section mirrored from the card's wiki page via the browser path. A second-pass spot-checker samples ~20% against the scans. The file is committed and becomes the cited authority in implementer/reviewer prompts (scan stays the tiebreak for disputes). digimoncard.io JSON is demoted to stats-only. Trigger: part of set onboarding, right after image mirroring; EX12's two slices first.

**D4 — Ingest ends with overrides, guarded.**
`merge_diff` (and the legacy full-ingest path) call `apply_overrides()` before writing `cards.json`. New guard in the merge: for any card with an override entry, the merged record's `evo_costs`/`type_eng` must be a superset of the override's — otherwise abort with a diff. Keyword-gate ordering fixed by making the gate helper refresh lexicons itself when the set being gated is newer than the lexicon build.

**D5 — Retry inside the workflow scripts.**
A shared `tryAgent(promptFn, opts, {retries: 1, backoffMs})` wrapper in the campaign scripts: on `null` (terminal API error), wait and retry once; on a second failure, record `AGENT-FAILED` and continue the wave (the resume path already handles re-runs). Credit-exhaustion detection: if ≥3 consecutive failures arrive within the backoff window, abort the whole run immediately so resume restarts cleanly.

**D6 — Review economics: auto-fix stage + batch review, review itself unchanged.**
The reviewer schema gains `fix_class: mechanical | structural` per issue. Mechanical directives (single key/value flips, verb swaps with named replacements) are applied by `merge_wave.py --apply-fixes` and the scoped suite re-run; structural rejections dispatch fix agents as today. Simple cards (eggs, keyword-only bodies, ≤2 clauses) are grouped 3–4 per Opus reviewer call; complex cards keep dedicated reviewers. The per-card scan-grounded review itself — the omission catcher — is not weakened.

## Risks / Trade-offs

- **[Worktree diffs can carry unintended files]** → manifest-path filtering at apply time (`git apply --include=<path>` per manifest entry); anything outside the manifest is reported, never applied.
- **[Worker worktrees are cleaned before review reads them]** → wave ordering: reviewers run before any worktree cleanup; `merge_wave.py` performs cleanup only after verdicts are recorded.
- **[Extraction pass itself hallucinates text]** → verbatim-only rule (no paraphrase), spot-check pass, and the scan remains authoritative on any dispute; extraction errors found later are fixed in the .printed.md via the normal review loop.
- **[Auto-fix misapplies a directive]** → auto-fix only for `fix_class: mechanical` with a machine-checkable target (exact old value present); suite re-run gates the result; anything ambiguous falls through to a fix agent.
- **[Batch review dilutes attention on a sleeper-complex "simple" card]** → complexity classification errs complex (clause count + printed-text length threshold); reviewer can escalate any card in the batch to a solo re-review.

## Migration Plan

Tooling-first, no flag days: land `merge_wave.py` + sync script + ingest guards (usable immediately), then update the skill/workflow prompt templates to the manifest transport, then run EX12 extraction. Old-style embedded-body results remain mergeable via the existing manual path during transition. Rollback = revert prompt templates; tools are additive.

## Open Questions

- Whether the manifest transport also carries reviewer outputs (fix diffs as patches rather than directives) — deferred until the auto-fix stage proves itself.
- Where `.printed.md` extraction QA thresholds land (sample rate, mismatch tolerance) — tune on the EX12 run.
