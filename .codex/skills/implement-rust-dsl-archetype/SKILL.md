---
name: implement-rust-dsl-archetype
description: Use when Codex needs to implement a Digimon archetype, deck, card group, or card list as Rust engine YAML DSL cards rather than only assess readiness or plan gap groups.
---

# Implement Rust DSL Archetype

## Overview

Implement an archetype by converting a resolved card pool into tested Rust engine YAML DSL cards. Preserve the no-approximations policy: every gameplay choice must go through engine actions or `PendingSelection`, and every behavior change must start with a failing Rust test.

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
- `references/subagent-tdd-workflow.md` for task slicing, subagent roles, and prompt templates.

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
4. Create bite-sized TDD tasks.
   - One task should cover one card clause or one reusable primitive plus its first card fixture.
   - Each task must name exact files, first failing test, expected RED failure, implementation surface, GREEN command, and tracker update.
5. Execute with subagents.
   - Follow `references/subagent-tdd-workflow.md`.
   - Dispatch fresh implementer subagents for independent tasks.
   - Dispatch spec-compliance review before code-quality review after each task.
   - Do not move to the next task while review findings remain open.
6. Update trackers and archetype notes as behavior lands.
   - `docs/RUST_ENGINE_GAPS.md` for missing engine primitives.
   - `qa/dsl-vocab-gaps.md` for missing YAML schema/lowering vocabulary.
   - Per-archetype notes under `qa/archetype-qa/` or `qa/archetype-qa/dsl/` for readiness status.
7. Verify before completion.
   - Run targeted Rust tests for changed card/primitive surfaces.
   - Run broader DSL/action/selection suites when shared DSL, mask, selection, replacement, or engine behavior changed.
   - Run `git diff --check`.

## Task Requirements

Every task must include:

- Printed text evidence from `data/cards.json`.
- Existing YAML/DSL examples to mimic when possible.
- A failing test under `code/digimon-engine/tests/` before YAML or engine edits.
- Action-mask or pending-selection assertions for every player-visible choice.
- Negative tests for filters, missing targets, optional decline, and once-per-turn where relevant.
- Tracker edits that are precise about closed, partial, and still-open sub-shapes.

## Stop Conditions

Stop and report a blocker instead of implementing when:

- A new player-visible choice cannot fit current pending-selection/action-mask surfaces.
- A card needs an unimplemented timing, replacement window, zone move, formula, or event payload.
- The only apparent implementation is `process: []`, card-local raw Rust, or an auto-pick.
- Implementing the card would require changing action or tensor contracts without a dedicated plan.

## Output

When reporting progress, lead with:

- Cards implemented and tests run.
- Remaining cards blocked by capability, not by one-off card chores.
- Tracker files updated.
- Any suites not run and why.
