---
name: batch-implement-cards-rust
description: Archetype-scoped TDD pipeline that hand-writes Rust `CardEffect` implementations in `code/digimon-engine/src/cards/`. Processes cards in batches of 4 with parallel Opus sub-agents. Each agent writes DebugRunner behavioral tests from card text FIRST, then implements the `CardEffect` struct to pass them. A review agent audits each batch. Orchestrator handles module registration. Tracks verdicts in `validated_cards_rust.json`.
argument-hint: <ARCHETYPE_NAME> [--cards CARD1,CARD2,...] [--batch-size N] [--report-only] [--skip-tests]
---

# Batch Implement Cards (Rust) — Archetype-Scoped Test-First Implementation Pipeline

Hand-write Rust `CardEffect` implementations for an entire archetype. Cards are processed in batches of 4 with parallel Opus sub-agents in isolated git worktrees. Each agent writes behavioral tests from card text first, then implements the effect to pass them. A separate review agent audits each batch. **The orchestrator wires registration** (`mod.rs`, `cards.rs`) after merging — agents never touch shared registry files.

Cards in the archetype that are already registered in Rust are skipped. This skill is the Rust analog of `/batch-fix-cards`, adapted for a compile-time, trait-based engine.

## When to Use

- Implementing a new archetype in the Rust engine (`code/digimon-engine/src/cards/`)
- Migrating an already-Python-implemented archetype one-way to Rust (see CLAUDE.md §21)
- Running a test-driven implementation pass across a deck list

**Not for:** Fixing existing Rust scripts (no FIX mode in v1 — there are very few real Rust scripts yet). Not for Python scripts (use `/batch-fix-cards`). Not for gameplay testing (use `/gameplay-qa`).

## Flags

- `--batch-size N`: Override batch size (default: 4)
- `--report-only`: Resolve pool + print plan, no tests/edits
- `--skip-tests`: Emit `CardEffect` structs without DebugRunner tests (strongly discouraged — breaks TDD discipline)
- `--cards CARD1,CARD2,...`: Override card pool with explicit comma-separated list

## Quick Reference

| Resource | Path |
|----------|------|
| Rust engine scripting API | `docs/RUST_ENGINE_API.md` |
| Rust/Python parity tracker | `docs/RUST_PYTHON_PARITY.md` |
| Engine design spec | `docs/superpowers/specs/2026-04-15-rust-engine-rewrite-design.md` |
| Test-card worked examples | `code/digimon-engine/src/cards/test_cards.rs` |
| Behavioral test patterns | `code/digimon-engine/tests/test_cards_behavioral.rs`, `code/digimon-engine/tests/security_effects.rs` |
| DebugRunner builder | `code/digimon-engine/src/debug_runner.rs` |
| Effect registry | `code/digimon-engine/src/cards.rs` |
| Card metadata | `digimon_gym/engine/data/cards.json` |
| Deck library | `digimon_gym/engine/data/deck_library.json` |
| C# reference | `DCGO/Assets/Scripts/CardEffect/{SET}/{COLOR}/{CLASS_NAME}.cs` (underscore naming) |
| Validated cards (Rust) | `qa/qa-reports/validated_cards_rust.json` |
| Archetype QA (Rust) | `qa/archetype-qa/rust/{archetype_slug}.md` |
| Engine gaps (Rust) | `qa/archetype-qa/engine-gaps-rust.md` |

---

## Phase 1: Resolve Card Pool

### 1a. Build archetype manifest

Reuse the existing Python tool `code/tools/resolve_deck.py`. It reads `digimon_gym/engine/data/deck_library.json`, which is populated upstream by `code/tools/meta_loader.py` from local-card-shop TCG meta decklists (DigimonMeta.com, Egman Events, DigimonCard.io, DigiLab). Agents do not re-scrape — treat `deck_library.json` as the authoritative deck pool and use `resolve_archetype` to extract the card manifest:

```python
import sys; sys.path.insert(0, '.')
from tools.resolve_deck import resolve_archetype

# If $ARGUMENTS contains --cards, pass as override:
# manifest = resolve_archetype('ARCHETYPE_NAME', cards_override=['CARD1', 'CARD2', ...])
manifest = resolve_archetype('ARCHETYPE_NAME')
```

