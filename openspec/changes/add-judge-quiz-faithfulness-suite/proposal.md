## Why

The TCG Judges' Discord publishes a 30-question Digimon TCG rules quiz built entirely from adversarial timing, immunity, and rules-check edge cases — the exact class of interaction the no-approximations policy is most likely to get subtly wrong. Each question is a judge dressing a *rule* in specific cardboard, and each ships an official correct answer with a rules rationale. That makes the quiz a ready-made faithfulness oracle: an independent, authoritative corpus of "here is the board, here is the right outcome" that the Rust engine should reproduce.

Today none of these 30 scenarios are pinned. The quiz references **≈70 distinct cards** (exact IDs frozen in [`card-resolution.md`](./card-resolution.md) from the source PDF); only 7 are confirmed implemented (DSL) against the correct printings, and there are **0 behavioral tests** across the set. Worse, the scenarios probe rules-engine machinery (immunity scope, deferred rules-checks, declare-then-pay cost windows, trigger activation sites, `<Partition>`/DigiXros departure, token lifecycle) where we have only *partial* coverage today — e.g. `mid_attack_security_attack_recompute.rs` covers the security-count rule but nothing pins the declare-then-pay cost window. We do not currently know which of the 30 the engine gets right.

This change turns the quiz into a permanent regression suite using the **real cards the judges chose** (authoring the missing ones in scope), run **discover-then-pin**: tests assert the judge-correct outcome; a failing test is a discovered faithfulness gap that gets logged and fixed, never a weakened assertion.

## What Changes

- **Resolve the corpus (gating spike — DONE).** The exact `card_id` for every card in all 30 questions is pinned in [`card-resolution.md`](./card-resolution.md), read off the PDF card images (these supersede a name-based inventory that guessed wrong printings for most cards). No `BLOCKED-DATA` scenarios remain. Residual spike work is re-deriving implementation status against the correct IDs. Cross-reference each scenario's rationale against `DCGO/` C# as the behavioral tiebreaker.
- **Discovery wave (zero authoring).** For every scenario whose cards are already implemented, encode the quiz board state as a `DebugRunner` behavioral test asserting the judge answer. Run them. PASS → pinned; FAIL → logged to `qa/archetype-qa/engine-gaps.md` as a discovered gap with the DCGO citation.
- **Author the missing cards, cluster by cluster.** The 39 unimplemented cards are authored via the existing `/batch-implement-cards-rust-dsl` TDD pipeline (full faithful card text + per-card behavioral tests), grouped by the rule cluster their quiz question exercises so each wave lands a coherent slice.
- **Encode all 30 quiz scenarios** as cross-card interaction behavioral tests under a new `tests/judge_quiz/` tree, organized by rule cluster (A–G), each test docstring quoting the question, the judge answer, the rules-manual citation, and the DCGO reference.
- **Fix the rules-engine gaps** the discovery wave and authoring surface (TDD), each as its own scoped slice routed through `engine-gaps.md`.
- **Reconcile trackers**: per-question verdict table in a new `qa/qa-reports/judge-quiz.md`, gap entries archived to `qa/resolved-gaps.md`, card verdicts in `validated_cards_dsl.json`.

## Capabilities

### New Capabilities
- `judge-quiz-faithfulness-suite`: A permanent behavioral-test suite reproducing all 30 TCG-Judges'-quiz scenarios with the real referenced cards, each test asserting the official judge-correct outcome; every referenced card faithfully implemented; the rules clusters the quiz probes (immunity scope, deferred rules-check, declare-then-pay cost, trigger activation site, `<Partition>`/DigiXros departure, token lifecycle, zone/keyword scoping) verified; and a per-question verdict ledger reconciled to test reality.

### Modified Capabilities
<!-- None expected. Gap fixes surfaced by the discovery wave may touch existing capabilities (e.g. permanent-deletion-semantics, dedigivolve-resolution-parity, security-card-effects); any such requirement change will be added as a MODIFIED delta to that capability's spec when the specific gap is confirmed during the spike/discovery phase, not pre-emptively. -->

## Impact

- **Tests:** new `code/digimon-engine/tests/judge_quiz/` tree (one module per rule cluster) + new per-card behavioral tests under `tests/cards_behavioral/<set>/` for the 39 authored cards.
- **Card content:** `code/digimon-engine/cards/<set>/*.yaml` — ~39 new card specs (full faithful text, not quiz-scoped subsets).
- **Engine / DSL (Rust):** `code/digimon-engine/src/` and `code/digimon-dsl/src/` — only where the discovery wave or authoring surfaces a genuine gap (candidate hot spots: declare-then-pay cost window, immunity-scope "affects me vs affects the battle", granted-effect ownership, deferred rules-check ordering, token placement lifecycle).
- **Reference:** read-only use of the `DCGO/` submodule as the behavioral tiebreaker per CLAUDE.md source priority; no DCGO edits.
- **Trackers:** new `qa/qa-reports/judge-quiz.md` (per-question verdict ledger); `qa/qa-reports/validated_cards_dsl.json`, `qa/archetype-qa/engine-gaps.md`, `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md` reconciled.
- **No RL contract change** beyond an additive pending-selection sub-range if a gap fix requires one (handled per the existing additive action-space contract; `docs/ACTION_SPEC.md` updated if so).
- **Scope multiplier risk:** authoring 39 full cards is the bulk of the effort and dwarfs the test-writing. The plan is phased and cluster-gated so value (discovered gaps + pinned scenarios) lands incrementally even if the full card pool isn't completed in one pass.
