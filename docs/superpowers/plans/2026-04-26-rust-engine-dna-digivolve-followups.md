# Rust Engine: DNA Digivolve Follow-ups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Address the minor follow-ups surfaced during the [DNA digivolve execution plan's](2026-04-26-rust-engine-dna-digivolve-execute.md) reviews — test-helper deduplication, the `pay_memory_unchecked` extraction (eliminating the bypass-branch drift hazard), the hardcoded example-count assertions, doc-comment polish, and the deeply-nested stage-2 callback.

**Architecture:** Five small, independent tasks. Each is a single-concern refactor with TDD and a focused commit. None changes externally observable behavior; all are internal cleanup that pays down debt flagged in the previous plan's reviews.

**Tech Stack:** Rust (`code/digimon-engine/`), Cargo workspace tests.

**Sister plan:** [2026-04-26-rust-engine-dna-digivolve-execute.md](2026-04-26-rust-engine-dna-digivolve-execute.md) — the parent plan that landed Tasks 1-9 of the DNA digivolve work and surfaced these follow-ups.

---

## Out-of-scope items deferred to separate plans

These follow-ups were also flagged but warrant their own plans:

- **DSL `dna_costs` schema extension.** Currently DNA-digivolve cards must supply cost data via the `CardData` ingest path; the OnDnaDigivolve *clause* is DSL-authorable but the cost itself is not. Addressing this requires `CardSpec` schema work, validator updates, compile-pipeline wiring, and updates to `cards/_examples/TST_DNA_TRIGGER.yaml` to inline its cost. ~2-4 hours, owner: future Phase 2 work.
- **`Game::install_pending_selection(sel, phase)` primitive.** The stage-1 phase re-flip in `Game::initiate_dna_digivolve` (game_actions.rs:2336-2340) is necessary because `resolve_generic_selection` restores `current_phase` to `previous_phase` *before* invoking the callback. A unified install primitive would make this footgun structural rather than rule-by-convention. Touches every selection-installing site (~15+ sites in `effect_context/selections.rs` and `game_actions.rs`); architectural change, ~half-day.

---

## File Structure

**Modify:**
- `code/digimon-engine/src/debug_runner.rs` — add public DNA test helpers (Task 1)
- `code/digimon-engine/src/dna_digivolve.rs` — replace local `tests_stacking` helpers with the new `debug_runner` ones (Task 1)
- `code/digimon-engine/tests/dna_digivolve_user_action.rs` — replace local helpers with imports (Task 1)
- `code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs` — replace local helpers with imports (Task 1)
- `code/digimon-engine/src/game.rs` — add `Game::pay_memory_unchecked` (Task 2); update `dna_digivolve_inner`'s doc-comment cross-reference (Task 4)
- `code/digimon-engine/src/effect_context/mod.rs` — refactor bypass branch to use `pay_memory_unchecked` (Task 2)
- `code/digimon-engine/tests/dsl/embedded_registry.rs` — replace count assertion (Task 3)
- `code/digimon-engine/tests/dsl/phase0_exit.rs` — replace count assertion (Task 3)
- `code/digimon-engine/tests/dsl/phase1b_exit.rs` — replace count assertion (Task 3)
- `code/digimon-engine/tests/dsl/roundtrip.rs` — replace count assertion (Task 3)
- `code/digimon-engine/tests/dsl/phase2b_zone_moves.rs` — fix unused `tgt_handle` (Task 4)
- `code/digimon-engine/src/game_actions.rs` — extract stage-2 callback to `Game::resolve_dna_digivolve_stage2`, add `[Rejected]` log lines for silent no-op paths (Task 5)

---

## Tasks

### Task 1: Promote DNA test helpers to `debug_runner.rs`

Three test sites duplicate four near-identical helpers (`empty_req`, `lvl_req`, `lv_card`, `dna_card`/`dna_trigger_card_data`). Promote them to `debug_runner.rs` next to `make_test_card`.

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` (add helpers near line 504, after `make_test_egg`)
- Modify: `code/digimon-engine/src/dna_digivolve.rs` (remove local helpers from `tests_stacking` mod, line ~169-180; import the new ones)
- Modify: `code/digimon-engine/tests/dna_digivolve_user_action.rs` (remove local helpers at lines 10-43; import the new ones)
- Modify: `code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs` (remove local helpers at lines 23-57; import the new ones)

#### - [ ] Step 1: Add the helpers to `debug_runner.rs`

After `make_test_egg` in `code/digimon-engine/src/debug_runner.rs` (around line 504), add:

```rust
// ─── DNA-digivolve test helpers ───────────────────────────────────────