`manifest.unique_cards` yields `CardEntry` objects with `card_id`, `card_name`, `card_kind`, `level`, `colors`, `traits`, `dp`, `play_cost`, `evo_costs`, `effect_text`, `inherited_text`, `security_text`, `csharp_path`, `deck_frequency`. `manifest.meta_share` and `manifest.best_decklist` come from the scraped tournament data. The Python-side `script_status` / `script_path` fields are **not used** — Rust has its own registry source of truth.

If `resolve_archetype` raises `UnknownArchetype`, tell the user and offer `python code/tools/resolve_deck.py --list-archetypes --min-meta-share 0.01` to find valid names. If `deck_library.json` is stale or missing entries for the archetype, refresh via `python code/tools/meta_loader.py --scrape-digimonmeta URL` / `--scrape-digilab` / `--build` before retrying.

### 1b. Determine Rust registration status

For each `card_id`, check whether it is already registered in Rust. Grep for the string `registry.insert("{card_id}"` across `code/digimon-engine/src/cards/` recursively:

```bash
grep -rn 'registry.insert("BT17-001"' code/digimon-engine/src/cards/
```

Classify each card as:
- **`SKIP (already in Rust)`** — present in a set module's `register()` call. Not dispatched.
- **`IMPLEMENT`** — not yet registered. Will be dispatched to a worker agent.

Also load `qa/qa-reports/validated_cards_rust.json` (create with empty `cards: {}` if absent). Any card with a prior `IMPLEMENTED` or `BLOCKED` verdict in that file is also `SKIP` (idempotency).

### 1c. Build reverse archetype map

Build a `card_id → [archetype_name, ...]` map by scanning all archetypes in `deck_library.json`. This lets the final report surface cross-archetype ownership (e.g., "BT17-050 is used by 3 other archetypes").

---

## Phase 2: Batch + Plan

### 2a. Group cards into batches

Default batch size is 4 (overridable). Grouping heuristics (same as `/batch-fix-cards`):
1. Cards that reference each other by name → same batch.
2. Tamer + its buffed Digimon → same batch.
3. Option cards + their target Digimon → same batch.
4. Remaining slots filled in card-ID order.

### 2b. Present plan and require approval

Print the batch plan as a table:

```
Archetype: Royal Knights
Total cards in pool: 17
Already in Rust:     3 (SKIP)
Prior verdict:       2 (SKIP)
To implement:        12 → 3 batches of 4

Batch 1: BT17-010, BT17-011, BT17-050, BT17-051
Batch 2: BT17-088, BT17-089, BT17-090, BT17-100
Batch 3: BT17-101, BT17-102, BT17-103, BT17-104
```

**Require explicit user confirmation before Phase 4.**

---

## Phase 3: Pre-read Shared Context (orchestrator)

Before dispatching any agents, read these files once and hold them for embedding in every agent prompt:

1. `docs/RUST_ENGINE_API.md` (full) — the scripting API reference.
2. `docs/RUST_PYTHON_PARITY.md` (full) — gating doc.
3. `qa/archetype-qa/engine-gaps-rust.md` (create if absent with header `# Engine Gaps Tracker (Rust)`).
4. `code/digimon-engine/src/cards/test_cards.rs` (full) — worked-example excerpt.
5. `code/digimon-engine/src/debug_runner.rs` — read the builder API section (lines around `pub fn builder`, `with_registry`, `add_card`, `hand`, `deck`, `security`, `play`, `place_on_field`, `attack_player`, `make_test_card`).

Pre-create directories if missing:
- `code/digimon-engine/src/cards/{set}/` — e.g. `code/digimon-engine/src/cards/bt17/`
- `code/digimon-engine/tests/behavioral/{set}/` — e.g. `code/digimon-engine/tests/behavioral/bt17/`
- `qa/archetype-qa/rust/`

`{set}` is the lowercased prefix from the card ID (`BT17-001` → `bt17`, `ST20-03` → `st20`, `AD1-011` → `ad1`).

---

## Phase 4: Batch Loop

Repeat for each batch.

### 4A. Gather per-card context (orchestrator)

For each of the 4 cards in the batch, collect:

1. **Card metadata** from `digimon_gym/engine/data/cards.json`:
   - `card_name_eng`, `effect_description_eng`, `inherited_effect_description_eng`, `security_effect_eng`
   - `card_kind`, `level`, `dp`, `play_cost`, `card_colors`, `type_eng` (traits), `evo_costs`, `dna_costs`

2. **C# reference**: glob `DCGO/Assets/Scripts/CardEffect/{SET}/*/{CLASS_NAME}.cs` where `{CLASS_NAME}` is `card_id.replace("-", "_")` (e.g. `BT17-001` → `BT17_001.cs`). Read the file if found.

