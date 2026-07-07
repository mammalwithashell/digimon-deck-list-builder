# Tasks: Harden the card-authoring pipeline

## 1. Merge tooling + base pinning (usable immediately)

- [ ] 1.1 `scripts/agent_worktree_sync.sh <expected-sha>`: verify/hard-reset the agent worktree base; fail fast on mismatch; document the one-line preamble invocation
- [ ] 1.2 `code/tools/author_set/merge_wave.py`: manifest-driven apply (path-filtered 3-way diffs) → pack-compile check → registration assertion (mod wiring + >0 discovered tests) → verdict writes → scoped suite → wave summary; idempotent re-run
- [ ] 1.3 `merge_wave.py --apply-fixes`: mechanical-directive auto-apply (exact-target match only) + scoped suite re-run; ambiguous directives routed to fix agents
- [ ] 1.4 Unit tests for merge_wave (fixture wave: approved card, out-of-manifest file, dead-module case, idempotency)

## 2. Worker transport contract

- [ ] 2.1 Define the worker manifest schema (card, verdict, files+sha256, test evidence lines, notes, gaps) and add it to the workflow scripts' structured-output schemas
- [ ] 2.2 Update implementer prompt templates (batch skill + campaign workflow scripts): write files at canonical paths, return manifest, no embedded bodies; keep the honesty contract
- [ ] 2.3 Update reviewer prompt templates: read the worker worktree read-only; file-not-on-main-tree is never a rejection; add `fix_class: mechanical|structural` per issue
- [ ] 2.4 Sequence worktree cleanup after review+merge in the orchestrator flow; document in the skills

## 3. Workflow-script resilience

- [ ] 3.1 `tryAgent` retry-with-backoff wrapper in the campaign workflow scripts (1 retry on transient failure; AGENT-FAILED on second)
- [ ] 3.2 Credit-exhaustion fast-abort (≥3 consecutive failures within the backoff window aborts the run for clean resume)

## 4. Ingest integrity

- [ ] 4.1 `apply_overrides()` at the end of `merge_diff` and the legacy ingest CLI path
- [ ] 4.2 Override-regression guard (evo_costs / type_eng shrink vs override → abort with diff report) + unit tests
- [ ] 4.3 Keyword-gate helper refreshes lexicons when they predate the gated set's ingest
- [ ] 4.4 DUAL override map: require a face-verification citation comment per entry (lint or convention doc)

## 5. Printed-text extraction (EX12 first)

- [ ] 5.1 Extraction runner: vision agent per card → `cards/<set>/<ID>.printed.md` (verbatim boxes, circles, DUAL faces, keywords) + rulings mirror via the browser path
- [ ] 5.2 Spot-check pass (≥1 in 5 sampled vs scans) gating corpus adoption
- [ ] 5.3 Run extraction over the 54 EX12 Shambala/VB cards; commit the corpus
- [ ] 5.4 Update authoring prompt templates to cite `.printed.md` as printed authority (scan = tiebreak; JSON = stats only, divergences flagged in manifests)

## 6. Tracker hygiene

- [ ] 6.1 Dedupe `qa/dsl-vocab-gaps.md` to a single copy; add the re-duplication check
- [ ] 6.2 Document orchestrator-write-only tracker discipline in the skills; workers/reviewers report via manifests

## 7. Validation

- [ ] 7.1 Dry-run the hardened pipeline end-to-end on a small real wave (e.g. EX12 wave S1 eggs+Lv3s) and fix friction before the full campaign relies on it
