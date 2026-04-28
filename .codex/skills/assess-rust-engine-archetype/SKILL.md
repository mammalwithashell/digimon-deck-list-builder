---
name: assess-rust-engine-archetype
description: Assess whether the Digimon Rust engine and YAML DSL can implement a requested archetype, deck, card group, or card list; use when Codex needs to inspect printed card text, current Rust DSL examples/lowering support, action/pending-selection coverage, and engine tests to produce a readiness report with concrete implementation gaps.
---

# Assess Rust Engine Archetype

## Overview

Use this skill to answer: "Can the current Rust engine DSL implement this archetype faithfully?" Produce an evidence-backed readiness report, not a speculative implementation plan.

Prefer the Rust engine and DSL as the target. Treat legacy Python and DCGO only as references, and preserve the no-approximations policy: every gameplay choice must be represented by an engine action or `PendingSelection`.

## Workflow

1. Identify the assessment target.
   - If the user provided card IDs or a decklist, use those exact cards.
   - If the user provided only an archetype name, derive the likely card set from `data/deck_library.json`, local decklists, and `data/cards.json`.
   - Separate core cards from tech cards when the archetype is broad.

2. Read authoritative card text first.
   - Use `data/cards.json` fields such as `effect_text`, `inherited_text`, and `security_text`.
   - Consult `docs/RULES_CONTEXT.md` for keyword/timing semantics.
   - Use Fandom/wiki or DCGO only when printed text and local rules docs do not resolve a behavior question.

3. Inspect the DSL surface.
   - Read `code/digimon-dsl/README.md`, `code/digimon-dsl/src/spec.rs`, `code/digimon-dsl/src/step.rs`, and `code/digimon-dsl/src/predicate.rs` only as needed.
   - Compare against authored examples in `code/digimon-engine/cards/_examples/*.yaml`.
   - Check lowering coverage in `code/digimon-engine/src/dsl_cards/`, especially `lower_triggered.rs`, `lower_replacement.rs`, `lower_delay.rs`, `lower_aura.rs`, `lower_cost_reduction.rs`, and `step/`.

4. Inspect engine capability behind the DSL.
   - Check `docs/RUST_ENGINE_API.md` for `EffectContext` primitives.
   - Check `code/digimon-engine/src/selection.rs`, `src/action/`, and `src/effect_context/` for selection/action support.
   - Check existing behavioral tests under `code/digimon-engine/tests/`, especially `tests/dsl/`, `tests/cards_behavioral/`, `tests/combat/`, and `tests/cost_hooks/`.
   - Check gap trackers such as `docs/RUST_ENGINE_GAPS.md`, `qa/dsl-vocab-gaps.md`, `docs/RUST_PYTHON_PARITY.md`, and legacy `qa/archetype-qa/engine-gaps.md` when present.

5. Classify each card/effect.
   - `ready`: expressible in current YAML DSL and backed by tests or obvious lowering.
   - `dsl-gap`: engine can likely support the behavior, but YAML schema/lowering lacks a needed predicate, step, timing, binding, formula, or selection form. Route reusable DSL vocabulary or lowering gaps to `qa/dsl-vocab-gaps.md`.
   - `engine-gap`: the Rust engine lacks the underlying rule primitive, timing hook, action mask surface, state mutation, token/zone behavior, or pending-selection flow. Route reusable Rust engine capability gaps to `docs/RUST_ENGINE_GAPS.md`.
   - `rules-gap`: printed text cannot be assessed without a ruling or unresolved rules interpretation.
   - `test-gap`: behavior appears supported but lacks meaningful regression coverage for this archetype.
   - `data-gap`: local card metadata, traits, colors, levels, alt paths, or card IDs are missing or inconsistent.

6. Produce a report.
   - Start with a concise verdict: ready, mostly ready, blocked, or unknown.
   - List concrete gaps with file references, the smallest missing capability, and the tracker each reusable gap belongs in.
   - Distinguish DSL gaps from engine gaps; do not collapse them into "not implemented."
   - Include suggested first tests for blockers, because new Rust card behavior is TDD.
   - Avoid claiming an effect is implementable unless the DSL and engine can surface all choices without hidden auto-selection.

## Output Shape

Use `references/report-rubric.md` for the recommended table and gap format. Keep the final answer short enough to act on, but include enough evidence that a maintainer can decide what to build next.

When implementation is requested after assessment, continue with TDD: write a failing Rust behavioral test under `code/digimon-engine/tests/` before adding DSL or engine behavior.
