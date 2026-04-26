# DebugRunner Helpers for DSL Card Tests — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the DebugRunner helpers spec'd in `docs/RUST_DSL_TEST_API.md` §3 so per-card DSL tests can be written.

**Architecture:** Extend `code/digimon-engine/src/debug_runner.rs` with new methods and `DebugRunnerBuilder` verbs. Add a small public accessor on `Game` for non-draining event-log access. Helpers are thin views over existing engine APIs (`Game::resolve_selection`, `Game::pending_selection`, `CardRegistry::lookup`, `DslCardEffect`); no new state machines.

**Tech Stack:** Rust 2021, `digimon-engine` crate, `digimon-dsl` crate (path dep), `serde_yml` for inline YAML parsing.

**Reference:** [`docs/RUST_DSL_TEST_API.md`](../../RUST_DSL_TEST_API.md) §3 enumerates every helper this plan implements. Each helper is currently marked **(spec)** in that doc; Task 10 strips those markers.

---

## File structure

| File | Responsibility |
|---|---|
| `code/digimon-engine/src/game.rs` | Add `pub fn events(&self) -> &[GameEvent]`. One-line accessor; non-draining read. |
| `code/digimon-engine/src/debug_runner.rs` | All new runner methods and builder verbs. Single file owns the test harness surface. |
| `code/digimon-engine/tests/debug_runner_dsl.rs` | New integration test file covering every new helper. Existing `debug_runner.rs` `#[cfg(test)] mod tests` covers in-module unit tests. |
| `docs/RUST_DSL_TEST_API.md` | Strip **(spec)** markers in §3 once helpers exist. |

`DebugRunner` struct gains:
- `compiled_cards: HashMap<String, Arc<CompiledCard>>` — populated by `dsl_card`/`from_dsl_yaml` for structural assertions.

`DebugRunnerBuilder` gains:
- `compiled_cards: HashMap<String, Arc<CompiledCard>>` — same map, threaded through `build_inner`.

No new files in `src/`. Single integration-test binary added under `tests/`.

---

## Task ordering rationale

Tasks 1–6 deliver test-driving primitives (events, pending selection, action submission). Tasks 7–9 deliver card-loading primitives. Tasks are ordered so each builds on the last; the integration-test file (`tests/debug_runner_dsl.rs`) accumulates cases across tasks.

---

### Task 1: Add `Game::events()` accessor

**Files:**
- Modify: `code/digimon-engine/src/game.rs:611` (within the "Event accumulator" section, just above `next_event_seq`)

**Why:** `Game::drain_events` removes events from the buffer. The DebugRunner's checkpoint helpers (Task 2) need a non-draining read so test code can capture a checkpoint, do work, and slice events emitted since the checkpoint.

- [ ] **Step 1: Add the failing test**

Add to `code/digimon-engine/src/debug_runner.rs` at the end of the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn game_events_accessor_returns_emitted_events_without_draining() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    let before = r.game.events().len();
    r.play(0, 0);
    let after = r.game.events().len();
    assert!(after > before, "play should emit events without draining");

    // Calling events() again returns the same slice (non-draining).
    let same = r.game.events().len();
    assert_eq!(same, after);
}
```

- [ ] **Step 2: Run the test — expect failure**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml debug_runner::tests::game_events_accessor_returns_emitted_events_without_draining
```

Expected: compilation error `no method named events found for struct Game`.

- [ ] **Step 3: Implement the accessor**

In `code/digimon-engine/src/game.rs`, in the "Event accumulator" section just above `pub fn next_event_seq`:

```rust
/// Borrow the accumulated event log without draining. Tests use this
/// via `DebugRunner::events_since` to assert on event emission across
/// a known checkpoint.
pub fn events(&self) -> &[crate::events::GameEvent] {
    &self.events
}
```

- [ ] **Step 4: Run the test — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml debug_runner::tests::game_events_accessor_returns_emitted_events_without_draining
```

Expected: PASS.

- [ ] **Step 5: Run the full engine suite to confirm no regression**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/game.rs code/digimon-engine/src/debug_runner.rs
git commit -m "engine: add Game::events() non-draining accessor for runner helpers"
```

---

### Task 2: Event-log helpers on DebugRunner

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` (add three methods to the `impl DebugRunner` block, just before the existing `// ─── Builder for DebugRunner.` separator at the closing of the impl)
- Create: `code/digimon-engine/tests/debug_runner_dsl.rs` (new integration-test file)

- [ ] **Step 1: Create the failing integration test**

