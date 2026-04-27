# `/batch-implement-cards-rust-dsl` — Design Spec

**Date:** 2026-04-26
**Status:** Design — pending implementation plan
**Companion docs:**
- [`docs/RUST_DSL_TEST_API.md`](../../RUST_DSL_TEST_API.md) — DSL card test API (canonical for test patterns).
- [`docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`](2026-04-21-card-scripting-dsl.md) — DSL syntax + compile pipeline.
- [`docs/RUST_ENGINE_API.md`](../../RUST_ENGINE_API.md) — `EffectContext` surface.
- [`.claude/skills/batch-implement-cards-rust/SKILL.md`](../../../.claude/skills/batch-implement-cards-rust/SKILL.md) — sibling skill; structural template.

---

## 1. Context

The Rust engine is migrating to a declarative YAML DSL for card scripting (phases 0–3 already landed; phases 2c–2f and 3 in flight). The pattern that works for hand-written `CardEffect` structs (`/batch-implement-cards-rust`) is a strong template for a DSL-flavored equivalent — but the differences are substantive enough that forking the skill is cleaner than parameterizing the existing one.

**This skill (`/batch-implement-cards-rust-dsl`)** is the archetype-scoped (or pool-scoped) TDD pipeline that produces YAML card specs at `code/digimon-engine/cards/<set>/<CARD_ID>.yaml` and DebugRunner behavioral tests at `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs`.

**Why a separate skill (not a flag on the existing one):**
- Output artifacts differ — declarative YAML vs Rust closures.
- Test layout differs — `tests/cards_behavioral/` (DSL convention) vs `tests/behavioral/` (legacy Rust convention).
- No `cards.rs` registry wiring needed — `build.rs` discovers YAML automatically.
- Verdict vocabulary differs — adds `AUDITED-OK` / `AUDITED-MISSING-TESTS` / `AUDITED-DRIFT`, splits `BLOCKED` into engine-gap vs DSL-vocab-gap.
- Different model mix — Sonnet implementer + Sonnet scout + Opus reviewer, rather than Opus everywhere.

## 2. Scope and non-goals

### In scope (v1)
- Implement-from-scratch path for cards with no shipping YAML.
- Audit-only path for cards that already ship YAML (read + diff + add missing tests; emit drift diff but **do not modify** existing YAML).
- Two pool sources: `qa/dsl-test-pool.md` (curated) and any archetype in `data/deck_library.json`.
- Verdict tracking in `qa/qa-reports/validated_cards_dsl.json`.
- Engine-gap and DSL-vocab-gap routing to separate trackers.

### Out of scope (v1, deferred to v1.1+)
- `--fix` mode — automatic rewrite of drifted YAML. v1 emits a diff proposal only; human applies.
- Notion sync.
- Pinecone retrieval (explicit user decision: no Pinecone in v1 of this skill).
- Auto-escalation Sonnet→Opus on review-failure loops.
- Replacing hand-written `CardEffect` structs with DSL — cross-pipeline migration is a separate workflow.

## 3. Skill surface

```
/batch-implement-cards-rust-dsl <ARCHETYPE_OR_POOL> [flags]
```

### Positional argument (one of)
- An archetype name from `data/deck_library.json` (e.g. `"Royal Knights"`).
- The literal token `--pool` to target every card in `qa/dsl-test-pool.md`.

### Flags

| Flag | Default | Purpose |
|---|---|---|
| `--cards CARD1,CARD2,...` | unset | Override resolution; explicit comma-separated list. Wins over both archetype and `--pool`. |
| `--batch-size N` | `4` | Per-batch worker count. |
| `--report-only` | off | Phases 1–2 only — resolve, classify, plan, exit. No agents dispatched. |
| `--implementer-model {sonnet,opus}` | `sonnet` | Escape hatch for hard archetypes. Scout stays Sonnet, reviewer stays Opus regardless. |
| `--no-audit` | off | Skip AUDIT-mode dispatch for cards already shipping YAML. They become `SKIP` instead. |
| `--skip-tests` | off | Emit YAML without behavioral tests. Strongly discouraged — breaks TDD discipline. |