3. **Prior verdict** (if any) from `validated_cards_rust.json`.

### 4B. Dispatch 4 parallel Opus workers

One Agent tool call per card, all in **a single assistant message** for true parallelism. Each call uses `subagent_type: "general-purpose"`, `isolation: "worktree"`, and the prompt template below. Use Opus (high effort).

**Worker agent prompt template** (all slots `{{…}}` filled by the orchestrator):

```
You are implementing a single Digimon TCG card effect in the Rust engine via TDD.

# Your card

Card ID: {{CARD_ID}}
Card Name: {{CARD_NAME}}
Card Kind: {{CARD_KIND}}
Level: {{LEVEL}}          DP: {{DP}}          Play Cost: {{PLAY_COST}}
Colors: {{COLORS}}
Traits (type_eng): {{TRAITS}}
Evo Costs: {{EVO_COSTS}}
DNA Costs: {{DNA_COSTS}}

## Effect text (authoritative — implement EXACTLY)
{{EFFECT_TEXT}}

## Inherited effect text
{{INHERITED_TEXT}}

## Security effect text
{{SECURITY_TEXT}}

## C# reference implementation (behavioral source of truth)
Path: {{CSHARP_PATH}}
```
{{CSHARP_BODY}}
```

## Prior verdict (if any)
{{PRIOR_VERDICT_JSON}}

# Engine context pack

## RUST_ENGINE_API.md (full)
{{RUST_ENGINE_API_BODY}}

## RUST_PYTHON_PARITY.md (full — consult BEFORE declaring BLOCKED)
{{PARITY_BODY}}

## engine-gaps-rust.md (current known gaps)
{{GAPS_BODY}}

## test_cards.rs (reference implementations)
{{TEST_CARDS_BODY}}

## DebugRunner builder API (from debug_runner.rs)
{{DEBUG_RUNNER_EXCERPT}}

# Your task

Deliverables (and ONLY these — do NOT touch `cards.rs` or any `mod.rs`):
1. `code/digimon-engine/src/cards/{{SET_LOWER}}/{{CARD_ID_LOWER_UNDERSCORE}}.rs` — the CardEffect struct.
2. `code/digimon-engine/tests/behavioral/{{SET_LOWER}}/{{CARD_ID_LOWER_UNDERSCORE}}.rs` — DebugRunner tests.

`{{CARD_ID_LOWER_UNDERSCORE}}` is the card id lowercased with `-` replaced by `_` (e.g. `BT17-001` → `bt17_001`).

## Workflow (TDD — follow in order)

**Step 1 — Decompose card text into numbered clauses.**
For each clause, capture: (a) timing (On Play / When Digivolving / Main / Security / Inherited / On Deletion / etc.), (b) exact text, (c) expected behavior, (d) C# mapping (which method in the reference file).

**Step 2 — Write DebugRunner tests FIRST.**
Create `code/digimon-engine/tests/behavioral/{{SET_LOWER}}/{{CARD_ID_LOWER_UNDERSCORE}}.rs`. One `#[test]` function per clause. For conditional effects, write BOTH a positive and a negative test. Use this skeleton:

```rust
//! Behavioral tests for {{CARD_ID}} — {{CARD_NAME}}

use digimon_engine::cards::CardEffectRegistry;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Expiry};
use std::sync::Arc;

// Local registration so we don't depend on build_registry() wiring yet.
// (The orchestrator will wire this into `cards.rs` after the review passes.)
fn registry_with_card() -> CardEffectRegistry {
    let mut r = CardEffectRegistry::new();
    // Replace with your card's struct path once implemented.
    // r.insert("{{CARD_ID}}", Arc::new(path::to::YourStruct));
    r
}

// Build a CardData with the card's real stats from cards.json.
// make_test_card defaults to Digimon/Lv3/DP2000/Cost3/Red — override as needed.
fn make_this_card() -> digimon_engine::card_data::CardData {
    let mut c = make_test_card("{{CARD_ID}}", "{{CARD_NAME}}");
    c.card_kind = CardKind::{{CARD_KIND_VARIANT}};
    c.level = {{LEVEL_OPT}};
    c.dp = {{DP_OPT}};
    c.play_cost = {{PLAY_COST}};
    c.colors = vec![{{COLORS_VEC}}];
    c.traits = vec![{{TRAITS_VEC}}];
    c.evo_costs = {{EVO_COSTS_VEC}};
    c.effect_text = "{{EFFECT_TEXT_ESCAPED}}".to_string();
    c.inherited_text = "{{INHERITED_TEXT_ESCAPED}}".to_string();
    c.security_text = "{{SECURITY_TEXT_ESCAPED}}".to_string();
    c
}

#[test]
fn clause_1_{{short_name}}() {
    let mut r = DebugRunner::builder()
        .add_card(make_this_card())
        .with_registry(registry_with_card())
        .hand(0, &["{{CARD_ID}}"])
        .memory({{memory_pre_fund}})
        .start();
    // ... exercise clause 1, assert state changes.
}
```

