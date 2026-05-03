---
name: implement-rust-dsl-archetype
description: Use when Codex needs to implement a Digimon archetype, deck, card group, explicit card list, or DSL test pool as Rust engine YAML DSL cards in batches with TDD, per-card IMPLEMENT/AUDIT/SKIP modes, batch review, tracker updates, and no-approximations gap routing.
---

# Implement Rust DSL Archetype

## Overview

Implement an archetype by converting a resolved card pool into tested Rust engine YAML DSL cards. Process cards in small batches, like the Claude `batch-implement-cards-rust-dsl` skill: resolve and classify the pool, group related cards, run per-card TDD workers, review each batch, then merge, wire, test, and update trackers.

Preserve the no-approximations policy: every gameplay choice must go through engine actions or `PendingSelection`, and every behavior change must start with a failing Rust test.

**REQUIRED SUB-SKILLS:** Use `superpowers:test-driven-development`, `superpowers:subagent-driven-development`, and `superpowers:verification-before-completion`. Use `assess-rust-engine-archetype` for card-pool and gap-resolution evidence.

## Required Docs

Read these before dispatching implementation work:

- `AGENTS.md` and `CLAUDE.md`
- `docs/RUST_DSL_AGENT_GUIDE.md`
- `docs/RUST_DSL_TEST_API.md`
- `docs/RUST_ENGINE_API.md`
- `docs/ACTION_SPEC.md` and `docs/TENSOR_SPEC.md`
- `docs/RUST_ENGINE_GAPS.md`
- `qa/dsl-vocab-gaps.md`
- `qa/archetype-qa/engine-gaps.md`

Load reference files only when needed:

- `references/card-pool-resolution.md` for resolving archetype cards and implementation order.
- `references/subagent-tdd-workflow.md` for batch task slicing, subagent roles, and prompt templates.

## Batch Defaults

- Default batch size: `4`.
- Supported modes:
  - `IMPLEMENT`: no YAML exists at `code/digimon-engine/cards/<set>/<CARD-ID>.yaml`; write tests first, then YAML.
  - `AUDIT`: YAML exists; audit faithfulness and test coverage before changing behavior.
  - `SKIP`: prior tracker verdict is complete, or the user asked not to audit existing YAML.
- Batch related cards together:
  - Cards that name each other in printed text.
  - Tamers plus the Digimon they explicitly buff.
  - Options plus the named Digimon they target.
  - Remaining slots in stable card-ID order.
- Mixed batches are allowed. A batch may contain both `IMPLEMENT` and `AUDIT` cards.
- The main orchestrator owns shared files: `mod.rs`, `main.rs`, QA trackers, gap trackers, and batch summaries. Per-card workers only own the card YAML and its card test file.

## Workflow

1. Announce the skill and required sub-skills.
2. Resolve the target card pool.
   - Exact card IDs or a decklist win over archetype inference.
   - For an archetype name, follow `references/card-pool-resolution.md`.
   - Produce a queue grouped by dependency: supported DSL cards first, then cards that need reusable DSL gaps, then cards that need engine primitives.
3. Run a readiness pass before writing code.
   - Reuse `assess-rust-engine-archetype` logic.
   - Classify each card/effect as `ready`, `dsl-gap`, `engine-gap`, `rules-gap`, `test-gap`, or `data-gap`.
   - Do not implement blocked effects by no-op YAML, broad `raw_rust`, or hidden auto-selection.
4. Classify and batch the work.
   - Compute `IMPLEMENT`, `AUDIT`, or `SKIP` for each card from YAML existence and prior verdicts in `qa/qa-reports/validated_cards_dsl.json` when present.
   - Print a batch plan before implementation: total cards, counts by mode, cards per batch, and any blocked prerequisites.
   - If the user asked for report-only planning, stop after the batch plan.
5. Pre-wire batch test discovery before dispatch.
   - For each non-skipped planned card, ensure `code/digimon-engine/tests/cards_behavioral/<set>/mod.rs` contains `mod <card_id_lower>;`.
   - Ensure `code/digimon-engine/tests/cards_behavioral/main.rs` contains `mod <set>;`.
   - Ensure the per-card test file exists so workers can run `cargo test --test cards_behavioral -- <card_id_lower>`.
   - Remove placeholder registration later for cards that end up `BLOCKED` and wrote no tests.
6. Execute one batch at a time.
   - Follow `references/subagent-tdd-workflow.md`.
   - Gather per-card context from `data/cards.json`, DCGO if present, existing YAML/tests for `AUDIT`, and the relevant gap trackers.
   - For each `IMPLEMENT` card, run a read-only scout pass when the card is complex or likely to expose DSL/engine gaps.
   - Dispatch one fresh worker per non-skipped card in the batch. Workers must use disjoint paths: one YAML file and one test file.
   - Dispatch one batch reviewer after all workers return. Review spec compliance first, then code quality and maintainability.
   - Do not move to the next batch while review findings or targeted batch test failures remain open.
7. Merge, wire, and summarize each batch.
   - Reject worker output that modifies files outside its assigned YAML/test paths unless explicitly approved.
   - Apply review fixes once. If targeted batch tests still fail, run one bounded fix pass, then stop and report the blocker.
   - Update `qa/qa-reports/validated_cards_dsl.json` when present, per-archetype DSL QA artifacts, and reusable gap trackers.
   - Print a per-batch table with `Card ID`, `Mode`, `Verdict`, `Review`, `Tests`, and `Notes`.
8. Update trackers and archetype notes as behavior lands.
   - `docs/RUST_ENGINE_GAPS.md` for missing engine primitives.
   - `qa/dsl-vocab-gaps.md` for missing YAML schema/lowering vocabulary.
   - Per-archetype notes under `qa/archetype-qa/` or `qa/archetype-qa/dsl/` for readiness status.
9. Verify before completion.
   - Run targeted Rust tests for changed card/primitive surfaces.
   - Run broader DSL/action/selection suites when shared DSL, mask, selection, replacement, or engine behavior changed.
   - Run `cargo test --manifest-path code/digimon-engine/Cargo.toml` after a full archetype or multi-batch run unless the user scoped verification narrower.
   - Run `git diff --check`.

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

When reporting progress, lead with:

- Cards implemented and tests run.
- Batch totals and current batch number when running more than one batch.
- Remaining cards blocked by capability, not by one-off card chores.
- Tracker files updated.
- Any suites not run and why.