/// Build a `DnaRequirement` matching exactly one level. All other
/// constraint fields (colors, name_contains, text_contains) are empty.
pub fn dna_req_lv(level: u8) -> DnaRequirement {
    DnaRequirement {
        level,
        card_colors: Vec::new(),
        name_contains: String::new(),
        text_contains: String::new(),
    }
}

/// Build a minimal `CardData` with a non-default `level` set on top of
/// `make_test_card`'s defaults.
pub fn make_test_card_with_level(card_id: &str, card_name: &str, level: u8) -> CardData {
    let mut d = make_test_card(card_id, card_name);
    d.level = Some(level);
    d
}

/// Build a minimal Digimon `CardData` with a single `DnaCost` whose
/// `requirement1` and `requirement2` are level-only (`dna_req_lv`).
pub fn make_test_dna_card(
    card_id: &str,
    card_name: &str,
    req1_level: u8,
    req2_level: u8,
    memory_cost: i16,
) -> CardData {
    let mut d = make_test_card(card_id, card_name);
    d.dna_costs = vec![DnaCost {
        memory_cost,
        requirement1: dna_req_lv(req1_level),
        requirement2: dna_req_lv(req2_level),
    }];
    d
}
```

Add `DnaCost, DnaRequirement` to the existing `use crate::card_data::*;` import at the top of `debug_runner.rs`. (Verify with a Read of the imports section before editing — the file already imports `CardData` and `CardKind`; if the import shape is `use crate::card_data::{CardData, CardKind};`, extend it to `use crate::card_data::{CardData, CardKind, DnaCost, DnaRequirement};`.)

#### - [ ] Step 2: Verify the helpers compile

Run: `cargo build --manifest-path code/digimon-engine/Cargo.toml`
Expected: clean build, no warnings about the new helpers.

#### - [ ] Step 3: Replace `dna_digivolve.rs::tests_stacking` local helpers

In `code/digimon-engine/src/dna_digivolve.rs`, find the `#[cfg(test)] mod tests_stacking` (around line 161-228). Inside that module:

- Delete the local `empty_req()` function (around line 169).
- Delete the local `lvl_req(level: u8)` function (around line 178).
- Replace any `lvl_req(N)` call with `crate::debug_runner::dna_req_lv(N)`.
- Update the `use` block at the top of `tests_stacking` to add `use crate::debug_runner::dna_req_lv;` so the call sites can drop the `crate::debug_runner::` prefix.

The `lvl_card(idx, level)` and `perm_at(data_index)` helpers in this test module use `make_test_card` plus extra mutation that's specific to the unit-test setup (varying `card_id` per index). **Keep them local** — they're not duplicated elsewhere and have a narrower contract than the new public helpers.

#### - [ ] Step 4: Replace `tests/dna_digivolve_user_action.rs` local helpers

In `code/digimon-engine/tests/dna_digivolve_user_action.rs`, find the helpers at lines 10-43:

- Delete `empty_req()`, `lvl_req(level)`, `dna_card(card_id, name, req1_lv, req2_lv, mem)`, and `lv_card(card_id, name, level)`.
- Update the `use` block to add: `use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, make_test_dna_card};`.
- Replace `dna_card(...)` call sites with `make_test_dna_card(...)`.
- Replace `lv_card(...)` call sites with `make_test_card_with_level(...)`.

The signature shape is identical (same arg order). The `mem: i16` parameter on `dna_card` matches `make_test_dna_card`'s `memory_cost: i16`. No call-site adjustments beyond the rename.

#### - [ ] Step 5: Replace `tests/dsl/phase2g_on_dna_digivolve.rs` local helpers

In `code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs`, find the helpers at lines 23-57:

- Delete `empty_req()`, `lvl_req(level)`, `lv_card(card_id, name, level)`.
- Replace `dna_trigger_card_data()` (the bespoke "Lv7 with 5+6 DNA cost, no memory" builder at lines 48-57) by inlining: it has a single call site (likely the test setup); replace that call with `make_test_dna_card("TST-DNA-TRIGGER", "DnaTrigger", 5, 6, 0)` and additionally set `d.level = Some(7)` if the test relied on a specific level. (Verify by reading the test file — only mutate level if the original `dna_trigger_card_data` set one explicitly.)
- Update the `use` block to add: `use digimon_engine::debug_runner::{make_test_card_with_level, make_test_dna_card};`.