Create `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
//! Integration tests for DSL-aware DebugRunner helpers.
//! See docs/RUST_DSL_TEST_API.md §3 for the helper contracts.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;

#[test]
fn event_checkpoint_and_events_since_slice_correctly() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    // Play once before the checkpoint — these events must NOT appear in the slice.
    r.play(0, 0);
    let cp = r.event_checkpoint();
    let before_count = r.event_checkpoint();
    assert_eq!(cp, before_count, "checkpoint is just the current length");

    // Set up another play after the checkpoint.
    let r2_card = make_test_card("TEST-002", "TestTwo");
    let mut r = DebugRunner::builder()
        .add_card(r2_card.clone())
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001", "TEST-002"])
        .memory(10)
        .start();

    let _ = r.play(0, 0);
    let cp = r.event_checkpoint();
    let _ = r.play(0, 0);

    let since: Vec<&GameEvent> = r.events_since(cp).iter().collect();
    assert!(!since.is_empty(), "play after checkpoint emits events");
    assert!(
        since.iter().all(|e| match e {
            GameEvent::MemoryChange { seq, .. }
            | GameEvent::Play { seq, .. }
            | GameEvent::TurnStart { seq, .. }
            | GameEvent::PhaseChange { seq, .. }
            | GameEvent::Digivolve { seq, .. }
            | GameEvent::Attack { seq, .. }
            | GameEvent::Trash { seq, .. } => *seq as usize >= cp,
            _ => true,
        }),
        "every event in slice has seq >= checkpoint"
    );
}

#[test]
fn events_of_kind_filters_by_predicate() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    let cp = r.event_checkpoint();
    r.play(0, 0);

    let memory_events = r.events_of_kind(cp, |e| matches!(e, GameEvent::MemoryChange { .. }));
    let play_events = r.events_of_kind(cp, |e| matches!(e, GameEvent::Play { .. }));

    assert!(!memory_events.is_empty(), "play emits at least one MemoryChange");
    assert_eq!(play_events.len(), 1, "play emits exactly one Play event");
}
```

- [ ] **Step 2: Run the failing test**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl
```

Expected: compile error `no method named event_checkpoint found for struct DebugRunner`.

- [ ] **Step 3: Implement the helpers**

In `code/digimon-engine/src/debug_runner.rs`, inside `impl DebugRunner`, just before the closing `}` of the impl block (after `pub fn perm_handle`):

```rust
// ─── Event log helpers (see docs/RUST_DSL_TEST_API.md §3) ─────────

/// Snapshot the current event-log length. Pair with `events_since` to
/// assert on events emitted by a specific runner action.
pub fn event_checkpoint(&self) -> usize {
    self.game.events().len()
}

/// Slice of events emitted since `checkpoint`. The slice is borrowed
/// from the live event log; do not retain across runner mutations.
pub fn events_since(&self, checkpoint: usize) -> &[crate::events::GameEvent] {
    let events = self.game.events();
    let start = checkpoint.min(events.len());
    &events[start..]
}

/// Filtered borrow of events emitted since `checkpoint`. Predicate is
/// evaluated per event; matches are returned in emission order.
pub fn events_of_kind<F>(
    &self,
    checkpoint: usize,
    predicate: F,
) -> Vec<&crate::events::GameEvent>
where
    F: Fn(&crate::events::GameEvent) -> bool,
{
    self.events_since(checkpoint)
        .iter()
        .filter(|e| predicate(e))
        .collect()
}
```

- [ ] **Step 4: Run the tests — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl
```

Expected: PASS for both tests.

- [ ] **Step 5: Run the full engine suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: add event-log checkpoint and slice helpers"
```

---

### Task 3: Pending-selection read accessors

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` (add five methods to `impl DebugRunner`)
- Modify: `code/digimon-engine/tests/debug_runner_dsl.rs` (add tests)

**Why:** §3 of the doc lists `pending_selection`, `pending_selection_view`, `pending_kind`, `pending_is_optional`, `pending_action_count`. All are read-only views over `Game::pending_selection`.

- [ ] **Step 1: Add the failing tests**

Append to `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
use digimon_engine::selection::SelectionKind;

#[test]
fn pending_selection_accessors_return_none_when_no_selection() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    assert!(r.pending_selection().is_none());
    assert!(r.pending_selection_view().is_none());
    assert!(r.pending_kind().is_none());
    assert_eq!(r.pending_action_count(), 0);
    assert!(!r.pending_is_optional());
}

#[test]
fn pending_selection_accessors_reflect_installed_selection() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::selection::{
        EffectChoiceEntry, PendingSelection, SelectionKind,
    };

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    // Install a synthetic EffectChoice selection directly (no card needed).
    let card_handle = r.game.players[0].hand[0].handle();
    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: r.game.current_phase,
        valid_action_ids: vec![100, 101],
        is_optional: true,
        prompt: "Test prompt".to_string(),
        effect_choices: Some(vec![
            EffectChoiceEntry { label: "A".to_string(), action_id: 100 },
            EffectChoiceEntry { label: "B".to_string(), action_id: 101 },
        ]),
        source_card: card_handle,
        source_permanent: None,
        callback: Box::new(|_, _| {}),
        on_decline: None,
    });

    assert!(r.pending_selection().is_some());
    let view = r.pending_selection_view().expect("view should exist");
    assert_eq!(view.kind, SelectionKind::EffectChoice);
    assert_eq!(view.valid_action_ids, vec![100, 101]);
    assert_eq!(r.pending_kind(), Some(SelectionKind::EffectChoice));
    assert!(r.pending_is_optional());
    assert_eq!(r.pending_action_count(), 2);
}
```

- [ ] **Step 2: Run the failing tests**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl pending_selection
```

Expected: compile errors for missing methods.

- [ ] **Step 3: Implement the accessors**

In `code/digimon-engine/src/debug_runner.rs`, after the event-log helpers from Task 2:

```rust
// ─── Pending selection accessors (see docs/RUST_DSL_TEST_API.md §3) ─

/// Borrow the currently-parked selection, if any.
pub fn pending_selection(&self) -> Option<&crate::selection::PendingSelection> {
    self.game.pending_selection.as_ref()
}

