---
name: implement-rust-dsl-archetype
description: Use when Codex needs to implement a Digimon archetype, deck, card group, explicit card list, or DSL test pool as Rust engine YAML DSL cards. Always resolve the full requested card pool, process all unblocked cards in batches of 4 with parallel per-card implementer/auditor subagents plus batch reviewers, continue across batches without waiting for user prodding, and report only after the requested pool is complete or a true blocker prevents further progress.
---

# Implement Rust DSL Archetype

## Operating Contract

Convert the requested card pool into tested Rust engine YAML DSL cards. Build the complete queue first, then execute every unblocked card in batches of exactly 4 where possible. Do not stop after the first batch to ask whether to continue.

Use subagents as the normal execution model: one fresh implementer/auditor per non-skipped card and at least one reviewer per completed batch. If subagents are unavailable, report that limitation before implementation and ask whether to proceed locally.

Preserve no approximations: every gameplay choice must go through engine actions or `PendingSelection`, and every behavior change must start with a failing Rust behavioral test.

Required sub-skills: `superpowers:test-driven-development`, `superpowers:subagent-driven-development`, and `superpowers:verification-before-completion`. Use `assess-rust-engine-archetype` for card-pool and gap-resolution evidence.

## Required Docs

Read only what is needed before dispatching implementation work:

- `AGENTS.md` and `CLAUDE.md`
- `docs/RUST_DSL_AGENT_GUIDE.md`
- `docs/RUST_DSL_TEST_API.md`
- `docs/RUST_ENGINE_API.md`
- `docs/ACTION_SPEC.md` and `docs/TENSOR_SPEC.md`
- `docs/RUST_ENGINE_GAPS.md`
- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/engine-gaps.md`

Load these references as soon as they become relevant:

- `references/card-pool-resolution.md`: archetype/deck/card-list resolution and queue shape.
- `references/subagent-tdd-workflow.md`: batch orchestration, subagent roles, prompts, merge, and verification.

## Defaults

- Default batch size: `4`.
- Modes:
  - `IMPLEMENT`: no production YAML exists; write failing tests before YAML.
  - `AUDIT`: YAML exists; verify printed-text faithfulness and add missing tests. Report drift before changing behavior.
  - `SKIP`: prior tracker verdict is complete or the user excluded existing YAML.
  - `BLOCKED`: faithful implementation needs a reusable DSL, engine, rules, test, or data gap first.
- Batch related cards together, then fill remaining slots by stable card-ID order.
- The main orchestrator owns shared files: `main.rs`, `mod.rs`, QA trackers, gap trackers, and summaries. Workers own only assigned YAML and per-card test files.

## Workflow

1. Announce this skill and the required sub-skills.
2. Resolve the full target pool using exact card IDs or decklists first, then `references/card-pool-resolution.md`.
3. Run a readiness pass before code edits. Classify every card/effect as `ready`, `dsl-gap`, `engine-gap`, `rules-gap`, `test-gap`, or `data-gap`.
4. Build the complete batch plan: totals by mode, blocked prerequisites, and all batches of up to 4 cards. If the user requested report-only planning, stop here.
5. Pre-wire test discovery for every non-skipped card before dispatch.
6. Execute all batches sequentially using `references/subagent-tdd-workflow.md`.
   - Dispatch per-card workers in parallel inside each batch.
   - Run batch review after workers return.
   - Fix review findings and targeted test failures before advancing.
   - Continue automatically to the next batch until the planned queue is exhausted or a true stop condition applies.
7. Update trackers after each batch, but keep interim user-facing updates brief. Do not ask the user to confirm the next batch.
8. Verify before completion with targeted Rust tests, broader suites for any touched shared surface, full engine tests after a full archetype or multi-batch run unless scoped narrower, and `git diff --check`.

## Task Requirements

Every task must include:

- Printed text evidence from `data/cards.json`.
- Existing YAML/DSL examples to mimic when possible.
- A failing test under `code/digimon-engine/tests/` before YAML or engine edits.
- Action-mask or pending-selection assertions for every player-visible choice.
- Negative tests for filters, missing targets, optional decline, and once-per-turn where relevant.
- Tracker edits that are precise about closed, partial, and still-open sub-shapes.

Every batch must include:

- Mode counts: `IMPLEMENT`, `AUDIT`, `SKIP`, and `BLOCKED`.
- One reviewer verdict per processed card.
- Targeted test output for the changed set/card filter.
- Tracker updates for `IMPLEMENTED`, `PARTIAL`, `AUDITED-OK`, `AUDITED-MISSING-TESTS`, `AUDITED-DRIFT`, and `BLOCKED`.
- Gap-kind discipline for blocked cards: `engine`, `dsl`, `hybrid`, `rules`, `test`, or `data`.

## Stop Conditions

Stop and report a blocker instead of implementing when:

- A new player-visible choice cannot fit current pending-selection/action-mask surfaces.
- A card needs an unimplemented timing, replacement window, zone move, formula, or event payload.
- The only apparent implementation is `process: []`, card-local raw Rust, or an auto-pick.
- Implementing the card would require changing action or tensor contracts without a dedicated plan.

## Output

During execution, send only short status updates unless there is a blocker. Do not present a first-batch summary as though the task is complete.

Final reporting must lead with:

- Cards implemented and tests run.
- Batch totals across the whole requested pool.
- Remaining cards blocked by capability, not by one-off card chores.
- Tracker files updated.
- Any suites not run and why.