If the inlined call needs both DNA cost AND a non-default level, structure it as:

```rust
let mut card = make_test_dna_card("TST-DNA-TRIGGER", "DnaTrigger", 5, 6, 0);
card.level = Some(7);
runner_builder.add_card(card)
```

Don't introduce a new helper — single-call-site usage doesn't warrant one.

#### - [ ] Step 6: Run all DNA digivolve tests

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context effect_initiated_dna_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2g_on_dna_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --lib dna_digivolve::tests_stacking
```

Expected: all green. The 4 user-action tests, 6 effect-context tests, 2 DSL tests, 2 unit tests — 14 tests total, all pass.

#### - [ ] Step 7: Commit

```bash
git add code/digimon-engine/src/debug_runner.rs \
        code/digimon-engine/src/dna_digivolve.rs \
        code/digimon-engine/tests/dna_digivolve_user_action.rs \
        code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs
git commit -m "engine: dedupe DNA test helpers via debug_runner public API"
```

---

### Task 2: Extract `Game::pay_memory_unchecked`

The wrapper's `ignore_requirements && cost > 0` branch in `EffectContext::effect_initiated_dna_digivolve` directly mutates `self.game.memory` and emits `MemoryChange` — duplicating the body of `Game::pay_memory` minus the floor check. If `pay_memory` ever grows side effects (logging, hooks, end-turn auditing), this branch silently drifts. Centralize the floor-bypass primitive.

**Files:**
- Modify: `code/digimon-engine/src/game.rs` (add `pay_memory_unchecked` near line 827, after `pay_memory`)
- Modify: `code/digimon-engine/src/effect_context/mod.rs` (refactor bypass branch at lines 1961-1974)

#### - [ ] Step 1: Read the current `pay_memory` implementation

Read `code/digimon-engine/src/game.rs:801-827`. Confirm the body is:

```rust
pub fn pay_memory(&mut self, cost: u16) -> bool {
    if cost == 0 {
        return true;
    }
    let new_memory = self.memory - cost as i16;
    if new_memory < self.rules.memory_range.0 {
        return false;
    }
    let delta = new_memory - self.memory;
    self.memory = new_memory;
    let seq = self.next_event_seq();
    let player = self.turn_player();
    self.events.push(crate::events::GameEvent::MemoryChange {
        seq,
        player,
        delta,
        total: self.memory,
    });
    true
}
```

#### - [ ] Step 2: Add `pay_memory_unchecked` after `pay_memory`

Insert immediately after line 827 (the closing `}` of `pay_memory`):

```rust
/// Pay memory cost **without** the floor check. Used by effect-initiated
/// flows that explicitly opt out of the affordability constraint
/// (`ignore_requirements: true`). Always mutates and emits the
/// `MemoryChange` event — even if the resulting memory dips below
/// `rules.memory_range.0`.
///
/// Callers must have already decided that the floor check should be
/// skipped (typically because a printed effect overrides the normal
/// rules). For ordinary plays, use `pay_memory` instead.
///
/// `cost == 0` is a no-op (returns immediately, no event emitted) —
/// matches `pay_memory`'s zero-cost short-circuit.
pub(crate) fn pay_memory_unchecked(&mut self, cost: u16) {
    if cost == 0 {
        return;
    }
    let new_memory = self.memory - cost as i16;
    let delta = new_memory - self.memory;
    self.memory = new_memory;
    let seq = self.next_event_seq();
    let player = self.turn_player();
    self.events.push(crate::events::GameEvent::MemoryChange {
        seq,
        player,
        delta,
        total: self.memory,
    });
}
```

`pub(crate)` is the right visibility — the only caller is `EffectContext` in the same crate.

#### - [ ] Step 3: Refactor the bypass branch in `effect_context/mod.rs`

In `code/digimon-engine/src/effect_context/mod.rs:1961-1974`, the current bypass branch reads:

```rust
if ignore_requirements && effective_cost > 0 {
    let new_memory = self.game.memory - effective_cost as i16;
    let delta = new_memory - self.game.memory;
    self.game.memory = new_memory;
    let seq = self.game.next_event_seq();
    let player = self.game.turn_player();
    self.game
        .events
        .push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.game.memory,
        });
    // Pass cost=0 to the inner so it doesn't double-pay.
    self.game.dna_digivolve_inner(
        target_a,
        target_b,
        hand_owner,
        hand_index,
        0,
        false,
    )
} else { ... }
```

Replace lines 1961-1974 with:

```rust
if ignore_requirements && effective_cost > 0 {
    self.game.pay_memory_unchecked(effective_cost);
    // Pass cost=0 to the inner so it doesn't double-pay.
    self.game.dna_digivolve_inner(
        target_a,
        target_b,
        hand_owner,
        hand_index,
        0,
        false,
    )
} else { ... }
```

The `else` branch is unchanged.

#### - [ ] Step 4: Run all DNA digivolve tests

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context effect_initiated_dna_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
```