/// Cloneable, callback-free snapshot of the pending selection.
pub fn pending_selection_view(&self) -> Option<crate::selection::PendingSelectionView> {
    self.game.pending_selection.as_ref().map(|sel| sel.view())
}

/// Convenience: the selection kind, if a selection is installed.
pub fn pending_kind(&self) -> Option<crate::selection::SelectionKind> {
    self.game.pending_selection.as_ref().map(|sel| sel.kind)
}

/// Whether `PASS` (action 62) is a legal action on the current prompt.
/// Returns `false` if no selection is installed.
pub fn pending_is_optional(&self) -> bool {
    self.game
        .pending_selection
        .as_ref()
        .map(|sel| sel.is_optional)
        .unwrap_or(false)
}

/// Number of legal action IDs (excluding PASS). Returns 0 if no
/// selection is installed.
pub fn pending_action_count(&self) -> usize {
    self.game
        .pending_selection
        .as_ref()
        .map(|sel| sel.valid_action_ids.len())
        .unwrap_or(0)
}
```

- [ ] **Step 4: Run the tests — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl pending_selection
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: add pending-selection read accessors"
```

---

### Task 4: `execute_action`

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` (one method)
- Modify: `code/digimon-engine/tests/debug_runner_dsl.rs` (one test)

**Why:** Wraps `Game::resolve_selection` with the `selecting_player` already known to the runner. Tests no longer have to look up the player every time they submit an action.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
#[test]
fn execute_action_resolves_pending_selection() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};
    use std::sync::{Arc, Mutex};

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    let card_handle = r.game.players[0].hand[0].handle();
    let chosen = Arc::new(Mutex::new(None::<u16>));
    let chosen_for_cb = chosen.clone();

    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: r.game.current_phase,
        valid_action_ids: vec![100, 101],
        is_optional: false,
        prompt: "Test prompt".to_string(),
        effect_choices: Some(vec![
            EffectChoiceEntry { label: "A".to_string(), action_id: 100 },
            EffectChoiceEntry { label: "B".to_string(), action_id: 101 },
        ]),
        source_card: card_handle,
        source_permanent: None,
        callback: Box::new(move |_, action_id| {
            *chosen_for_cb.lock().unwrap() = Some(action_id);
        }),
        on_decline: None,
    });

    r.execute_action(101).expect("action 101 must resolve");
    assert_eq!(*chosen.lock().unwrap(), Some(101));
    assert!(r.pending_selection().is_none(), "selection cleared after resolve");
}
```

- [ ] **Step 2: Run the test — expect failure**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl execute_action
```

Expected: compile error.

- [ ] **Step 3: Implement `execute_action`**

In `code/digimon-engine/src/debug_runner.rs`, after `pending_action_count`:

```rust
// ─── Action submission ────────────────────────────────────────────

/// Submit an action ID to the parked selection. The selecting player
/// is read from the selection itself — callers don't have to track it.
///
/// Returns `Err(SelectionError)` if no selection is installed or the
/// action is not legal.
pub fn execute_action(
    &mut self,
    action_id: u16,
) -> Result<(), crate::selection::SelectionError> {
    let player = match self.game.pending_selection.as_ref() {
        Some(sel) => sel.selecting_player,
        None => return Err(crate::selection::SelectionError::NoPendingSelection),
    };
    self.game.resolve_selection(player, action_id)
}
```

- [ ] **Step 4: Run the test — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl execute_action
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: add execute_action wrapper over Game::resolve_selection"
```

---

### Task 5: `auto_resolve`

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` (one method)
- Modify: `code/digimon-engine/tests/debug_runner_dsl.rs` (one test)

**Why:** Mirror of Python `runner.auto_resolve()`. Picks the first legal action at every prompt until no `pending_selection` remains. Used when the test asserts end-state aggregates rather than branch-specific behavior.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
#[test]
fn auto_resolve_drains_chained_selections() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};
    use std::sync::{Arc, Mutex};

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    let card_handle = r.game.players[0].hand[0].handle();
    let resolution_count = Arc::new(Mutex::new(0u32));

    // First selection installs a second selection inside its callback.
    let cb_count = resolution_count.clone();
    let card2 = card_handle;
    let cb1: Box<dyn FnOnce(&mut digimon_engine::game::Game, u16) + Send + Sync> =
        Box::new(move |game, _action_id| {
            *cb_count.lock().unwrap() += 1;
            // Install a follow-up selection.
            let cb_count2 = cb_count.clone();
            game.pending_selection = Some(PendingSelection {
                kind: SelectionKind::EffectChoice,
                selecting_player: 0,
                previous_phase: game.current_phase,
                valid_action_ids: vec![200, 201],
                is_optional: false,
                prompt: "Second prompt".to_string(),
                effect_choices: Some(vec![
                    EffectChoiceEntry { label: "X".to_string(), action_id: 200 },
                    EffectChoiceEntry { label: "Y".to_string(), action_id: 201 },
                ]),
                source_card: card2,
                source_permanent: None,
                callback: Box::new(move |_, _| {
                    *cb_count2.lock().unwrap() += 1;
                }),
                on_decline: None,
            });
        });

    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: r.game.current_phase,
        valid_action_ids: vec![100, 101],
        is_optional: false,
        prompt: "First prompt".to_string(),
        effect_choices: Some(vec![
            EffectChoiceEntry { label: "A".to_string(), action_id: 100 },
            EffectChoiceEntry { label: "B".to_string(), action_id: 101 },
        ]),
        source_card: card_handle,
        source_permanent: None,
        callback: cb1,
        on_decline: None,
    });

    r.auto_resolve();
    assert!(r.pending_selection().is_none(), "all selections resolved");
    assert_eq!(*resolution_count.lock().unwrap(), 2, "both callbacks fired");
}

#[test]
fn auto_resolve_passes_when_optional_and_no_actions() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::selection::{PendingSelection, SelectionKind};
    use std::sync::{Arc, Mutex};

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    let card_handle = r.game.players[0].hand[0].handle();
    let declined = Arc::new(Mutex::new(false));
    let declined_cb = declined.clone();

    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::Hand,
        selecting_player: 0,
        previous_phase: r.game.current_phase,
        valid_action_ids: vec![], // no legal targets
        is_optional: true,
        prompt: "Optional with no targets".to_string(),
        effect_choices: None,
        source_card: card_handle,
        source_permanent: None,
        callback: Box::new(|_, _| panic!("callback must not fire on PASS")),
        on_decline: Some(Box::new(move |_| {
            *declined_cb.lock().unwrap() = true;
        })),
    });

    r.auto_resolve();
    assert!(r.pending_selection().is_none());
    assert!(*declined.lock().unwrap(), "on_decline fired via PASS");
}
```