Tests must compile. They will FAIL at this point (no effect registered yet). Run:
```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test behavioral_{{SET_LOWER}}_{{CARD_ID_LOWER_UNDERSCORE}}
```
(or `cargo test --manifest-path code/digimon-engine/Cargo.toml {{CARD_ID_LOWER_UNDERSCORE}}` as a filter.) Confirm expected failures.

**Step 3 — Implement `CardEffect`.**
Create `code/digimon-engine/src/cards/{{SET_LOWER}}/{{CARD_ID_LOWER_UNDERSCORE}}.rs`. Follow the `Effect::on_play(card).name(...).condition(|ctx| {...}).process(|ctx| {...}).build()` builder pattern from RUST_ENGINE_API.md. One `CardEffect` struct whose `effects()` returns a `Vec<Effect>` containing every clause on the card (OnPlay, WhenDigivolving, Inherited, Security, etc.).

Zero-sized struct, no fields. Closures must be `Fn + Send + Sync + 'static`; capture only `Copy` handles.

Provide a local registration helper:
```rust
pub fn register(registry: &mut crate::cards::CardEffectRegistry) {
    registry.insert("{{CARD_ID}}", std::sync::Arc::new(YourStruct));
}
```

**Step 4 — Wire the test's local registry.**
Update `registry_with_card()` in the test file to insert your struct. Use the full path, e.g. `digimon_engine::cards::{{SET_LOWER}}::{{CARD_ID_LOWER_UNDERSCORE}}::YourStruct`. The card module is not yet exposed in `cards.rs`, so reach in via the crate path OR (simpler) re-declare a minimal `mod` wrapper inside the test file with `#[path = "..."]` if the crate path isn't visible.

The orchestrator will wire the permanent `pub mod` in Phase 4D — your job is only to make THIS test file compile and pass.

**Step 5 — Re-run tests.** Expect all green.

**Step 6 — Faithfulness self-audit** against the 16-item checklist (below). Flag any unresolved issue.

**Step 7 — Emit verdict.**

## Verdicts

- `IMPLEMENTED` — every clause implemented faithfully; all tests pass.
- `PARTIAL` — core clauses work; some nuance deferred with explicit comment. Explain precisely what's missing.
- `BLOCKED` — a required mechanic is not yet available in the Rust engine. Describe the missing API and append to `engine-gaps-rust.md` content block in your output.

**Never ship stubs, auto-selections, or silent drops.** If a clause needs a player choice the engine can't yet surface, the card is BLOCKED — not PARTIAL.

## 16-item error checklist

1. No clause from the card text is silently dropped.
2. Every player choice uses `ctx.select_*` — no `.iter().next()`, no `[0]`, no `min`/`max` over targets.
3. Optional effects (`(Optional)`, "you may") use the `.optional()` builder flag on the `Effect`.
4. Inherited effects use `Effect::inherited(card)` — separate `Effect` instances, not piggybacked on OnPlay.
5. Security effects use `Effect::security(card)` (timing `EffectTiming::SecuritySkill`).
6. `[When Attacking]` uses `EffectTiming::OnAttack` — verify against `docs/RUST_ENGINE_API.md`, do NOT parrot the Python enum name.
7. DP modifications use `ctx.add_dp_modifier(target, value, expiry)` with an explicit `Expiry::EndOfTurn` / `Expiry::EndOfOpponentsTurn` / etc. — never mutate fields directly.
8. Trait / name matching uses `CardSource::contains_card_name()` (case-insensitive substring) — never `==` over raw strings.
9. Alt-digivolve effects enumerate ALL qualifying traits/names from card text.
10. All closures are `Fn + Send + Sync + 'static`. Capture only `Copy` handles (`CardHandle`, `PermanentHandle`, `PlayerId`).
11. Never stash a `PermanentHandle` across a deletion — iterate high-index-to-low or snapshot `CardHandle`s first.
12. Field presence checks use `ctx.permanent_of(card).is_some()`, not index arithmetic.
13. Memory swings use `ctx.gain_memory` / `ctx.lose_memory` (honor seesaw semantics per parity doc §1.2–1.5).
14. Every `.process` closure is complete. No `pass`-equivalents, no TODOs.
15. Before declaring `BLOCKED`, consult `docs/RUST_PYTHON_PARITY.md` — the mechanic may already be implemented.
16. Any engine gap you hit must be appended to `engine-gaps-rust.md` in the verdict block (Orchestrator merges it): entry format `## {{CARD_ID}} — {{effect clause}}` with missing-API description.