Expected: all green. The 6 effect-context tests + 4 user-action tests pass — including the test that exercises the `ignore_requirements: true` path.

#### - [ ] Step 5: Run the full crate suite

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: 252 pass / 4 fail. The 4 failures are pre-existing `data/cards.json` IO failures in `tests/dsl/{phase0_exit, real_cards_json}.rs` — unchanged from before.

#### - [ ] Step 6: Commit

```bash
git add code/digimon-engine/src/game.rs code/digimon-engine/src/effect_context/mod.rs
git commit -m "engine: extract Game::pay_memory_unchecked

Centralizes the floor-bypass memory mutation. Eliminates the
drift hazard between EffectContext's ignore_requirements branch
and Game::pay_memory."
```

---

### Task 3: Replace hardcoded `cards/_examples/` count assertions

Four DSL tests assert exact directory size (`assert_eq!(.., 16, ...)`). Every new example card touches all four. Replace with positive lookups + a `>=` floor — durable across additions.

**Files:**
- Modify: `code/digimon-engine/tests/dsl/embedded_registry.rs:5`
- Modify: `code/digimon-engine/tests/dsl/phase0_exit.rs:17`
- Modify: `code/digimon-engine/tests/dsl/phase1b_exit.rs:10`
- Modify: `code/digimon-engine/tests/dsl/roundtrip.rs:24`

#### - [ ] Step 1: Read each file's full context

Before editing, read each of the four files in full to understand what the count assertion is gating:

- `code/digimon-engine/tests/dsl/embedded_registry.rs`
- `code/digimon-engine/tests/dsl/phase0_exit.rs`
- `code/digimon-engine/tests/dsl/phase1b_exit.rs`
- `code/digimon-engine/tests/dsl/roundtrip.rs`

The count assertion is typically a smoke check that the embedded pack / loader picked up files. The semantically correct assertion is "at least the cards we care about are present", not "exactly N files exist".

#### - [ ] Step 2: `embedded_registry.rs` — replace exact count with floor + lookups

Current (line 5):
```rust
assert_eq!(registry.len(), 16, "expected 16 examples in embedded pack");
```

Replace with:
```rust
assert!(
    registry.len() >= 1,
    "embedded pack must contain at least 1 example card; got {}",
    registry.len()
);
```

Then verify the existing positive lookup at the bottom of that test (e.g., `registry.lookup("TST-DNA-TRIGGER").is_some()`) still asserts the cards-of-interest are present. If the test relies on the count for any other reason (e.g., to size a buffer), preserve that — but for documentation/smoke purposes, the floor is enough.

#### - [ ] Step 3: `phase0_exit.rs` — same treatment

Current (line 17):
```rust
assert_eq!(specs.len(), 16, "expected exactly 16 examples");
```

Replace with:
```rust
assert!(
    specs.len() >= 1,
    "phase 0 exit: at least 1 example must be present; got {}",
    specs.len()
);
```

#### - [ ] Step 4: `phase1b_exit.rs` — same treatment

Current (line 10):
```rust
assert_eq!(registry.len(), 16);
```

Replace with:
```rust
assert!(registry.len() >= 1, "phase 1b exit: registry must be non-empty");
```

#### - [ ] Step 5: `roundtrip.rs` — same treatment

Current (line 24):
```rust
assert_eq!(loaded.len(), 16, "expected 16 worked examples, got {}", loaded.len());
```

Replace with:
```rust
assert!(
    loaded.len() >= 1,
    "roundtrip: at least 1 example must round-trip; got {}",
    loaded.len()
);
```