- [ ] **Step 2: Run the failing tests**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl auto_resolve
```

Expected: compile error.

- [ ] **Step 3: Implement `auto_resolve`**

In `code/digimon-engine/src/debug_runner.rs`, after `execute_action`:

```rust
/// Drive every pending selection by submitting the first legal action
/// (or `PASS` when the prompt is optional and has no legal targets).
/// Loops until no selection is parked.
///
/// **Do not** use when testing a specific branch — `auto_resolve` always
/// picks `valid_action_ids[0]`, so it cannot tell you which branch fired.
/// Use `execute_action` / `execute_branch` for branch-specific tests, then
/// call `auto_resolve` after the branching decision is locked.
///
/// Hard cap of 256 iterations as a runaway-loop guard.
pub fn auto_resolve(&mut self) {
    const MAX_ITERATIONS: u32 = 256;
    for _ in 0..MAX_ITERATIONS {
        let action_id = match self.game.pending_selection.as_ref() {
            None => return,
            Some(sel) => {
                if let Some(&a) = sel.valid_action_ids.first() {
                    a
                } else if sel.is_optional {
                    crate::action::space::PASS
                } else {
                    panic!(
                        "auto_resolve: pending selection has no legal actions \
                         and is not optional (kind={:?}, prompt={:?})",
                        sel.kind, sel.prompt
                    );
                }
            }
        };
        if let Err(e) = self.execute_action(action_id) {
            panic!("auto_resolve: execute_action failed: {:?}", e);
        }
    }
    panic!(
        "auto_resolve: exceeded {} iterations without draining selections — \
         likely a callback that re-installs without progress",
        MAX_ITERATIONS
    );
}
```

- [ ] **Step 4: Run the tests — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl auto_resolve
```

Expected: PASS for both tests.

- [ ] **Step 5: Run the full engine suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: add auto_resolve to drain chained selections"
```

---

### Task 6: `execute_branch`

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` (one method)
- Modify: `code/digimon-engine/tests/debug_runner_dsl.rs` (one test)

**Why:** Convenience wrapper for `EffectChoice` selections — converts a label index into the underlying action ID, so test code reads `r.execute_branch(0)` instead of fishing the action ID out of `effect_choices`.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
#[test]
fn execute_branch_submits_label_indexed_action_id() {
    use digimon_engine::card_source::CardHandle;
    use digimon_engine::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};
    use std::sync::{Arc, Mutex};

    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    let card_handle = r.game.players[0].hand[0].handle();
    let chosen = Arc::new(Mutex::new(None::<u16>));
    let chosen_cb = chosen.clone();

    r.game.pending_selection = Some(PendingSelection {
        kind: SelectionKind::EffectChoice,
        selecting_player: 0,
        previous_phase: r.game.current_phase,
        valid_action_ids: vec![100, 101, 102],
        is_optional: false,
        prompt: "Three branches".to_string(),
        effect_choices: Some(vec![
            EffectChoiceEntry { label: "A".to_string(), action_id: 100 },
            EffectChoiceEntry { label: "B".to_string(), action_id: 101 },
            EffectChoiceEntry { label: "C".to_string(), action_id: 102 },
        ]),
        source_card: card_handle,
        source_permanent: None,
        callback: Box::new(move |_, a| {
            *chosen_cb.lock().unwrap() = Some(a);
        }),
        on_decline: None,
    });

    r.execute_branch(2).expect("branch 2 must submit action 102");
    assert_eq!(*chosen.lock().unwrap(), Some(102));
}

#[test]
fn execute_branch_errors_when_no_effect_choice_selection() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();

    let err = r.execute_branch(0);
    assert!(err.is_err(), "errors when no selection installed");
}
```

- [ ] **Step 2: Run the failing tests**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl execute_branch
```

