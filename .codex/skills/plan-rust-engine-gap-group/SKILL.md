---
name: plan-rust-engine-gap-group
description: Use when the user asks to plan a numbered Rust engine/DSL archetype gap roadmap group, such as "plan group 5", "plan gap group 7", or requests an implementation plan from the archetype engine and DSL gap spec.
---

# Plan Rust Engine Gap Group

## Overview

Create or update a focused implementation plan for one numbered group in the Digimon Rust engine and YAML DSL archetype gap roadmap. The output is a Superpowers implementation plan, not code changes.

**REQUIRED SUB-SKILL:** Use `superpowers:writing-plans` before writing the plan.

## Workflow

1. Announce: `I'm using the plan-rust-engine-gap-group skill and the writing-plans skill to create the implementation plan.`
2. Parse the requested group number. If missing, ask for the number before reading broadly.
3. Read the roadmap source:
   - `docs/superpowers/specs/2026-04-29-archetype-engine-dsl-gap-roadmap-design.md`
   - `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`
4. Read the shared contract docs before planning implementation details:
   - `AGENTS.md`, `CLAUDE.md`
   - `docs/RUST_ENGINE_API.md`
   - `docs/RUST_DSL_TEST_API.md`
   - `docs/ACTION_SPEC.md`
   - `docs/TENSOR_SPEC.md`
   - `docs/RUST_ENGINE_GAPS.md`
   - `qa/archetype-qa/engine-gaps.md`
   - `qa/dsl-vocab-gaps.md`
5. Check whether a plan already exists for the group in `docs/superpowers/plans/`.
   - If no plan exists, create one.
   - If a plan exists but is incomplete or stale, update it instead of duplicating it.
   - If the group has split child plans, preserve that split and plan the next unfinished slice.
6. Use `superpowers:writing-plans` exactly for the plan artifact: required header, bite-sized tasks, exact file paths, TDD test-first steps, commands with expected results, tracker updates, and self-review.
7. Save the plan to `docs/superpowers/plans/YYYY-MM-DD-gap-group-N-<slug>.md`, unless the parent plan already names a specific file.
8. Do not implement engine or DSL code unless the user explicitly asks for execution after the plan is written.

## Group Map

| Group | Slug | Focus |
|---|---|---|
| 1 | `event-context-dispatch` | Event payloads, inherited dispatch, breeding/option/security observers |
| 2 | `selection-primitives` | Pending selections, action masks, source refs, DP-budget picks |
| 3 | `cost-replacement` | Pay-cost continuation, replacement causes, Partition, Delay replacement |
| 4 | `zone-movement-stack-operations` | Effect movement between hand/trash/security/breeding and stack mutation |
| 5 | `option-delay-link-state` | Option flow, Delay, Plug-In, Link, Training, scheduled option effects |
| 6 | `modifiers-auras-keywords` | Modifiers, auras, combat keywords, dynamic DP/security formulas |
| 7 | `dsl-predicates-formulas` | Predicate/formula/lowering vocabulary without raw-Rust escapes |
| 8 | `token-card-data` | Tokens, `CardKind::Token`, DNA costs, aliases, Ace Overflow metadata |
| 9 | `archetype-unlocks` | Archetype readiness passes after capability groups |
| 10 | `acceptance-regression-gates` | Cross-group verification, contract review, tracker discipline |

## Plan Requirements

Every plan must include:

- A clear scope note naming dependencies and surfaces that must not run in parallel.
- A file structure section listing likely Rust, DSL, YAML, test, and tracker files.
- One task per narrow behavior slice, each with failing test, run-to-fail command, minimal implementation guidance, run-to-pass command, tracker update, and commit step.
- Action-mask and action-decoder tests for every new player-visible choice.
- PyO3/RL/frontend contract review if `ACTION_SPACE_SIZE`, tensor shape, masks, or exposed runner constants change.
- Tracker update instructions for `docs/RUST_ENGINE_GAPS.md`, `qa/archetype-qa/engine-gaps.md`, and `qa/dsl-vocab-gaps.md`.
- A final self-review for spec coverage, placeholder scan, type/name consistency, and no-approximations compliance.

Never close a gap on paper without a passing test command and a precise tracker edit. Never add hidden auto-selection, no-op card effects, UI-only rules, or raw-Rust escape hatches as the final planned state.

## Group 5 Defaults

When the user says `plan group 5`, use:

- Plan path: `docs/superpowers/plans/YYYY-MM-DD-gap-group-5-option-delay-link-state.md`, unless the parent plan specifies `docs/superpowers/plans/2026-04-29-gap-group-5-option-delay-link.md`.
- First fixtures: `LM-027` Red Scramble, `BT22-098` Unique Emblem: Fable Waltz, `BT17-097` Return to the Primogenitor, `ST22-08` Offensive Plug-In V, `BT13-110` Royal Knights of the Purge.
- Core files to inspect: `code/digimon-engine/src/enums.rs`, `game_actions.rs`, `game_phases.rs`, `effect_context.rs`, `scheduled_effects.rs`, `dsl_cards/lower_delay.rs`, `dsl_cards/timing_map.rs`.
- Test lanes to inspect or create under: `code/digimon-engine/tests/option_flow/` and `code/digimon-engine/tests/dsl/delay.rs`.

Separate Delay into start-of-turn Delay, event-gated Delay, and replacement-window Delay. Keep Plug-In/Link state distinct from digivolution sources and regular battle-area permanents.

## Existing Plan Handling

Use `Get-ChildItem docs/superpowers/plans` or equivalent file listing to find existing group plans. If a partial plan exists:

- Read it before writing.
- Preserve completed checked-off tasks.
- Add missing writing-plan detail instead of rewriting unrelated sections.
- If the existing plan is already complete, report that and offer to plan the next child slice or execute it.