The existing `every_example_round_trips` and `every_example_validates` tests in this file iterate over `loaded` and round-trip each individually — those provide the actual coverage. The count assertion was redundant smoke.

#### - [ ] Step 6: Run the DSL test suite

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl
```

Expected: same pass count as before (252-ish), same 4 pre-existing failures (`phase0_exit::phase_0_exit_criteria` and 3 `real_cards_json::*` — IO failures unrelated to this change). The count-assertion replacements should not introduce new failures.

#### - [ ] Step 7: Commit

```bash
git add code/digimon-engine/tests/dsl/embedded_registry.rs \
        code/digimon-engine/tests/dsl/phase0_exit.rs \
        code/digimon-engine/tests/dsl/phase1b_exit.rs \
        code/digimon-engine/tests/dsl/roundtrip.rs
git commit -m "engine: replace cards/_examples/ count asserts with floors

Adding a new example card no longer requires updating four
unrelated test files. Per-card lookups in the same tests
provide the positive coverage."
```

---

### Task 4: Doc-comment cross-reference + style polish + unused variable fix

Three independent micro-cleanups, batched into one commit.

**Files:**
- Modify: `code/digimon-engine/src/game.rs:1064-1066` (doc-comment cross-reference)
- Modify: `code/digimon-engine/src/game.rs:1119` (drop `.into_iter()`)
- Modify: `code/digimon-engine/tests/dsl/phase2b_zone_moves.rs:114` (use or remove `tgt_handle`)

#### - [ ] Step 1: Add cross-reference to `Game::initiate_dna_digivolve` in `dna_digivolve_inner`'s doc-comment

The doc-comment for `Game::dna_digivolve_inner` mentions only `EffectContext::effect_initiated_dna_digivolve` as a caller (around line 1064-1066). Add the user-action caller.

Read `code/digimon-engine/src/game.rs:1062-1066` first. The current text:

```rust
/// The pay-memory-bypass branch (`ignore_requirements && cost > 0`) is
/// *not* present here — callers that need to bypass the affordability
/// floor must subtract from `self.memory` before calling. See
/// `EffectContext::effect_initiated_dna_digivolve` for the wrapper that
/// implements that branch.
```

Replace with:

```rust
/// The pay-memory-bypass branch (`ignore_requirements && cost > 0`) is
/// *not* present here — callers that need to bypass the affordability
/// floor must subtract from `self.memory` before calling (see
/// `Game::pay_memory_unchecked`). The two callers are:
/// - `EffectContext::effect_initiated_dna_digivolve` — engine-effect
///   wrapper that handles the IR's `(cost, ignore_requirements)` shape
///   and invokes the bypass branch when needed.
/// - `Game::initiate_dna_digivolve`'s stage-2 selection callback — the
///   user-action path; passes the printed cost minus
///   `BeforePayCost` reductions and never bypasses.
```

(This refers to `Game::pay_memory_unchecked` from Task 2; ensure Task 2 lands first or this doc-comment will reference an unimplemented helper. Tasks should be done in order.)

#### - [ ] Step 2: Drop redundant `.into_iter()` in `dna_digivolve_inner`

In `code/digimon-engine/src/game.rs:1119`:

Current:
```rust
perm_a.card_sources.extend(perm_b.card_sources.into_iter());
```

Replace with:
```rust
perm_a.card_sources.extend(perm_b.card_sources);
```

`Vec<T>` implements `IntoIterator` natively; the explicit `.into_iter()` is redundant. Style nit; behavior unchanged.

#### - [ ] Step 3: Resolve the unused `tgt_handle` warning in phase2b_zone_moves.rs

Read `code/digimon-engine/tests/dsl/phase2b_zone_moves.rs:108-160` to confirm the test's intent.

The variable `tgt_handle` at line 114 is declared but never read — `src_handle` (line 113) is the only one used in `EffectContext::new`. The test then resolves the `SelectHand` selection by index, not by handle. The `tgt_handle` line is noise.

Two options, choose by intent:

**Option A (preferred — remove the dead binding):** Delete line 114 entirely:

```rust
// Before:
let src_handle = runner.game.players[0].hand[0].handle();
let tgt_handle = runner.game.players[0].hand[1].handle();