Expected: compile error.

- [ ] **Step 3: Implement `execute_branch`**

In `code/digimon-engine/src/debug_runner.rs`, after `auto_resolve`:

```rust
/// Submit the action ID matching `effect_choices[label_index]`. Only
/// valid for `SelectionKind::EffectChoice` prompts.
pub fn execute_branch(
    &mut self,
    label_index: usize,
) -> Result<(), crate::selection::SelectionError> {
    let action_id = match self.game.pending_selection.as_ref() {
        None => return Err(crate::selection::SelectionError::NoPendingSelection),
        Some(sel) => {
            if !matches!(sel.kind, crate::selection::SelectionKind::EffectChoice) {
                return Err(crate::selection::SelectionError::NoPendingSelection);
            }
            let entries = sel
                .effect_choices
                .as_ref()
                .ok_or(crate::selection::SelectionError::NoPendingSelection)?;
            entries
                .get(label_index)
                .map(|e| e.action_id)
                .ok_or(crate::selection::SelectionError::NoPendingSelection)?
        }
    };
    self.execute_action(action_id)
}
```

- [ ] **Step 4: Run the tests — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl execute_branch
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: add execute_branch convenience for EffectChoice"
```

---

### Task 7: `compiled_card` storage and accessors

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` — add `compiled_cards` field on both `DebugRunner` and `DebugRunnerBuilder`, thread through `build_inner`, add accessors.
- Modify: `code/digimon-engine/tests/debug_runner_dsl.rs` — test that loading a known card surfaces its `CompiledCard`.

**Why:** Tasks 8 and 9 need a place to store compiled-card pointers so structural assertions (clause count, kinds, OPT flags) can run against them.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
use digimon_dsl::compiled::{CompiledCard, CompiledClause};
use std::sync::Arc;

#[test]
fn compiled_card_returns_registered_compiled_pointer() {
    use digimon_dsl::compile::compile;
    use digimon_dsl::CardSpec;

    // Minimal inline spec — we'll go through from_dsl_yaml in Task 8;
    // here we test the storage layer in isolation.
    let yaml = r#"
card: DSL-RUN-001
name: RunnerTestOne
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - gain_memory: 1
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = compile(&spec).expect("compiles");

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("DSL-RUN-001", "RunnerTestOne"))
        .build();
    runner.insert_compiled_card("DSL-RUN-001", Arc::new(compiled));

    let c = runner.compiled_card("DSL-RUN-001");
    assert_eq!(c.card, "DSL-RUN-001");
    assert_eq!(c.effects.len(), 1);

    let clause = runner.dsl_clause("DSL-RUN-001", 0);
    matches!(clause, CompiledClause::Triggered(_));
}
```

- [ ] **Step 2: Run the failing test**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl compiled_card
```

Expected: compile error — `compiled_card` and `insert_compiled_card` don't exist.

- [ ] **Step 3: Add the field and accessors**

In `code/digimon-engine/src/debug_runner.rs`:

a) Update the imports at the top of the file to include the new types:

```rust
use std::sync::Arc;
use digimon_dsl::compiled::{CompiledCard, CompiledClause};
```

b) Add the field to `DebugRunner` (the existing struct definition near the top — find `pub struct DebugRunner { pub game: Game, }` and replace it):

```rust
pub struct DebugRunner {
    pub game: Game,
    /// Compiled DSL specs for cards loaded via `dsl_card` / `from_dsl_yaml`.
    /// Used by `compiled_card` / `dsl_clause` for structural assertions.
    compiled_cards: HashMap<String, Arc<CompiledCard>>,
}
```

c) Update `DebugRunner::wrap` (existing) to initialize the new field:

```rust
pub fn wrap(game: Game) -> Self {
    Self {
        game,
        compiled_cards: HashMap::new(),
    }
}
```

d) Add the accessors inside `impl DebugRunner`, after `execute_branch`:

```rust
// ─── Compiled card storage and accessors ──────────────────────────

/// Register a compiled DSL spec under `card_id`. Used by
/// `dsl_card` / `from_dsl_yaml` builder verbs and tests that want to
/// install a compiled card without going through the loader.
pub fn insert_compiled_card(&mut self, card_id: &str, compiled: Arc<CompiledCard>) {
    self.compiled_cards.insert(card_id.to_string(), compiled);
}

/// Borrow the compiled spec for a registered DSL card. Panics if the
/// card was not loaded via `dsl_card` / `from_dsl_yaml`.
pub fn compiled_card(&self, card_id: &str) -> &CompiledCard {
    self.compiled_cards.get(card_id).unwrap_or_else(|| {
        panic!(
            "compiled_card({}): card was not registered via dsl_card / from_dsl_yaml",
            card_id
        )
    })
}

/// Borrow a clause from a compiled card by index. Panics on missing card or out-of-range index.
pub fn dsl_clause(&self, card_id: &str, idx: usize) -> &CompiledClause {
    let c = self.compiled_card(card_id);
    c.effects.get(idx).unwrap_or_else(|| {
        panic!(
            "dsl_clause({}, {}): only {} clauses defined",
            card_id,
            idx,
            c.effects.len()
        )
    })
}
```

e) Add a matching `compiled_cards` field on `DebugRunnerBuilder` (find the existing `pub struct DebugRunnerBuilder { … }` and add the field at the bottom):