## Output format (return ALL of this)

```
## {{CARD_ID}} — {{CARD_NAME}}

### Verdict: IMPLEMENTED|PARTIAL|BLOCKED

### Clause analysis
Clause 1 (OnPlay): "<exact text>"
  Expected: <behavior>
  Implemented in: <file:func>
  Tests: <test names>
  Status: MATCH | PARTIAL | BLOCKED
...

### Files created
- code/digimon-engine/src/cards/{{SET_LOWER}}/{{CARD_ID_LOWER_UNDERSCORE}}.rs ({{N_EFFECTS}} effects)
- code/digimon-engine/tests/behavioral/{{SET_LOWER}}/{{CARD_ID_LOWER_UNDERSCORE}}.rs ({{N_TESTS}} tests)

### Test output (cargo test ... -- --nocapture, trimmed)
<paste the final passing (or failing, for BLOCKED) test summary>

### Engine gaps discovered (if any)
## {{CARD_ID}} — <clause>
Missing API: <description>
Suggested addition: <signature or approach>

### New patterns worth documenting in RUST_ENGINE_API.md (if any)
- <pattern name>: <short description>
```
```

### 4C. Dispatch review agent

After all 4 worker agents return, dispatch ONE Opus review agent (`isolation`: none, read-only). Pass it each worker's verdict block plus the paths to the files they wrote in their worktrees. The orchestrator copies those files out of the worktrees first so the reviewer can read them directly.

**Reviewer agent prompt template:**

```
You are reviewing Rust CardEffect implementations produced by 4 worker agents in a TDD pipeline.

For each card, you have:
- The worker's verdict block (pasted below).
- The files they wrote: `code/digimon-engine/src/cards/{{SET_LOWER}}/{{card}}.rs` and `code/digimon-engine/tests/behavioral/{{SET_LOWER}}/{{card}}.rs`.
- The card's authoritative metadata (pasted below).
- The C# reference (pasted below).

# Engine API reference (for checking correct API usage)
{{RUST_ENGINE_API_BODY}}

# Per-card materials
{{PER_CARD_MATERIALS}}

# 16-item checklist (apply to every card)
{{CHECKLIST_FROM_4B}}

# Your task
For each card, emit ONE of:
  {{CARD_ID}}: APPROVED
or
  {{CARD_ID}}: NEEDS-FIX
  - Issue 1: <description> — Fix: <file:line directive>
  - Issue 2: ...

Be precise — the orchestrator applies your fix directives verbatim.
```

### 4D. Merge + wire registration (orchestrator)

The orchestrator — not an agent — performs these steps in order:

1. **Copy files out of worktrees** into the main tree:
   - `code/digimon-engine/src/cards/{set}/{card_id_lower_underscore}.rs`
   - `code/digimon-engine/tests/behavioral/{set}/{card_id_lower_underscore}.rs`

2. **Apply review fixes** from any `NEEDS-FIX` card.

3. **Create or update `code/digimon-engine/src/cards/{set}/mod.rs`**:

   ```rust
   pub mod bt17_010;
   pub mod bt17_011;
   // ...

   pub fn register(registry: &mut crate::cards::CardEffectRegistry) {
       bt17_010::register(registry);
       bt17_011::register(registry);
       // ...
   }
   ```

4. **Update `code/digimon-engine/src/cards.rs`**: add `pub mod {set};` and a `{set}::register(&mut registry);` call inside `build_registry()` (only if this is the first card from this set).

5. **Create or update `code/digimon-engine/tests/behavioral/{set}/mod.rs`** (or equivalent test discovery path):
   - Check how existing behavioral tests are discovered (`#[path = "..."]` vs `#[cfg(test)] mod tests { ... }`). For each new test file, add an inclusion line so `cargo test` picks it up.