// After:
let src_handle = runner.game.players[0].hand[0].handle();
```

**Option B (use the binding):** If the test author intended to assert that the chosen hand-index 1 corresponds to `tgt_handle`, add an assertion. Read the surrounding assertions; if `runner.game.pending_selection.as_ref().unwrap().valid_action_ids[1]` is the test's interesting value, you could add `assert_eq!(runner.game.players[0].hand[1].handle(), tgt_handle, "hand[1] is the TGT card")` — but this is tautological. Don't add a fake assertion.

Default to Option A. Remove line 114.

#### - [ ] Step 4: Run the full crate suite

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: same pass count (252 pass / 4 pre-existing fail), no new warnings from the touched files.

Verify the `tgt_handle` warning is gone:

```bash
cargo build --manifest-path code/digimon-engine/Cargo.toml --tests 2>&1 | grep -E "tgt_handle|unused variable" || echo "  (no unused-variable warnings on tgt_handle — good)"
```

Expected: no matches.

#### - [ ] Step 5: Commit

```bash
git add code/digimon-engine/src/game.rs \
        code/digimon-engine/tests/dsl/phase2b_zone_moves.rs
git commit -m "engine: doc-comment cross-ref + style polish + drop unused tgt_handle"
```

---

### Task 5: Stage-2 callback extraction + `[Rejected]` log lines

The stage-2 callback in `Game::initiate_dna_digivolve` is ~85 lines deep inside the stage-1 closure (5 levels of indentation). Extract to a `Game::resolve_dna_digivolve_stage2` method to flatten. Also, both callbacks silently `return` on invalid state (`first_idx >= len`, `second_idx >= len`, `evo_hand_index >= len`, no matching DnaCost) — the surrounding `Game::initiate_dna_digivolve` body uses `self.logger.log("[Rejected] ...")` for diagnostics; mirror that pattern so the silent paths are observable.

**Files:**
- Modify: `code/digimon-engine/src/game_actions.rs:2197-2345` (stage-1 + stage-2 callbacks)
- Modify: `code/digimon-engine/src/game_actions.rs` (add `Game::resolve_dna_digivolve_stage2` method elsewhere in the impl block, near `initiate_dna_digivolve`)

#### - [ ] Step 1: Read the full callback structure

Read `code/digimon-engine/src/game_actions.rs:2197-2345` end-to-end. Verify:

- Stage-1 callback captures `selecting_player`, `previous_phase`, `hand_index`, `source_card` from the outer scope.
- Stage-2 callback captures `first_player`, `first_idx`, `evo_hand_index`, `evo_card`, `stage1_previous_phase` from the stage-1 scope.
- Stage-2 body (lines 2246-2333) is what we'll extract.

#### - [ ] Step 2: Add `Game::resolve_dna_digivolve_stage2` method

In `code/digimon-engine/src/game_actions.rs`, immediately after the closing `}` of `Game::initiate_dna_digivolve` (around line 2345), add:

```rust
/// Stage-2 resolution of `Game::initiate_dna_digivolve`'s two-stage
/// selection chain. Receives the chosen second-material `battle_area`
/// index and the captured stage-1 state. Re-resolves the matching
/// `DnaCost` orientation, applies `BeforePayCost` reductions, calls
/// `Game::dna_digivolve_inner`, and triggers the auto-end-of-turn
/// check (mirroring `digivolve_from_hand`).
///
/// Defensively re-validates indices because triggered effects fired
/// during stage-1 install can mutate the battle area between selection
/// install and resolution.
///
/// Returns `()` rather than `bool` — failures are logged via the
/// engine `logger` and otherwise leave game state restored to the
/// caller's responsibility (the `pending_selection` was already
/// consumed by `resolve_generic_selection` before this method ran).
pub(crate) fn resolve_dna_digivolve_stage2(
    &mut self,
    first_player: PlayerId,
    first_idx: usize,
    second_idx: usize,
    evo_hand_index: usize,
) {
    if second_idx >= self.player(first_player).battle_area.len() {
        self.logger.log(&format!(
            "[Rejected] resolve_dna_digivolve_stage2: second index {} out of range (battle_area size={})",
            second_idx,
            self.player(first_player).battle_area.len()
        ));
        return;
    }
    if first_idx == second_idx {
        self.logger.log(
            "[Rejected] resolve_dna_digivolve_stage2: first and second indices are equal",
        );
        return;
    }
    if evo_hand_index >= self.player(first_player).hand.len() {
        self.logger.log(&format!(
            "[Rejected] resolve_dna_digivolve_stage2: evo hand index {} out of range (hand size={})",
            evo_hand_index,
            self.player(first_player).hand.len()
        ));
        return;
    }

    let evo_meta =
        &self.card_data[self.player(first_player).hand[evo_hand_index].data_index];
    let battle = &self.player(first_player).battle_area;
    let perm_first = &battle[first_idx];
    let perm_second = &battle[second_idx];

    let stacking = crate::dna_digivolve::get_dna_stacking_order(
        evo_meta,
        perm_first,
        perm_second,
        &self.card_data,
    );
    let Some((first_is_top, dna_cost)) = stacking else {
        self.logger.log(
            "[Rejected] resolve_dna_digivolve_stage2: no matching DnaCost for chosen pair",
        );
        return;
    };
    let printed_cost = dna_cost.memory_cost;

    let (target_a, target_b) = if first_is_top {
        (
            PermanentHandle {
                player: first_player,
                index: first_idx as u8,
            },
            PermanentHandle {
                player: first_player,
                index: second_idx as u8,
            },
        )
    } else {
        (
            PermanentHandle {
                player: first_player,
                index: second_idx as u8,
            },
            PermanentHandle {
                player: first_player,
                index: first_idx as u8,
            },
        )
    };

    let total_reduction = self.scan_before_pay_cost_reduction(first_player);
    let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

    let _ = self.dna_digivolve_inner(
        target_a,
        target_b,
        first_player,
        evo_hand_index,
        effective_cost,
        true,
    );

    self.check_turn_end();
}
```

The exact `use crate::card_source::PermanentHandle;` import may be needed at the top of `game_actions.rs` — if `PermanentHandle` is already imported there (it almost certainly is, since the file uses it elsewhere), no import changes are needed. Verify by checking the file's existing imports.

The `printed_cost` is `i16` (per Task 5's note in the parent plan that `DnaCost::memory_cost` is `i16`). The cast `printed_cost as i32` widens correctly; `.max(0) as u16` clamps to non-negative and narrows.

#### - [ ] Step 3: Replace the stage-2 closure body with a delegation

In `code/digimon-engine/src/game_actions.rs:2246-2333` (the inner closure body), replace the entire body with:

```rust
callback: Box::new(move |game: &mut Game, action_id: u16| {
    let second_idx = action_id as usize;
    game.resolve_dna_digivolve_stage2(
        first_player,
        first_idx,
        second_idx,
        evo_hand_index,
    );
}),
```

The closure now captures only what it forwards (`first_player: PlayerId`, `first_idx: usize`, `evo_hand_index: usize` — all `Copy`), and the bulk of the logic lives at method indentation (1-2 levels) instead of closure-nesting indentation (5 levels).

The `evo_card` and `stage1_previous_phase` captures are no longer needed in the inner closure — the new method doesn't reference them. Remove their captures from the stage-1 closure (around line 2235). Specifically:

- The line `let stage1_previous_phase = previous_phase;` (around line 2235) becomes dead — remove it.
- The line that introduces `evo_card` for the stage-2 closure (look around the variable's first appearance in the stage-1 callback body) becomes dead — remove it.

(Verify no other site in the stage-1 callback uses these locals.)

#### - [ ] Step 4: Add `[Rejected]` log lines to stage-1 silent no-ops

In `code/digimon-engine/src/game_actions.rs`, the stage-1 callback (around lines 2199-2231) has three silent no-op paths:

1. `first_idx >= game.player(first_player).battle_area.len()` (around line 2212) — returns silently.
2. `evo_hand_index >= game.player(first_player).hand.len()` (around line 2215, if present — verify by reading; the validation may be implicit elsewhere).
3. `second_targets.is_empty()` (around line 2229) — returns silently.

For each, add a `game.logger.log(&format!("[Rejected] ..."))` call before the `return`. Match the style of the surrounding `[Rejected]` logs in `Game::initiate_dna_digivolve` itself (lines 2117, 2125, 2135, 2146, 2175). Example:

```rust
if first_idx >= game.player(first_player).battle_area.len() {
    game.logger.log(&format!(
        "[Rejected] dna_digivolve stage 1: first index {} out of range (battle_area size={})",
        first_idx,
        game.player(first_player).battle_area.len()
    ));
    return;
}
```

```rust
if second_targets.is_empty() {
    game.logger.log(&format!(
        "[Rejected] dna_digivolve stage 1: no valid second-material targets for first index {}",
        first_idx
    ));
    return;
}
```

#### - [ ] Step 5: Run all DNA digivolve tests

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dna_digivolve_user_action
cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context effect_initiated_dna_digivolve
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl phase2g_on_dna_digivolve
```