```rust
pub struct DebugRunnerBuilder {
    card_data: HashMap<String, CardData>,
    hands: HashMap<PlayerId, Vec<String>>,
    decks: HashMap<PlayerId, Vec<String>>,
    securities: HashMap<PlayerId, Vec<String>>,
    digitamas: HashMap<PlayerId, Vec<String>>,
    rules: Rules,
    registry: Option<CardEffectRegistry>,
    player_count: Option<u8>,
    initial_memory: Option<i16>,
    /// Compiled DSL specs accumulated by `dsl_card` / `from_dsl_yaml`.
    /// Threaded through to the resulting `DebugRunner` in `build_inner`.
    compiled_cards: HashMap<String, Arc<CompiledCard>>,
}
```

f) Update the `Default` impl to initialize the new field:

```rust
impl Default for DebugRunnerBuilder {
    fn default() -> Self {
        Self {
            card_data: HashMap::new(),
            hands: HashMap::new(),
            decks: HashMap::new(),
            securities: HashMap::new(),
            digitamas: HashMap::new(),
            rules: Rules::standard(),
            registry: None,
            player_count: None,
            initial_memory: None,
            compiled_cards: HashMap::new(),
        }
    }
}
```

g) Update `build_inner` to thread the field into `DebugRunner` (find the line `DebugRunner { game }` and replace with):

```rust
DebugRunner {
    game,
    compiled_cards: std::mem::take(&mut self.compiled_cards),
}
```

- [ ] **Step 4: Run the test — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl compiled_card
```

Expected: PASS.

- [ ] **Step 5: Run the full engine suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: store CompiledCard refs and add compiled_card / dsl_clause accessors"
```

---

### Task 8: `from_dsl_yaml` builder verb + `CompiledCard → CardData` adapter

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` — add `card_data_from_compiled` private helper + `from_dsl_yaml` builder verb.
- Modify: `code/digimon-engine/tests/debug_runner_dsl.rs` — integration test that loads a card from inline YAML and plays it.

**Why:** Inline-YAML loading is the path used by DSL infra tests and "tweaked-spec" tests. Implements before `dsl_card` (Task 9) because `dsl_card` is the same code path with the lookup swapped for `CardRegistry::lookup`.

`CompiledCard` lacks several fields that `CardData` needs verbatim (effect_text strings, parsed `Keyword` enum). For test purposes the adapter fills these with empty defaults — DSL cards drive their behavior through compiled clauses, not through `CardData::keywords`.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
#[test]
fn from_dsl_yaml_registers_card_and_runs_on_play() {
    let yaml = r#"
card: DSL-RUN-002
name: RunnerTestTwo
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - gain_memory: 1
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("yaml compiles")
        .hand(0, &["DSL-RUN-002"])
        .memory(5)
        .start();

    let mem_before = runner.memory();
    runner.play(0, 0);
    runner.auto_resolve();

    // -3 cost + 1 from gain_memory step = net -2.
    assert_eq!(runner.memory(), mem_before - 3 + 1);

    // CompiledCard accessor works.
    let compiled = runner.compiled_card("DSL-RUN-002");
    assert_eq!(compiled.card, "DSL-RUN-002");
    assert_eq!(compiled.effects.len(), 1);
}
```