6. **Run batch tests:**
   ```bash
   cargo test --manifest-path code/digimon-engine/Cargo.toml {set}_
   ```
   If any fail, do **one** targeted fix round. Do not loop indefinitely — a persistent failure means the review agent missed something; escalate to the user with the failing output.

7. **Update `qa/qa-reports/validated_cards_rust.json`** — one entry per card (schema below). Bump `version` once per batch, update `last_updated`.

8. **Append to `qa/archetype-qa/rust/{archetype_slug}.md`** — batch summary (per-card verdict, test count, notes).

9. **Append any new gaps** to `qa/archetype-qa/engine-gaps-rust.md`.

10. **Present batch summary to user:**

    ```
    Batch 1 complete (4/12 cards)
    | Card ID   | Verdict     | Review   | Tests | Notes |
    | BT17-010  | IMPLEMENTED | APPROVED | 3/3   | OnPlay draw + inherited DP |
    | BT17-011  | IMPLEMENTED | APPROVED | 2/2   | Main option, no targets |
    | BT17-050  | PARTIAL     | APPROVED | 4/5   | Name aliasing clause deferred |
    | BT17-051  | BLOCKED     | APPROVED | 0/0   | Needs OnSecurityCheck timing |

    Running totals: IMPLEMENTED=2  PARTIAL=1  BLOCKED=1
    ```

### 4E. Continue to next batch

Proceed automatically. The user can interrupt between batches.

---

## Phase 5: Final Report

After all batches:

1. **Summary table** — IMPLEMENTED / PARTIAL / BLOCKED counts across the whole archetype.
2. **Per-card results** — verdict, reviewer status, test count, one-line notes.
3. **Files created** — grouped by set.
4. **Blocked cards + engine gaps** — one section per gap with affected cards.
5. **Full-suite green check:**
   ```bash
   cargo test --manifest-path code/digimon-engine/Cargo.toml
   ```
   Must pass. If not, the skill has left the tree in a broken state — escalate.
6. **Finalize** `qa/archetype-qa/rust/{archetype_slug}.md` with the template below.

### `qa/archetype-qa/rust/{archetype_slug}.md` template

```markdown
# Archetype Rust Implementation: {Archetype Name}
Date: {YYYY-MM-DD}
Total cards in pool: {N}
Processed this run: {M}
Pipeline: batch-implement-cards-rust

## Summary
- IMPLEMENTED: {n}
- PARTIAL: {n}
- BLOCKED: {n}
- SKIPPED (already in Rust): {n}

## Per-Card Verdicts
| Card ID | Name | Verdict | Review | Tests | Notes |
|---------|------|---------|--------|-------|-------|
| ...     |      |         |        |       |       |

## Blocked Cards
### {CARD_ID} {card_name}
- Effect text: "..."
- Missing engine API: ...
- Suggested addition: ...

## New Patterns Discovered
- {pattern}: {short description} — propose adding to RUST_ENGINE_API.md
```

---

## `validated_cards_rust.json` schema

```json
{
  "version": 1,
  "last_updated": "YYYY-MM-DD",
  "cards": {
    "BT17-001": {
      "card_name": "WarGreymon",
      "validated_date": "YYYY-MM-DD",
      "report": "batch-implement-cards-rust",
      "status": "IMPLEMENTED",
      "archetype": "Royal Knights",
      "notes": "OnPlay memory gain + When Digivolving draw; tests 3/3 passing"
    }
  }
}
```

`status` ∈ `{IMPLEMENTED, PARTIAL, BLOCKED}`. Do NOT touch the Python `validated_cards.json`.

---

## Invariants (the orchestrator enforces these)

- Worker agents **never** edit `code/digimon-engine/src/cards.rs` or any `mod.rs`. If a worker does, reject its output and re-dispatch.
- The orchestrator **always** runs `cargo test --manifest-path code/digimon-engine/Cargo.toml` once in Phase 5. If it fails, leave the tree as-is and report to the user.
- Card-ID conventions: file names are `{lower_case}_{nnn}.rs` (underscore, `BT17-001` → `bt17_001.rs`). Registry keys are the original hyphenated ID (`"BT17-001"`).
- The Python `qa/qa-reports/validated_cards.json` is never modified.
- No Notion or Pinecone calls in v1.

## Known limitations (v1)

- No FIX mode — works only on unimplemented cards.
- No Notion sync — add once a Rust tracker schema is agreed.
- No Pinecone retrieval — agents use inline context only.
- Registration wiring is orchestrator-side; agents cannot add a new set module themselves.
