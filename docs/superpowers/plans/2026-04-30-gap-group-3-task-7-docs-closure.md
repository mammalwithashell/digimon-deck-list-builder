# Group 3 Task 7 Documentation Closure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close Group 3 in the roadmap and gap docs after Partition, Delay-as-replacement, and Attack Cancellation are implemented and verified.

**Architecture:** This is a docs-only closure pass. It records the implemented Group 3 behavior, links the regression coverage, and marks the parent roadmap cost/replacement child-plan slice complete.

**Tech Stack:** Markdown documentation, Rust verification commands.

---

## Session Boundary

Suggested branch: `codex/group-3-task-7-docs-closure`.

Do not start this plan until these commits exist on the branch being closed:
- `feat: add partition source replacement flow`
- `feat: add delay prevention replacement flow`
- `feat: allow effects to cancel pending attacks`

This session owns:
- `docs/RUST_ENGINE_GAPS.md`
- `qa/archetype-qa/engine-gaps.md`
- `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`

---

### Task 1: Update Engine Gap Documentation

**Files:**
- Modify: `docs/RUST_ENGINE_GAPS.md`

- [ ] **Step 1: Replace the open Cost and Replacement gap entry**

In `docs/RUST_ENGINE_GAPS.md`, mark the Group 3 cost/replacement items as implemented with this text:

```markdown
### Cost and Replacement Framework

Status: implemented.

Regression coverage:
- `code/digimon-engine/tests/cost_hooks/stacked_would_play_reducers.rs`
- `code/digimon-engine/tests/cost_hooks/pay_cost_selection.rs`
- `code/digimon-engine/tests/replacements/context_predicates.rs`
- `code/digimon-engine/tests/replacements/partition.rs`
- `code/digimon-engine/tests/option_flow/replacement_integration.rs::bt17_097_delay_prevents_deletion_and_digivolves_from_hand`
- `code/digimon-engine/tests/replacements/attack_cancel.rs`

The engine supports stacked optional would-play cost reducers, triggered pay
costs that park pending selections before process execution, optional pay-cost
decline, replacement cause/controller predicates, Partition source selection,
Delay-as-replacement prevention, and effect-driven pending attack cancellation.
```

- [ ] **Step 2: Confirm no duplicate open Group 3 entry remains**

Run:

```bash
Select-String -Path docs/RUST_ENGINE_GAPS.md -Pattern 'Partition|Delay-as-replacement|attack cancellation|Cost and Replacement Framework' -Context 2,4
```

Expected: the remaining matches describe implemented coverage, not an open gap for Group 3.

---

### Task 2: Update Archetype QA Gaps

**Files:**
- Modify: `qa/archetype-qa/engine-gaps.md`

- [ ] **Step 1: Replace open Group 3 entries**

In `qa/archetype-qa/engine-gaps.md`, replace the open Group 3 cost/replacement entries with:

```markdown
### Cost and Replacement Framework

Resolved by Group 3:
- BT13-007 King Drasil_7D6 and ST21-13 Matt Ishida & T.K. Takaishi can both reduce AD1-025 Omnimon before memory is paid because AD1-025 has both `[Royal Knight]` and `[ADVENTURE]`.
- Triggered effect costs may install pending selections and resume process only after cost payment.
- Optional cost decline skips process without hidden auto-selection.
- Replacement predicates can inspect cause, source controller, and subject controller.
- Partition source requirements are enforced before prevention.
- Delay options can pay themselves as replacement costs and prevent deletion.
- Effects can end a pending attack after a printed cost resolves.
```

- [ ] **Step 2: Confirm no stale Group 3 blocker remains**

Run:

```bash
Select-String -Path qa/archetype-qa/engine-gaps.md -Pattern 'BT13-007|ST21-13|AD1-025|BT16-025|BT17-097|EX10-003|EX9-032|EX7-027|BT22-036' -Context 2,4
```

Expected: matches either appear under resolved Group 3 coverage or in unrelated future-group sections.

---

### Task 3: Update Parent Roadmap

**Files:**
- Modify: `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`

- [ ] **Step 1: Mark Group 3 child-plan work complete**

Under `Task 4: Create Child Plan for Cost and Replacement Framework`, mark the child-plan checklist complete:

```markdown
- [x] Create `docs/superpowers/plans/2026-04-29-gap-group-3-cost-replacement.md`.
- [x] Define slices for:
  - `.pay_cost()` for non-BeforePayCost triggered effects.
  - Stacked optional would-play cost reducers.
  - Optional cost decline path through pending selection.
  - Replacement context cause/controller predicate.
  - Partition source enforcement and selection.
  - Delay-as-replacement prevention.
  - Attack cancellation return path.
- [x] Require regression fixtures:
  - `EX10-003` Tumblemon for attack cancellation.
  - `AD1-025` Omnimon plus `BT13-007` King Drasil_7D6 plus `ST21-13` Matt Ishida & T.K. Takaishi for stacked play-cost reductions.
  - `BT16-025` Paildramon for Partition source enforcement.
  - `BT17-097` Return to the Primogenitor for Delay-as-replacement.
  - `EX9-032` / `EX7-027` / `BT22-036` for replacement cause gate.
```

- [ ] **Step 2: Add execution-plan references**

If the parent roadmap has a Group 3 notes area, add:

```markdown
Execution split:
- `docs/superpowers/plans/2026-04-30-gap-group-3-task-4-partition.md`
- `docs/superpowers/plans/2026-04-30-gap-group-3-task-5-delay-replacement.md`
- `docs/superpowers/plans/2026-04-30-gap-group-3-task-6-attack-cancel.md`
- `docs/superpowers/plans/2026-04-30-gap-group-3-task-7-docs-closure.md`
```

If the parent roadmap has no notes area, skip adding this block and leave the checklist as the only roadmap update.

---

### Task 4: Verify and Commit

**Files:**
- Stage all files from this plan.

- [ ] **Step 1: Run full Group 3 verification**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- pay_cost --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test cost_hooks -- stacked_would_play_reducers --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- context_predicates --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- attack_cancel --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test option_flow -- replacement_integration --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- replacement --nocapture
```

Expected: PASS.

- [ ] **Step 2: Check status**

Run:

```bash
git status --short
```

Expected: only the three documentation files are modified.

- [ ] **Step 3: Commit**

Run:

```bash
git add docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md
git commit -m "docs: close cost replacement gaps"
```

Expected: commit succeeds.