Expected: all green. Behavior is unchanged — the extraction is pure refactor; the log lines fire on paths the tests don't normally exercise.

#### - [ ] Step 6: Run the full crate suite

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: 252 pass / 4 pre-existing fail. No new regressions.

#### - [ ] Step 7: Commit

```bash
git add code/digimon-engine/src/game_actions.rs
git commit -m "engine: extract resolve_dna_digivolve_stage2 + add rejection logs

Stage-2 logic now lives in a Game method at 1-2 levels of indent
instead of buried 5 levels deep in nested closures. Silent no-op
paths now log [Rejected] messages for diagnosability."
```

---

## Self-Review Checklist

After all 5 tasks land, verify:

1. **Spec coverage:**
   - ✅ Task 1: 3 sites consume `dna_req_lv` / `make_test_card_with_level` / `make_test_dna_card`; local helpers gone.
   - ✅ Task 2: `Game::pay_memory_unchecked` exists; `EffectContext` bypass branch is 2 lines instead of 14.
   - ✅ Task 3: 4 hardcoded counts replaced with `>=` floors.
   - ✅ Task 4: Doc-comment lists both callers; `.into_iter()` gone; `tgt_handle` line removed.
   - ✅ Task 5: `Game::resolve_dna_digivolve_stage2` exists; stage-2 closure is ≤10 lines; `[Rejected]` logs on stage-1 silent paths.

