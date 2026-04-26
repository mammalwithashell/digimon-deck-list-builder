---
name: batch-implement-cards-rust-dsl
description: Archetype- or pool-scoped TDD pipeline that authors YAML card specs in `code/digimon-engine/cards/<set>/` plus DebugRunner tests in `code/digimon-engine/tests/cards_behavioral/<set>/`. Three-wave architecture (Sonnet scout → Sonnet implementer/auditor → Opus reviewer) with per-card mode auto-detection (IMPLEMENT for new YAML / AUDIT for existing YAML / SKIP for prior verdicts). Engine-gap and DSL-vocab-gap routed to separate trackers. Verdicts in `validated_cards_dsl.json`.
argument-hint: <ARCHETYPE_NAME|--pool> [--cards CARD1,CARD2,...] [--batch-size N] [--report-only] [--implementer-model {sonnet,opus}] [--no-audit] [--skip-tests]
---

# Batch Implement Cards (Rust DSL) — Archetype/Pool-Scoped Test-First DSL Pipeline

Author YAML card specs and behavioral tests for an entire archetype (or the curated DSL test pool) using the engine's declarative DSL. Cards are processed in batches of 4 with parallel sub-agents in isolated git worktrees: a Sonnet scout pre-curates context, a Sonnet implementer (or Sonnet auditor for existing YAML) writes tests-first then YAML, and an Opus reviewer audits each batch. **The orchestrator wires test-discovery `mod.rs` files** after merging — agents never touch shared registration.

This is the DSL-flavored sibling of `/batch-implement-cards-rust`. Cards already implemented as hand-written Rust `CardEffect` structs are not affected; cards already shipping YAML are routed to AUDIT mode.

## When to Use

- Authoring DSL YAML for a new archetype on the Rust engine.
- Running the curated `qa/dsl-test-pool.md` end-to-end (pattern-coverage smoke).
- Auditing existing YAML for drift after a DSL phase lands new vocabulary.

**Not for:** hand-written Rust `CardEffect` work (use `/batch-implement-cards-rust`). Not for Python scripts (use `/batch-fix-cards`). Not for gameplay testing (use `/gameplay-qa`). Not for full rewrites of drifted YAML (v1 is audit-only — emits diff proposals, does not modify shipping YAML).

## Flags

| Flag | Default | Purpose |
|---|---|---|
| `--cards CARD1,CARD2,...` | unset | Explicit comma-separated card list. Wins over both archetype and `--pool`. |
| `--batch-size N` | `4` | Per-batch worker count. |
| `--report-only` | off | Phases 1–2 only — resolve, classify, plan, exit. No agents dispatched. |
| `--implementer-model {sonnet,opus}` | `sonnet` | Escape hatch for hard archetypes. Scout stays Sonnet, reviewer stays Opus regardless. |
| `--no-audit` | off | Skip AUDIT-mode dispatch for cards already shipping YAML. They become `SKIP`. |
| `--skip-tests` | off | Emit YAML without behavioral tests. Strongly discouraged — breaks TDD discipline. |

The positional argument is either an archetype name from `data/deck_library.json` or the literal token `--pool` (which targets every card in `qa/dsl-test-pool.md`).

## Per-Card Mode (auto-detected)

- **IMPLEMENT** — no YAML at `code/digimon-engine/cards/<set>/<CARD_ID>.yaml`. Full scout → implementer → reviewer pipeline.
- **AUDIT** — YAML exists. Single Sonnet auditor → reviewer. No scout wave.
- **SKIP** — prior `IMPLEMENTED` or `AUDITED-OK` verdict in `validated_cards_dsl.json`, or `--no-audit` is set and YAML exists.

## Quick Reference

| Resource | Path |
|----------|------|
| DSL test API (canonical for test patterns) | `docs/RUST_DSL_TEST_API.md` |
| DSL syntax + compile pipeline | `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` |
| Engine API (`EffectContext`) | `docs/RUST_ENGINE_API.md` |
| Production YAML directory | `code/digimon-engine/cards/<set>/<CARD_ID>.yaml` |
| Card test directory | `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs` |
| Test discovery root | `code/digimon-engine/tests/cards_behavioral/main.rs` |
| Test discovery per set | `code/digimon-engine/tests/cards_behavioral/<set>/mod.rs` |
| Card metadata | `data/cards.json` |
| Deck library | `data/deck_library.json` |
| DSL test pool (curated) | `qa/dsl-test-pool.md` |
| C# reference | `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CARD_ID_UNDERSCORE}.cs` |
| Verdict tracker | `qa/qa-reports/validated_cards_dsl.json` |
| Engine-gap tracker | `qa/archetype-qa/engine-gaps.md` |
| DSL-vocab-gap tracker | `qa/dsl-vocab-gaps.md` |
| Per-archetype QA artifact | `qa/archetype-qa/dsl/<archetype_slug>.md` |
| Pool-progress artifact | `qa/dsl-test-pool-progress.md` |

## Design spec

`docs/superpowers/specs/2026-04-26-batch-implement-cards-rust-dsl-design.md`

---