### Auto-detected per-card mode
- **IMPLEMENT** — no YAML at `cards/<set>/<CARD_ID>.yaml`. Full scout → implementer → reviewer pipeline.
- **AUDIT** — YAML exists. Single Sonnet auditor → reviewer. No scout.
- **SKIP** — prior `IMPLEMENTED` or `AUDITED-OK` verdict in `validated_cards_dsl.json`, or `--no-audit` set and YAML exists.

## 4. Phase architecture

### Phase 1 — Resolve card pool

1. Parse the positional argument:
   - `--pool` → parse the table in `qa/dsl-test-pool.md`, take column 1 as card IDs.
   - Archetype name → `code/tools/resolve_deck.py::resolve_archetype`.
   - `--cards` always wins over both.
2. Classify each card by YAML existence at `code/digimon-engine/cards/<set_lower>/<CARD_ID>.yaml`:
   - YAML missing → `IMPLEMENT`
   - YAML present → `AUDIT` (or `SKIP` if `--no-audit`)
3. Cross-check `qa/qa-reports/validated_cards_dsl.json` (create with `{"version":1,"cards":{}}` if absent). Cards with prior `IMPLEMENTED` or `AUDITED-OK` verdicts → `SKIP`.
4. Build cross-archetype reverse map (`card_id → [archetype_name, ...]`) by scanning `deck_library.json` — surfaces "this card is also used by N other archetypes" in the final report.

### Phase 2 — Batch and plan

- Group into batches of `--batch-size` (default 4) using the same heuristics as `/batch-implement-cards-rust`:
  1. Cards that reference each other by name → same batch.
  2. Tamer + its buffed Digimon → same batch.
  3. Option card + its target Digimon → same batch.
  4. Remaining slots filled in card-ID order.
- Mixed-mode batches are allowed (e.g. 2 IMPLEMENT + 2 AUDIT in the same batch of 4).
- Print plan table; require explicit user approval before Phase 4.

### Phase 3 — Pre-read shared context (orchestrator)

Read once and hold in memory for embedding into every prompt:

1. `docs/RUST_DSL_TEST_API.md` (full).
2. The skill's positive-rules appendix (lives in `SKILL.md`; see §8 below).
3. `qa/archetype-qa/engine-gaps.md` (current engine gaps).
4. `qa/dsl-vocab-gaps.md` (current DSL vocab gaps; create empty if absent with header `# DSL Vocabulary Gaps Tracker`).

The DSL spec (`docs/superpowers/specs/2026-04-21-card-scripting-dsl.md`) and `docs/RUST_ENGINE_API.md` are **cited as paths**, not embedded — workers `Read` them on demand.

Pre-create directories: `code/digimon-engine/cards/<set>/`, `code/digimon-engine/tests/cards_behavioral/<set>/`, `qa/archetype-qa/dsl/`.

`<set>` is the lowercased prefix from the card ID:
- `BT17-001` → `bt17`
- `EX11-027` → `ex11`
- `P-117` → `p`
- `LM-029` → `lm`
- `AD1-025` → `ad1`

### Phase 4 — Batch loop

For each batch:

#### 4A. Per-card context gather (orchestrator)

For each card in the batch:
1. Pull metadata from `data/cards.json` — printed text (effect/inherited/security), `card_kind`, `level`, `dp`, `play_cost`, `card_colors`, `type_eng` (traits), `evo_costs`, `dna_costs`.
2. Find DCGO C# at `DCGO/Assets/Scripts/CardEffect/<SET>/<COLOR>/<CARD_ID_UNDERSCORE>.cs` (e.g. `BT15-003` → `BT15_003.cs`). Read body if found; note absent if not.
3. Look up prior verdict in `validated_cards_dsl.json`.
4. For AUDIT cards: also read existing YAML at `cards/<set>/<CARD_ID>.yaml` and existing test at `tests/cards_behavioral/<set>/<card_id_lower>.rs` if present.