2. **No placeholders:** Every code block shows real code; every command has expected output.

3. **Type consistency:** `dna_req_lv: u8 → DnaRequirement`, `make_test_dna_card(card_id, name, req1_level, req2_level, memory_cost: i16)`, `Game::pay_memory_unchecked(cost: u16)`, `Game::resolve_dna_digivolve_stage2(first_player, first_idx, second_idx, evo_hand_index)` — all consistent across tasks.

4. **No NEW regressions:** The 4 pre-existing `data/cards.json` failures remain unchanged. No other tests fail.

5. **Out-of-scope items honored:** No DSL `dna_costs` schema work. No `install_pending_selection` primitive. Both flagged for separate plans in the header.

---

## Reference Index

- **Sister plan (parent):** [2026-04-26-rust-engine-dna-digivolve-execute.md](2026-04-26-rust-engine-dna-digivolve-execute.md)
- **Test-helper duplications today:**
  - [src/dna_digivolve.rs:169-180](../code/digimon-engine/src/dna_digivolve.rs) — `tests_stacking` mod's `empty_req` + `lvl_req`.
  - [tests/dna_digivolve_user_action.rs:10-43](../code/digimon-engine/tests/dna_digivolve_user_action.rs) — `empty_req` + `lvl_req` + `dna_card` + `lv_card`.
  - [tests/dsl/phase2g_on_dna_digivolve.rs:23-57](../code/digimon-engine/tests/dsl/phase2g_on_dna_digivolve.rs) — `empty_req` + `lvl_req` + `lv_card` + `dna_trigger_card_data`.
- **Bypass branch source:** [effect_context/mod.rs:1961-1974](../code/digimon-engine/src/effect_context/mod.rs)
- **`pay_memory` reference:** [game.rs:801-827](../code/digimon-engine/src/game.rs)
- **Hardcoded counts:** [embedded_registry.rs:5](../code/digimon-engine/tests/dsl/embedded_registry.rs), [phase0_exit.rs:17](../code/digimon-engine/tests/dsl/phase0_exit.rs), [phase1b_exit.rs:10](../code/digimon-engine/tests/dsl/phase1b_exit.rs), [roundtrip.rs:24](../code/digimon-engine/tests/dsl/roundtrip.rs)
- **Stage-2 callback:** [game_actions.rs:2246-2333](../code/digimon-engine/src/game_actions.rs)
- **Unused `tgt_handle`:** [tests/dsl/phase2b_zone_moves.rs:114](../code/digimon-engine/tests/dsl/phase2b_zone_moves.rs)
