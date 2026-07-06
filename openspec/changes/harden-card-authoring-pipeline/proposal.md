# Proposal: Harden the card-authoring pipeline

## Why

The store-champs June-2026 campaign (103 cards) proved the multi-agent authoring pipeline works but surfaced systematic failure modes that cost real orchestrator time on every wave: card bodies shipped through structured output arrived HTML-entity-escaped or as prose-instead-of-code, agents forked from stale worktree bases five separate times, the ingest silently regressed override-corrected data, the merge ritual was hand-rolled (and the one skipped step — module registration — produced dead test files that "passed" while never compiling), and transient API failures cost full relaunch round-trips. EX12 raises the stakes: it is the first set with **no DCGO oracle and no official-DB bundles**, so every agent independently re-reads card scans (~160 redundant vision reads for the two slices) against paraphrased API text. These fixes should land before the EX12 waves so the new set is authored on the hardened path.

## What Changes

- **Worker output transport moves from embedded bodies to worktree diffs**: workers write real files in their isolated worktrees and return a manifest (paths, shas, pasted `test result:` lines); the orchestrator merges via `git diff <base> --binary | git apply --3way`. Reviewers read the worker worktree read-only. Eliminates entity-escaping, prose-instead-of-code, placeholder bodies, and truncation.
- **`merge_wave.py` tool**: one deterministic command consuming a wave's results manifest — applies diffs, wires `mod.rs`/`main.rs` registration (asserted, not assumed), updates verdicts, runs the scoped suite, emits a summary.
- **Printed-text extraction pass**: a one-time vision pass per card producing committed `cards/<set>/<ID>.printed.md` (verbatim clause text, digivolve circles, rulings mirrored from the wiki) — the canonical printed authority downstream agents cite instead of each re-reading the scan. Built for EX12 first; backfills other sets opportunistically.
- **Agent base pinning**: a shared `scripts/agent_worktree_sync.sh <expected-sha>` invoked by every worker preamble (replacing per-prompt copy-pasted reset/cp instructions); orchestrator-side base verification before dispatch.
- **Ingest integrity**: set pull/merge always ends with `apply_overrides()`; a guard refuses merges that shrink `evo_costs` (or drop trait entries) for any card present in `card_overrides.json`; lexicon refresh ordered before the keyword gate.
- **Workflow-script resilience**: per-item retry-with-backoff around `agent()` calls (absorbs rate-limit waves); fail-fast on credit exhaustion instead of burning all batches.
- **Review economics**: an orchestrator auto-fix stage applies single-edit reviewer directives (key flips, verb swaps) directly and reruns the suite; batch-review for simple cards (one reviewer per 3–4 eggs/vanilla Lv.3s); per-card review retained for complex cards.
- **Tracker hygiene**: dedupe `qa/dsl-vocab-gaps.md` (currently contains two full copies of itself); trackers become orchestrator-write-only — workers report, never edit.

## Capabilities

### New Capabilities
- `authoring-wave-pipeline`: worker transport contract (worktree diffs + manifest + honesty contract), wave merge tooling, auto-fix stage, batch-review policy, retry semantics, and base pinning for authoring waves.
- `printed-text-extraction`: scan-derived per-card printed-authority files with extraction QA and rulings mirroring.
- `set-ingest-integrity`: override-preserving pull/merge with regression guards and gate ordering.
- `qa-tracker-hygiene`: single-copy gap trackers with orchestrator-only write discipline.

### Modified Capabilities
<!-- none — the changes are pipeline/tooling; existing spec-level card-authoring requirements (DSL vocabulary, substrate integrity) are untouched -->

## Impact

- **Tools**: `code/tools/author_set/` (merge_wave.py, extraction runner, manifest schema), `code/tools/ingest_cards.py` + `tools/author_set/ingest_diff.py` (override auto-apply + guards), `scripts/agent_worktree_sync.sh` (new).
- **Skills/workflow scripts**: `/batch-implement-cards-rust-dsl`, `/implement-rust-dsl-archetype`, `/author-set` prompt templates and the campaign workflow scripts (transport + preamble + retry changes).
- **QA artifacts**: `qa/dsl-vocab-gaps.md` (dedupe), tracker write conventions documented in the skills.
- **Data**: `code/digimon-engine/cards/<set>/<ID>.printed.md` (new committed artifact class, EX12 first).
- **Dependencies**: none new; the extraction pass uses the existing lookup-skill image cache and the Playwright browser path for rulings.