#### 4B. Scout wave (Sonnet, parallel — IMPLEMENT cards only)

One Agent call per IMPLEMENT card, all in a single assistant message for true parallelism. AUDIT cards skip this wave.

Scout returns a structured brief (~2–5K tokens). Schema in §5.1.

If the scout flags pre-flight `gap_kind` with high confidence, the orchestrator may short-circuit and emit `BLOCKED` for that card without dispatching the implementer. The reviewer still confirms the gap.

#### 4C. Implementer / Auditor wave (parallel, isolated worktrees)

One Agent call per non-skipped card, all in one message. Each runs with `isolation: "worktree"`.

- IMPLEMENT cards → Sonnet implementer (or Opus if `--implementer-model opus`). Inputs: scout brief + static pack. Schema in §5.2.
- AUDIT cards → Sonnet auditor. Inputs: existing YAML + existing tests + printed text + DCGO C#. Schema in §5.3.

Each worker writes/edits up to two files:
- `code/digimon-engine/cards/<set>/<CARD_ID>.yaml`
- `code/digimon-engine/tests/cards_behavioral/<set>/<card_id_lower>.rs`

Workers **never** touch `main.rs`, any `mod.rs`, `cards.rs`, tracker JSON, or any other shared file.

#### 4D. Review wave (Opus, single agent, no isolation, read-only)

Reviewer reads all files written by workers (orchestrator copies them out of worktrees first) plus all worker verdict blocks. Adjudicates scout-vs-implementer disagreement on gap classification. Applies the hybrid checklist (test API §11 + skill positive rules from §8). Emits `APPROVED` or `NEEDS-FIX` per card. Schema in §5.4.

#### 4E. Merge and wire (orchestrator)

In order:

1. Copy YAML and test files out of worker worktrees into the main tree.
2. Apply review fixes verbatim.
3. Update `tests/cards_behavioral/<set>/mod.rs` — append `mod <card_id_lower>;` for each new card.
4. Update `tests/cards_behavioral/main.rs` — append `mod <set>;` if first card from this set.
5. Run targeted batch tests:
   ```bash
   cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral -- <set>
   ```
   On failure: one targeted fix round (re-dispatch a single Sonnet "fix" agent with failing output + reviewer's directives). If still failing, escalate to user with raw output.
6. Update `qa/qa-reports/validated_cards_dsl.json` with one entry per processed card (schema §6).
7. Append batch summary to `qa/archetype-qa/dsl/<archetype_slug>.md` (or `qa/dsl-test-pool-progress.md` for `--pool` runs).
8. Append new gap entries to `engine-gaps.md` and/or `dsl-vocab-gaps.md` per `gap_kind` (routing in §7).
9. Print batch summary table to user; auto-continue to next batch.

### Phase 5 — Final report

After all batches:

1. Whole-archetype summary: counts by verdict.
2. Per-card results table: verdict, reviewer status, test count, one-line notes.
3. Files created/modified, grouped by set.
4. Blocked cards split into engine-gap section and DSL-vocab-gap section.
5. Full-suite green check:
   ```bash
   cargo test --manifest-path code/digimon-engine/Cargo.toml
   ```
   Must pass; if not, the skill left the tree broken — escalate without auto-fixing.
6. Finalize `qa/archetype-qa/dsl/<archetype_slug>.md` from template (§9).

### Phase 6 — Idempotency

Re-running on the same archetype (or `--pool`) is safe: the `validated_cards_dsl.json` lookup in Phase 1 short-circuits completed cards. Archetypes can be partially completed across sessions; the JSON is the source of truth for "what's done."

## 5. Worker prompt schemas

Full prompt text lives in `SKILL.md`. This section pins the contract — what each agent receives and returns.

### 5.1 Scout (Sonnet, IMPLEMENT only)

**Inputs:**
- Card metadata + printed text
- DCGO C# body (or "absent")
- Paths (not bodies) to: `RUST_DSL_TEST_API.md`, DSL spec, `RUST_ENGINE_API.md`
- Path to `cards/` directory listing

**Returns:**

```
## Brief: <CARD_ID>

### Pattern rows (test API §4.3)
- <row tags, e.g. C3, E1, E2, D1>

### Required DSL verbs / step-kinds
- <verb_name> → DSL spec §X.Y [+optional usage note]
- ...

### Closest exemplar YAMLs
1. <path> — <one-line "why this is the closest match">
2. <path> — <one-line>

### Target engine APIs (from RUST_ENGINE_API.md)
- EffectContext::<method_name>
- ...

### Behavioral test scope (test API §5)
- Structural: <clause counts by scope/kind>
- Per-branch: <enumerate>
- Negative tests: <enumerate>

### Pre-flight gap suspicion
NONE | ENGINE-GAP: <description> | DSL-GAP: <description> | HYBRID: <description>
```

### 5.2 Implementer (Sonnet by default)

**Static context (~18–22K embedded):**
- Full `RUST_DSL_TEST_API.md`
- Skill positive-rules appendix (§8)
- Hybrid checklist (test API §11 + appendix)
- Current engine gaps + DSL vocab gaps
- Read-on-demand directives for DSL spec and `RUST_ENGINE_API.md` (paths only)

**Per-card context:**
- Card metadata + printed text
- DCGO C# body
- Scout brief (§5.1 output)
- Prior verdict if any

**Workflow (TDD-strict):**
1. Decompose card text into numbered clauses.
2. Write `tests/cards_behavioral/<set>/<card_id_lower>.rs` FIRST. Mandatory file header per test API §5: card text verbatim, DCGO ref path, pattern row tags. Include structural assertions, per-clause behavioral tests with positive + negative branches, OPT enforcement test, event-log test for cost firing where applicable.
3. Run tests, confirm expected failures.
4. Author `cards/<set>/<CARD_ID>.yaml`. Use scout's exemplar(s) as starting structure.
5. Re-run tests. Iterate until green.
6. Faithfulness self-audit against the hybrid checklist.
7. Emit verdict.

**Returns:**

```
## <CARD_ID> — <CARD_NAME>

### Verdict: IMPLEMENTED | PARTIAL | BLOCKED
### Gap kind (if BLOCKED): engine | dsl | hybrid
### Scout-disagreement (if any): <description>

### Clause analysis
Clause 1 (<timing>): "<exact text>"
  Expected: <behavior>
  YAML location: lines X–Y
  Tests: <test names>
  Status: MATCH | PARTIAL | BLOCKED
...

### Files written
- cards/<set>/<CARD_ID>.yaml (N clauses, M lines)
- tests/cards_behavioral/<set>/<card_id_lower>.rs (N tests)

### Test output (final cargo test summary, trimmed)

### Engine gaps discovered (if any)
## <CARD_ID> — <clause>
Missing API: <description>
Suggested addition: <signature>

### DSL vocab gaps discovered (if any)
## <CARD_ID> — <clause>
Missing verb / step kind / predicate: <description>
Lowers to engine API: <which one — proves it's DSL-only>
Suggested DSL syntax: <YAML shape>

### New patterns worth documenting in RUST_DSL_TEST_API.md (if any)
- <pattern>: <description>
```

### 5.3 Auditor (Sonnet, AUDIT only)

**Static context:** same as implementer.

**Per-card context:**
- Card metadata + printed text
- DCGO C# body
- Existing YAML body
- Existing test file body if present

**Workflow:**
1. Diff printed text against YAML clause-by-clause. Catch silent drops, missing branches, optionality mismatches, condition gaps, OPT misses.
2. Diff DCGO C# against YAML for behavioral fidelity.
3. Inventory existing tests against test API §5 expected coverage.
4. Emit one of:
   - `AUDITED-OK` — YAML faithful, tests cover §5 expectations.
   - `AUDITED-MISSING-TESTS` — YAML faithful, but tests are incomplete. Auditor writes the missing tests.
   - `AUDITED-DRIFT` — YAML disagrees with printed text or DCGO. Auditor emits a unified-diff proposal but does **not** modify the YAML in v1.
5. Faithfulness self-audit; emit verdict.

**Returns:**

```
## <CARD_ID> — <CARD_NAME> — AUDIT

### Verdict: AUDITED-OK | AUDITED-MISSING-TESTS | AUDITED-DRIFT | BLOCKED

### Faithfulness diff (if DRIFT)
Clause 1 (<timing>): "<printed text>"
  YAML says: <what's there>
  Should say: <correction>
  Source: printed text | DCGO C# line N

### Tests added (if MISSING-TESTS)
- <test name 1>
- <test name 2>

### Files written/modified
- tests/cards_behavioral/<set>/<card_id_lower>.rs (added N tests, total now M)
[YAML unchanged]
```

### 5.4 Reviewer (Opus, one per batch)

**Inputs:**
- All worker verdict blocks for the batch
- All YAML and test files written by workers (orchestrator copies them out of worktrees)
- Per-card metadata + DCGO C#
- Static context: full test API doc + hybrid checklist + skill positive rules (§8)

**Adjudication points:**
- For IMPLEMENT cards: hybrid checklist applied; scout-vs-implementer gap disagreement adjudicated; test enumeration completeness checked against test API §5.
- For AUDIT cards: confirm `AUDITED-OK` is real (no hidden drift); confirm `AUDITED-DRIFT` diff is correct.

**Returns:**

```
<CARD_ID>: APPROVED
or
<CARD_ID>: NEEDS-FIX
  - Issue 1: <description> — Fix: <file:line directive>
  - Issue 2: ...
```

## 6. `validated_cards_dsl.json` schema

```json
{
  "version": 1,
  "last_updated": "YYYY-MM-DD",
  "cards": {
    "BT15-003": {
      "card_name": "Nyaromon",
      "validated_date": "YYYY-MM-DD",
      "report": "batch-implement-cards-rust-dsl",
      "status": "IMPLEMENTED",
      "gap_kind": null,
      "archetype": "Slice — Nokia/Greymon/Omnimon",
      "yaml_path": "code/digimon-engine/cards/bt15/BT15-003.yaml",
      "test_path": "code/digimon-engine/tests/cards_behavioral/bt15/bt15_003.rs",
      "test_count": 7,
      "patterns": ["G4", "E2", "F5"],
      "notes": "Inherited When Attacking + OPT + top/bottom branch"
    }
  }
}
```

| Field | Domain |
|---|---|
| `status` | `IMPLEMENTED` / `PARTIAL` / `AUDITED-OK` / `AUDITED-MISSING-TESTS` / `AUDITED-DRIFT` / `BLOCKED` |
| `gap_kind` | `null` / `"engine"` / `"dsl"` / `"hybrid"` (only set when `status == BLOCKED`) |
| `patterns` | Array of test-API §4.3 row tags from the scout brief |

The Python-side `qa/qa-reports/validated_cards.json` is **never modified**.

## 7. Gap routing

When a worker reports `BLOCKED` with a `gap_kind`:

| `gap_kind` | Destination | Format |
|---|---|---|
| `engine` | `qa/archetype-qa/engine-gaps.md` | `## <CARD_ID> — <clause>` block + missing API description + suggested signature |
| `dsl` | `qa/dsl-vocab-gaps.md` | `## <CARD_ID> — <clause>` block + missing verb/predicate description + the engine API it would lower to (proves it's DSL-only) + suggested DSL syntax |
| `hybrid` | both trackers, cross-referenced | each entry references the other |

`qa/dsl-vocab-gaps.md` is created by the orchestrator on first run if absent, with header `# DSL Vocabulary Gaps Tracker`.

## 8. Skill positive-rules appendix

Embedded into both implementer and reviewer prompts. Forms the "C" half of the hybrid checklist (test API §11 + this appendix).

1. **TDD ordering is strict.** Tests are written before YAML. Test file must exist and fail before any YAML is authored. Implementer's verdict block must show the failing-test output before the passing-test output.
2. **File header docstring is mandatory** (test API §5). Format: card text verbatim from `cards.json`, DCGO C# reference path, pattern row tags from §4.3.
3. **One positive AND one negative test per condition.** Splitting is non-negotiable per test API §11.3. A single test asserting both directions is rejected.
4. **Every clause gets ≥1 integrated test** driven through `play` / `attack` / `end_turn`. Clause-isolated `EffectContext` tests (per §7) are *additional*, not substitutes.
5. **OPT clauses get an explicit lockout test** (test API §5 Section 5). Test that the second activation in the same turn is gated, and that the lockout clears after `end_turn`.
6. **Cost-firing clauses get an event-log test** (test API §5 Section 4). When an effect's cost has side effects (trash security, lose security, deletion), assert the corresponding `GameEvent` fires via `events_since(checkpoint)`.
7. **Use `dsl_card(id)`, never inline-paste production YAML** (test API §11.1). Inline fixtures are reserved for the cases enumerated in §10.
8. **Use `digimon_engine::action::space::*` constants, never hard-code action IDs** (test API §11.12).
9. **No approximations.** Every player choice surfaces through `pending_selection`. No `.iter().next()`, no `[0]`, no `min`/`max` over targets, no auto-resolutions of multi-option choices.
10. **No Python references.** Do not cite `engine_py_legacy/`, do not import Python script structure as ground truth. Ground truth is printed text + RULES_CONTEXT.md / fandom + DCGO C#.
11. **Engine-gap vs DSL-vocab-gap discipline.** Before declaring `BLOCKED`, confirm: does the engine *really* lack the primitive (grep `EffectContext`), or does only the DSL lack a verb that would lower to it? Set `gap_kind` accordingly.
12. **No `place_on_field` shortcuts when testing OnPlay paths** (test API §11.11). `place_on_field` is for post-play state only.
13. **No `auto_resolve` through a multi-branch prompt when testing a specific branch** (test API §11.4). Use `execute_branch` / `execute_action`, then `auto_resolve` only after the branching choice is locked.

## 9. Per-archetype QA artifact template

`qa/archetype-qa/dsl/<archetype_slug>.md`:

```markdown
# Archetype DSL Implementation: {Archetype Name}
Date: {YYYY-MM-DD}
Total cards in pool: {N}
Processed this run: {M}
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: {n}
- PARTIAL: {n}
- AUDITED-OK: {n}
- AUDITED-MISSING-TESTS: {n}
- AUDITED-DRIFT: {n}
- BLOCKED (engine): {n}
- BLOCKED (dsl): {n}
- BLOCKED (hybrid): {n}
- SKIPPED (prior verdict): {n}

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|

## Engine-Gap Blocked Cards
### {CARD_ID} {card_name}
- Effect text: "..."
- Missing engine API: ...
- Suggested addition: ...

## DSL-Vocab-Gap Blocked Cards
### {CARD_ID} {card_name}
- Effect text: "..."
- Missing DSL verb: ...
- Lowers to engine API: ...
- Suggested DSL syntax: ...

## New Patterns Discovered
- {pattern}: {description} — propose adding to RUST_DSL_TEST_API.md
```

For `--pool` runs: artifact is `qa/dsl-test-pool-progress.md` — single file accumulating across runs, one row per pool card, last verdict wins.

## 10. Orchestrator invariants

- Workers never edit `main.rs`, any `mod.rs`, `cards.rs`, `validated_cards_dsl.json`, or any tracker file. Orchestrator owns all shared state.
- Card-ID conventions: YAML files use original-case dashed IDs (`BT15-003.yaml`); Rust test files use lowercase-underscore (`bt15_003.rs`). Pack registry key is the original-case ID (`"BT15-003"`).
- The Python `qa/qa-reports/validated_cards.json` is never touched.
- No Notion calls.
- No Pinecone calls.
- Worktree dirty after a worker (files outside the two expected paths) → reject output and re-dispatch.

## 11. Verification (acceptance tests for the skill)

The implementation plan must demonstrate all five:

1. **Pool run end-to-end.** `--pool` against the test pool in `qa/dsl-test-pool.md`: every IMPLEMENT-mode card lands YAML + tests; every AUDIT-mode card produces an AUDITED verdict; full `cargo test --manifest-path code/digimon-engine/Cargo.toml` stays green at the end.
2. **Idempotency.** Re-run `--pool` immediately after — every card resolves to SKIP, no agents dispatched, no file writes.
3. **Mixed-mode batch routing.** A synthetic 4-card batch with 2 IMPLEMENT + 2 AUDIT: scout fires for 2, auditor fires for 2, reviewer adjudicates all 4, JSON tracker reflects mode-specific verdicts.
4. **Gap routing.** A deliberately-unimplementable card chosen from existing `engine-gaps.md` produces `BLOCKED` with correct `gap_kind`, entry lands in the right tracker (engine vs DSL).
5. **Archetype mode.** One small archetype (~8 cards) runs end-to-end; `qa/archetype-qa/dsl/<slug>.md` correctly populated; `validated_cards_dsl.json` updated.

## 12. Edge cases

- **Card-ID variant cases:** `P-117`, `LM-029`, `EX11-027`, `AD1-025`. Set-prefix extraction must handle single-letter sets (`P` → `cards/p/`, `LM` → `cards/lm/`).
- **DCGO C# missing:** some cards (especially `P-` promos) lack DCGO files. Worker proceeds with printed text + `RULES_CONTEXT.md`; verdict notes the missing reference.
- **Printed-text vs DCGO disagreement:** source-priority rule from `CLAUDE.md` applies — printed text wins; DCGO is implementation-detail tiebreaker only. Worker prompt restates this.
- **Scout brief empty / malformed:** orchestrator validates the structured fields before passing to implementer. If brief is unparseable, dispatch a single retry; if still bad, fall back to giving the implementer the full DSL spec embed for that one card and proceed.
- **Worker emits files outside expected paths:** orchestrator rejects output and re-dispatches once; if the second attempt also drifts, escalate to user.
- **`tools/resolve_deck.py` raises `UnknownArchetype`:** orchestrator surfaces the suggested fallback (`python code/tools/resolve_deck.py --list-archetypes --min-meta-share 0.01`) and exits.

## 13. Known limitations (v1)

- No `--fix` mode. `AUDITED-DRIFT` emits a diff proposal but does not modify YAML. Promoted to v1.1 when real drift is observed.
- No Notion sync.
- No Pinecone retrieval.
- Single fix round per batch (one re-dispatch on cargo failure, then escalate).
- Workers cannot add a new set's `mod.rs` or new test binary.
- `--report-only` is plan-only — does not run scouts to pre-classify gaps.
- Sonnet implementer is default; Opus is the escape hatch via `--implementer-model opus`. No automatic escalation on review-failure loops in v1.

## 14. Cross-references

- `docs/RUST_DSL_TEST_API.md` — DSL test-author reference; this skill's primary embedded context.
- `docs/superpowers/specs/2026-04-21-card-scripting-dsl.md` — DSL syntax + compile pipeline; read-on-demand by workers.
- `docs/RUST_ENGINE_API.md` — `EffectContext` surface; read-on-demand by workers.
- `qa/dsl-test-pool.md` — `--pool` mode resolution source.
- `qa/archetype-qa/engine-gaps.md` — engine-gap-flavored BLOCKED tracker.
- `qa/dsl-vocab-gaps.md` — DSL-vocab-flavored BLOCKED tracker (created on first run).
- `.claude/skills/batch-implement-cards-rust/SKILL.md` — sibling skill; structural template.
- `.claude/skills/batch-fix-cards/SKILL.md` — Python sibling; verdict-tracking conventions for reference only.