- [ ] **Step 2: Run the failing test**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl from_dsl_yaml
```

Expected: compile error — `from_dsl_yaml` does not exist.

- [ ] **Step 3: Add the `card_data_from_compiled` helper**

Add to the bottom of `code/digimon-engine/src/debug_runner.rs`, after the existing `make_test_egg` function:

```rust
/// Adapter: derive a `CardData` from a `CompiledCard` for runner setup.
///
/// The DSL preserves the structured fields `CardData` needs (kind, level,
/// dp, cost, colors, traits) but does not carry the raw effect-text
/// strings or parsed `Keyword` enum — DSL cards drive behavior through
/// compiled clauses. The text fields are left empty; behavioral tests
/// are unaffected.
fn card_data_from_compiled(c: &CompiledCard) -> CardData {
    use digimon_dsl::compiled::{CompiledCardKind, CompiledColor};

    let card_kind = match c.kind {
        CompiledCardKind::Digimon => CardKind::Digimon,
        CompiledCardKind::Tamer => CardKind::Tamer,
        CompiledCardKind::Option => CardKind::Option,
        CompiledCardKind::DigiEgg => CardKind::DigiEgg,
        CompiledCardKind::Token => CardKind::Token,
    };

    let colors = c
        .color
        .iter()
        .map(|cc| match cc {
            CompiledColor::Red => crate::enums::CardColor::Red,
            CompiledColor::Blue => crate::enums::CardColor::Blue,
            CompiledColor::Yellow => crate::enums::CardColor::Yellow,
            CompiledColor::Green => crate::enums::CardColor::Green,
            CompiledColor::Black => crate::enums::CardColor::Black,
            CompiledColor::Purple => crate::enums::CardColor::Purple,
            CompiledColor::White => crate::enums::CardColor::White,
        })
        .collect();

    CardData {
        card_id: c.card.clone(),
        card_name: c.name.clone(),
        card_kind,
        level: c.level,
        dp: c.dp,
        play_cost: c.cost.unwrap_or(0).max(0) as u16,
        colors,
        traits: c.traits.clone(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: c.card.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}
```

- [ ] **Step 4: Add the `from_dsl_yaml` builder verb**

In `impl DebugRunnerBuilder`, after the existing `add_card`:

```rust
/// Compile and register a DSL card from inline YAML. The card's spec
/// is parsed as a `digimon_dsl::CardSpec`, compiled, wrapped in a
/// `DslCardEffect`, and inserted into the effect registry. Synthetic
/// `CardData` is derived from the compiled spec so subsequent
/// `.hand(...)` / `.deck(...)` calls can reference the card by ID.
///
/// Returns `Err` with a serialized error string on parse / compile failure.
pub fn from_dsl_yaml(mut self, yaml: &str) -> Result<Self, String> {
    let spec: digimon_dsl::CardSpec =
        serde_yml::from_str(yaml).map_err(|e| format!("parse: {e}"))?;
    let compiled = digimon_dsl::compile::compile(&spec)
        .map_err(|errs| format!("compile: {errs:?}"))?;

    let card_id = compiled.card.clone();
    let card_data = card_data_from_compiled(&compiled);
    let compiled_arc = Arc::new(compiled);

    // Effect registry: install the DslCardEffect under the card_id.
    let mut registry = self.registry.take().unwrap_or_else(crate::cards::build_registry);
    let effect: Arc<dyn crate::effect::CardEffect> =
        Arc::new(crate::dsl_cards::DslCardEffect::new(compiled_arc.clone()));
    registry.insert(&card_id, effect);
    self.registry = Some(registry);

    self.card_data.insert(card_id.clone(), card_data);
    self.compiled_cards.insert(card_id, compiled_arc);
    Ok(self)
}
```

- [ ] **Step 5: Run the test — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl from_dsl_yaml
```

Expected: PASS.

- [ ] **Step 6: Run the full engine suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: add from_dsl_yaml builder verb + CardData adapter"
```

---

### Task 9: `dsl_card` builder verb (load by ID from embedded pack)

**Files:**
- Modify: `code/digimon-engine/src/debug_runner.rs` — add `embedded_registry` lazy singleton + `dsl_card` builder verb.
- Modify: `code/digimon-engine/tests/debug_runner_dsl.rs` — load a real card from the embedded pack and assert structural shape.

**Why:** This is the per-card-test default path: `runner.builder().dsl_card("BT15-003")` resolves through the embedded pack so tests follow the shipping spec.

The embedded pack is built from `code/digimon-engine/cards/_examples/` via `build.rs` and exposed by `dsl_registry::from_embedded()`. Existing `_examples` fixtures (BT13-007, BT17-015, ST2-13, BT22-084, etc.) are valid test targets.

The `dsl-yaml-loader` feature must be enabled for `register_dsl_cards` in `build_registry`. This builder verb does its own registration (not relying on `register_dsl_cards`), so it works regardless of feature flag. The pack itself is always built — `cards.pack` exists in `OUT_DIR` whether or not the feature is on.

- [ ] **Step 1: Add the failing test**

Append to `code/digimon-engine/tests/debug_runner_dsl.rs`:

```rust
#[test]
fn dsl_card_loads_real_card_from_embedded_pack() {
    use digimon_dsl::compiled::CompiledCardKind;

    // BT22-084 Nokia Shiramine is a Tamer in cards/_examples/.
    let runner = DebugRunner::builder()
        .dsl_card("BT22-084")
        .expect("BT22-084 must be in the embedded pack")
        .build();

    let compiled = runner.compiled_card("BT22-084");
    assert_eq!(compiled.card, "BT22-084");
    assert_eq!(compiled.kind, CompiledCardKind::Tamer);
}

#[test]
fn dsl_card_errors_on_unknown_card_id() {
    let result = DebugRunner::builder().dsl_card("NOPE-999");
    assert!(result.is_err(), "unknown card id must error");
}
```

- [ ] **Step 2: Run the failing tests**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl dsl_card
```

Expected: compile error.

- [ ] **Step 3: Add the embedded-registry singleton**

In `code/digimon-engine/src/debug_runner.rs`, near the top after the existing `use` statements:

```rust
use std::sync::OnceLock;

/// Lazy singleton of the embedded DSL pack registry. Shared across
/// every `dsl_card` call so the pack is deserialized exactly once per
/// process. Returns `Err` if the embedded blob fails to deserialize.
fn embedded_registry() -> Result<&'static digimon_dsl::CardRegistry, String> {
    static REG: OnceLock<Result<digimon_dsl::CardRegistry, String>> = OnceLock::new();
    REG.get_or_init(|| crate::dsl_registry::from_embedded())
        .as_ref()
        .map_err(|e| e.clone())
}
```

- [ ] **Step 4: Add the `dsl_card` builder verb**

In `impl DebugRunnerBuilder`, after `from_dsl_yaml`:

```rust
/// Load a card from the embedded DSL pack by ID, register its
/// `DslCardEffect` into the runner, and synthesize matching
/// `CardData`. Subsequent `.hand(...)` / `.deck(...)` calls can
/// reference the card by its ID.
///
/// Returns `Err` if the embedded pack failed to load, or if the
/// requested `card_id` is not present in the pack.
pub fn dsl_card(mut self, card_id: &str) -> Result<Self, String> {
    let reg = embedded_registry()?;
    let compiled = reg
        .lookup(card_id)
        .ok_or_else(|| format!("dsl_card({}): not in embedded pack", card_id))?;
    let card_data = card_data_from_compiled(compiled);
    let compiled_arc = Arc::new(compiled.clone());

    let mut registry = self.registry.take().unwrap_or_else(crate::cards::build_registry);
    let effect: Arc<dyn crate::effect::CardEffect> =
        Arc::new(crate::dsl_cards::DslCardEffect::new(compiled_arc.clone()));
    registry.insert(card_id, effect);
    self.registry = Some(registry);

    self.card_data.insert(card_id.to_string(), card_data);
    self.compiled_cards.insert(card_id.to_string(), compiled_arc);
    Ok(self)
}
```

- [ ] **Step 5: Run the tests — expect pass**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl dsl_card
```

Expected: PASS for both tests.

- [ ] **Step 6: Run the full engine suite**

```
cargo test --manifest-path code/digimon-engine/Cargo.toml
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/debug_runner.rs code/digimon-engine/tests/debug_runner_dsl.rs
git commit -m "runner: add dsl_card builder verb (load by ID from embedded pack)"
```

---

### Task 10: Strip `**(spec)**` markers from the test API doc

**Files:**
- Modify: `docs/RUST_DSL_TEST_API.md` — every `**(spec)**` row in §3 is now implemented; the markers should be removed so readers don't think the helpers are still pending.

**Why:** Self-consistency. Once Tasks 1–9 land, the doc's spec markers are misleading.

- [ ] **Step 1: Inspect the current markers**

```
grep -n "(spec)" docs/RUST_DSL_TEST_API.md
```

Expected output: 13 lines, all in §3.

- [ ] **Step 2: Remove the markers**

Use sed (or equivalent in your shell):

```bash
sed -i 's/ \*\*(spec)\*\*//g' docs/RUST_DSL_TEST_API.md
```

On Windows bash:

```bash
sed -i.bak 's/ \*\*(spec)\*\*//g' docs/RUST_DSL_TEST_API.md && rm docs/RUST_DSL_TEST_API.md.bak
```

- [ ] **Step 3: Verify the markers are gone**

```
grep -c "(spec)" docs/RUST_DSL_TEST_API.md
```

Expected output: `0`.

- [ ] **Step 4: Skim §3 for any explanatory sentences referencing "spec'd helpers"**

Read the §3 intro paragraph in `docs/RUST_DSL_TEST_API.md`:

> Tests drive the engine through `DebugRunner`. The DSL-aware additions to the runner are listed below. Helpers marked **(spec)** describe behavior the runner must implement; treat the signature as the contract.

Replace with:

> Tests drive the engine through `DebugRunner`. The DSL-aware additions to the runner are listed below.

Use the Edit tool with `old_string` matching the original sentence and `new_string` containing the replacement.

- [ ] **Step 5: Fix the `events_of_kind` row signature**

The doc's row for `events_of_kind` previously read:

> `runner.events_of_kind(kind)` → `Vec<&GameEvent>` | All events matching a discriminant (e.g. `OnDiscardSecurity`).

Replace with:

> `runner.events_of_kind(checkpoint, predicate)` → `Vec<&GameEvent>` | Events emitted since `checkpoint` for which `predicate(&GameEvent) -> bool` returns `true`. Use `matches!(e, GameEvent::OnDiscardSecurity { .. })` style predicates.

Use the Edit tool. The closure-based form is more flexible than a discriminant enum — the engine doesn't expose a discriminant type, and a predicate handles both single-variant filters and multi-variant combinations.

Also update the §5 Section 4 example in the doc — find the `discard_count = events.iter()` block and verify it uses `events_of_kind` correctly. The block already uses inline `.iter().filter()`, so no change needed.

- [ ] **Step 7: Commit**

```bash
git add docs/RUST_DSL_TEST_API.md
git commit -m "docs(dsl-test-api): strip (spec) markers now that runner helpers exist"
```

---

## Acceptance criteria

After Task 10 lands:

1. `cargo test --manifest-path code/digimon-engine/Cargo.toml --test debug_runner_dsl` runs all 10+ integration tests green.
2. `cargo test --manifest-path code/digimon-engine/Cargo.toml` (full engine suite) stays green.
3. Every helper listed in `docs/RUST_DSL_TEST_API.md` §3 has a corresponding method on `DebugRunner` or `DebugRunnerBuilder`.
4. No `**(spec)**` markers remain in `docs/RUST_DSL_TEST_API.md`.
5. The first per-card behavioral test (e.g. `tests/cards_behavioral/bt15/bt15_003.rs`, deferred to a later plan) can compile against this surface without further runner changes.

## Out of scope

- Per-card behavioral tests under `tests/cards_behavioral/<set>/<card_id>.rs`. Those are authored by the forthcoming `/batch-implement-cards-rust-dsl` skill against the helpers this plan delivers.
- The example card pool research task (`qa/dsl-test-pool.md`). That is a separate sub-agent dispatch.
- Mechanic-level test patterns (combat, keyword, replacement). Same — separate work.
- Production card YAMLs at `code/digimon-engine/cards/<set>/<card_id>.yaml`. The pack still loads from `cards/_examples/`; the move to `cards/<set>/` happens when production-card authoring begins.
