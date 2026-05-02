# PyO3 PvP Bindings — to_ui_json, pending_selection, events, recording

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expand `digimon-engine-py` so a Python caller can drive a full human-vs-human game — `g.to_ui_json()`, `g.get_pending_selection()`, `g.get_events_since_last_step()`, `g.get_recording()` all return non-stub data parity-tested against the Python engine.

**Architecture:** Add four new surfaces on the Rust side (a serializable view of `PendingSelection`; a `GameEvent` enum + per-step accumulator on `Game`; a `to_ui_json` function that builds a dict matching Python's `serialization.to_ui_json`; a `GameRecorder` that matches Python's recording dict shape). Then wire each one through PyO3 as a `PyDict` / `PyList` builder. Tests land in two places: Rust-side behavioral tests under `digimon-engine/tests/` using `DebugRunner`, and Python-side parity tests in `tests/engine/test_rust_backend_parity.py` that build a game on both engines and compare shapes.

**Tech Stack:** Rust (`digimon-engine`, `digimon-engine-py` w/ PyO3 0.22), Python 3.11, maturin, pytest, serde 1.

---

## File Structure

### Rust — new files

- `digimon-engine/src/events.rs` — `GameEvent` enum, `EventSeq` counter, `Game::emit()` helper.
- `digimon-engine/src/serialization.rs` — `to_ui_json(&Game) -> UiState` builders returning nested `serde_json::Value` trees that match Python's `serialization.to_ui_json` keys.
- `digimon-engine/src/runners/recorder.rs` — `GameRecorder`, `InitialState`, `PlayerInitialState`, `RecordedAction` mirroring Python's `digimon_gym/engine/recording.py` dict shape.
- `digimon-engine/tests/ffi_parity/main.rs` — new test binary declaring sub-modules.
- `digimon-engine/tests/ffi_parity/selection_view.rs` — pending-selection-view tests.
- `digimon-engine/tests/ffi_parity/events.rs` — GameEvent accumulation + emission tests.
- `digimon-engine/tests/ffi_parity/ui_json.rs` — to_ui_json shape tests.
- `digimon-engine/tests/ffi_parity/recorder.rs` — recording shape tests.

### Rust — modified

- `digimon-engine/src/lib.rs` — declare new modules, re-export `GameEvent`, `PendingSelectionView`, `to_ui_json`, `GameRecorder`.
- `digimon-engine/src/selection.rs` — add `PendingSelectionView` struct and `PendingSelection::view()`.
- `digimon-engine/src/game.rs` — add `events: Vec<GameEvent>` + `event_seq: u64` fields and emit-helpers; hook emits into `gain_memory`, `set_memory`, `pay_memory`, `declare_winner`, `handle_deckout`. Add `drain_events()`.
- `digimon-engine/src/action/decode.rs` — emit `Play` when a play action resolves, `PhaseChange` when `current_phase` transitions, `TurnStart` when `turn_count` bumps.
- `digimon-engine/src/runners/headless.rs` — embed `Option<GameRecorder>`, wire `record_action` around `step`, expose `recorder()` + `drain_events()` passthroughs.
- `digimon-engine/Cargo.toml` — add `[[test]] ffi_parity` entry.

### PyO3 layer — modified

- `digimon-engine-py/src/lib.rs` — add `to_ui_json()`, `get_pending_selection()`, `get_events_since_last_step()`, rewrite `get_recording()`. Player-ID translation at each boundary.

### Python tests — modified

- `tests/engine/test_rust_backend_parity.py` — add shape-parity assertions for the four new methods.

---

## Task 1: Add `PendingSelectionView` on the Rust side

**Files:**
- Modify: `digimon-engine/src/selection.rs` (add view struct + method after line 102)
- Create: `digimon-engine/tests/ffi_parity/main.rs`
- Create: `digimon-engine/tests/ffi_parity/selection_view.rs`
- Modify: `digimon-engine/Cargo.toml` (add test binary entry)

**Rationale:** This is the smallest self-contained surface. `PendingSelection` contains two non-`Clone` / non-`Send`-able callback fields (`Box<dyn FnOnce>`) that cannot cross the FFI boundary. Introduce a pure-data view that the PyO3 layer can convert to a `PyDict` without touching the callbacks. Rust-side tests confirm the view round-trips the serializable fields.

- [ ] **Step 1.1: Add `[[test]] ffi_parity` entry to Cargo.toml**

Open `digimon-engine/Cargo.toml` and add after the `deck_tools` entry (after line 59):

```toml
[[test]]
name = "ffi_parity"
path = "tests/ffi_parity/main.rs"
```

- [ ] **Step 1.2: Create test binary entry point**

Create `digimon-engine/tests/ffi_parity/main.rs`:

```rust
//! Tests for the FFI-facing surfaces (PendingSelectionView, GameEvent,
//! to_ui_json, GameRecorder). Exercised by the PyO3 crate via
//! `digimon-engine-py`, but the Rust half is validated here so failures
//! don't have to travel through Python to be diagnosed.

mod selection_view;
```

(Further mods — `events`, `ui_json`, `recorder` — are added in later tasks.)

- [ ] **Step 1.3: Write the failing selection-view test**

Create `digimon-engine/tests/ffi_parity/selection_view.rs`:

```rust
//! PendingSelectionView exposes only the serializable fields of a
//! `PendingSelection`. Round-trip test via TEST-010: play it, grab the
//! installed selection, take its view, assert every field matches.

use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::SelectionKind;

fn runner_with_two_opponents() -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-010", "PilotDelete"))
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["TEST-010"])
        .memory(3)
        .start();
    r.place_on_field(1, "ALLY", Some(0));
    r.place_on_field(1, "ALLY", Some(0));
    r
}

#[test]
fn pending_selection_view_mirrors_serializable_fields() {
    let mut r = runner_with_two_opponents();
    r.play(0, 0);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("TEST-010 installs a pending selection");
    let view = sel.view();

    assert_eq!(view.kind, SelectionKind::OppField);
    assert_eq!(view.selecting_player, 0);
    assert_eq!(view.previous_phase, GamePhase::Main);
    assert_eq!(
        view.valid_action_ids,
        vec![encode_attack(0, 0), encode_attack(0, 1)],
    );
    assert!(view.is_optional);
    assert!(view.prompt.len() > 0, "prompt must not be empty");
    assert!(view.effect_choices.is_none());
}

#[test]
fn pending_selection_view_kind_as_str_round_trips() {
    let mut r = runner_with_two_opponents();
    r.play(0, 0);
    let sel = r.game.pending_selection.as_ref().unwrap();
    let view = sel.view();
    // Stable string form used by the PyO3 layer and by the Python-side
    // state filter for UI rendering. Variants use their `Debug` spelling.
    assert_eq!(view.kind_str(), "OppField");
    assert_eq!(view.previous_phase_str(), "Main");
}
```

- [ ] **Step 1.4: Run tests — expect compile failure**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity`
Expected: FAIL — `no method named 'view' found for struct 'PendingSelection'`, `no method named 'kind_str'`, `no method named 'previous_phase_str'`.

- [ ] **Step 1.5: Add `PendingSelectionView` and implement `PendingSelection::view()`**

In `digimon-engine/src/selection.rs`, append after the `impl std::fmt::Debug for PendingSelection` block (around line 118):

```rust
/// Pure-data subset of `PendingSelection` — excludes the non-serializable
/// `callback` / `on_decline` closures so this type is `Clone` + `Send` and
/// can cross the FFI boundary. Matches the Python `pendingSelection` dict
/// shape emitted by `digimon_gym/engine/game/serialization.py:338`.
#[derive(Debug, Clone)]
pub struct PendingSelectionView {
    pub kind: SelectionKind,
    pub selecting_player: PlayerId,
    pub previous_phase: GamePhase,
    pub valid_action_ids: Vec<u16>,
    pub is_optional: bool,
    pub prompt: String,
    pub effect_choices: Option<Vec<EffectChoiceEntry>>,
    pub source_card: CardHandle,
    pub source_permanent: Option<PermanentHandle>,
}

impl PendingSelectionView {
    /// Stable string form of `kind`, used for JSON/PyDict output. Debug
    /// formatting is deliberate — every variant stringifies to its Rust
    /// identifier so Python consumers can pattern-match without a shared
    /// schema.
    pub fn kind_str(&self) -> String {
        format!("{:?}", self.kind)
    }

    /// Stable string form of `previous_phase`.
    pub fn previous_phase_str(&self) -> String {
        format!("{:?}", self.previous_phase)
    }
}

impl PendingSelection {
    /// Snapshot of the serializable fields for FFI / UI consumers.
    pub fn view(&self) -> PendingSelectionView {
        PendingSelectionView {
            kind: self.kind,
            selecting_player: self.selecting_player,
            previous_phase: self.previous_phase,
            valid_action_ids: self.valid_action_ids.clone(),
            is_optional: self.is_optional,
            prompt: self.prompt.clone(),
            effect_choices: self.effect_choices.clone(),
            source_card: self.source_card,
            source_permanent: self.source_permanent,
        }
    }
}
```

- [ ] **Step 1.6: Re-export from `lib.rs`**

Open `digimon-engine/src/lib.rs` and ensure `selection::PendingSelectionView` is re-exported alongside the existing `selection::*` exports (add a line to the existing `pub use crate::selection::...` block, or add:

```rust
pub use crate::selection::PendingSelectionView;
```

if no such re-export exists yet).

- [ ] **Step 1.7: Run tests — expect pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity`
Expected: PASS (both tests green).

- [ ] **Step 1.8: Commit**

```bash
git add digimon-engine/src/selection.rs digimon-engine/src/lib.rs \
        digimon-engine/tests/ffi_parity/main.rs \
        digimon-engine/tests/ffi_parity/selection_view.rs \
        digimon-engine/Cargo.toml
git commit -m "feat(engine): add PendingSelectionView for FFI consumers"
```

---

## Task 2: Add `GameEvent` enum and accumulator on `Game`

**Files:**
- Create: `digimon-engine/src/events.rs`
- Modify: `digimon-engine/src/lib.rs` (declare module)
- Modify: `digimon-engine/src/game.rs` (add fields + emit helpers + drain)
- Create: `digimon-engine/tests/ffi_parity/events.rs`
- Modify: `digimon-engine/tests/ffi_parity/main.rs` (add mod events)

**Rationale:** Python emits `GameEvent` dicts via `digimon_gym/engine/events.py` (fields: `type`, `seq`, `player`, `source_card_id`, `source_slot`, `target_card_id`, `target_slot`, `meta`). Rust today has only a text-based `GameLogger`. Add a structured enum with the same field shape, plus a per-`Game` accumulator that the step wrapper drains after each action. Emission is wired in this task for the cases reachable through test-card flows: `MemoryChange`, `TurnStart`, `PhaseChange`, `GameOver`, `Play`. The enum defines all 10 variants from the spec so later card-migration work can emit the rest without schema churn.

- [ ] **Step 2.1: Write the failing test**

Create `digimon-engine/tests/ffi_parity/events.rs`:

```rust
//! GameEvent accumulator is drained per step. TEST-001 ("On Play: Gain 1
//! memory") should emit at minimum a Play event and a MemoryChange event
//! when played.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;

#[test]
fn memory_change_event_emitted_on_gain_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    r.game.drain_events(); // clear any startup events

    r.game.gain_memory(2);

    let events = r.game.drain_events();
    assert!(
        events.iter().any(|e| matches!(e, GameEvent::MemoryChange { delta: 2, .. })),
        "gain_memory(2) should emit MemoryChange {{ delta: 2 }}; got {:?}",
        events,
    );
}

#[test]
fn play_and_memory_events_emitted_on_play() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    r.game.drain_events(); // clear startup events
    r.play(0, 0);

    let events = r.game.drain_events();
    let has_play = events.iter().any(|e| matches!(e, GameEvent::Play { .. }));
    let has_memory = events
        .iter()
        .any(|e| matches!(e, GameEvent::MemoryChange { .. }));
    assert!(has_play, "expected a Play event after r.play(); got {:?}", events);
    assert!(
        has_memory,
        "expected MemoryChange events (pay cost + OnPlay gain); got {:?}",
        events
    );
}

#[test]
fn events_have_monotonic_seq_numbers() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    r.game.drain_events();

    r.game.gain_memory(1);
    r.game.gain_memory(1);
    r.game.gain_memory(1);

    let events = r.game.drain_events();
    let seqs: Vec<u64> = events.iter().map(|e| e.seq()).collect();
    let sorted: Vec<u64> = {
        let mut s = seqs.clone();
        s.sort_unstable();
        s
    };
    assert_eq!(seqs, sorted, "seq must be monotonic; got {:?}", seqs);
    for w in seqs.windows(2) {
        assert!(w[1] > w[0], "seq must be strictly increasing");
    }
}

#[test]
fn drain_events_clears_buffer() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    let _ = r.game.drain_events();
    r.game.gain_memory(1);
    let first = r.game.drain_events();
    assert_eq!(first.len(), 1);
    let second = r.game.drain_events();
    assert!(
        second.is_empty(),
        "drain_events must clear the buffer; got {:?}",
        second
    );
}
```

- [ ] **Step 2.2: Add `mod events` to the test binary entry**

Edit `digimon-engine/tests/ffi_parity/main.rs`:

```rust
mod selection_view;
mod events;
```

- [ ] **Step 2.3: Run tests — expect compile failure**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity`
Expected: FAIL — `no module 'events' in crate 'digimon_engine'`, `no method named 'drain_events'`.

- [ ] **Step 2.4: Create the `events` module**

Create `digimon-engine/src/events.rs`:

```rust
//! Structured game events emitted during action resolution. Mirrors
//! Python's `digimon_gym/engine/events.py::GameEvent` — a tagged enum
//! consumed by UI animation and replay layers.
//!
//! Emission coverage is currently partial: `MemoryChange`, `TurnStart`,
//! `PhaseChange`, `GameOver`, and `Play` are wired in by this module's
//! initial landing. `Digivolve`, `Attack`, `Trash`, `Mill`, and
//! `SecurityReveal` variants exist on the enum and will be emitted as
//! card-migration work wires the corresponding game paths.
//!
//! Every event carries a monotonically increasing `seq` allocated by
//! `Game::next_event_seq`. Consumers drain the buffer via
//! `Game::drain_events` (the runner does this around each `step`).

use crate::enums::{GamePhase, PlayerId};

/// Tagged event payload. `#[non_exhaustive]` on each variant would force
/// Python consumers to pattern-match defensively forever; we prefer the
/// Rust enum itself `#[non_exhaustive]` instead so new variants can be
/// added without breaking downstream matches.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum GameEvent {
    /// `memory` just changed by `delta`. Emitted by `Game::gain_memory`,
    /// `pay_memory`, and `set_memory`. `delta` is signed and may be zero
    /// (set_memory can be a no-op; callers can filter if they care).
    MemoryChange {
        seq: u64,
        player: PlayerId,
        delta: i16,
        total: i16,
    },

    /// A new turn has started. Emitted after `turn_count` bumps.
    TurnStart {
        seq: u64,
        player: PlayerId,
        turn_count: u16,
    },

    /// The current phase has changed to `phase`.
    PhaseChange {
        seq: u64,
        player: PlayerId,
        phase: GamePhase,
    },

    /// A card entered the battle area from hand.
    Play {
        seq: u64,
        player: PlayerId,
        card_id: String,
        field_index: u8,
    },

    /// `player` digivolved `top` onto a permanent at `field_index`.
    /// (Variant defined for future wiring — not emitted yet.)
    Digivolve {
        seq: u64,
        player: PlayerId,
        top_card_id: String,
        field_index: u8,
        from_stack_top: String,
    },

    /// A Digimon declared an attack.
    /// (Variant defined for future wiring — not emitted yet.)
    Attack {
        seq: u64,
        player: PlayerId,
        attacker_field_index: u8,
        target_field_index: Option<u8>,
        target_player: Option<PlayerId>,
    },

    /// A card was moved to trash from some zone.
    /// (Variant defined for future wiring — not emitted yet.)
    Trash {
        seq: u64,
        player: PlayerId,
        card_id: String,
    },

    /// A card was milled (deck→trash from the top of the deck).
    /// (Variant defined for future wiring — not emitted yet.)
    Mill {
        seq: u64,
        player: PlayerId,
        card_id: String,
    },

    /// A security card was revealed during a security check.
    /// (Variant defined for future wiring — not emitted yet.)
    SecurityReveal {
        seq: u64,
        defender: PlayerId,
        card_id: String,
    },

    /// The game ended. `winner` is `None` on a draw.
    GameOver {
        seq: u64,
        winner: Option<PlayerId>,
    },
}

impl GameEvent {
    /// Monotonic sequence number allocated at emission time.
    pub fn seq(&self) -> u64 {
        match self {
            GameEvent::MemoryChange { seq, .. }
            | GameEvent::TurnStart { seq, .. }
            | GameEvent::PhaseChange { seq, .. }
            | GameEvent::Play { seq, .. }
            | GameEvent::Digivolve { seq, .. }
            | GameEvent::Attack { seq, .. }
            | GameEvent::Trash { seq, .. }
            | GameEvent::Mill { seq, .. }
            | GameEvent::SecurityReveal { seq, .. }
            | GameEvent::GameOver { seq, .. } => *seq,
        }
    }

    /// Stable string type name. Matches Python `GameEvent.type`.
    pub fn type_str(&self) -> &'static str {
        match self {
            GameEvent::MemoryChange { .. } => "MemoryChange",
            GameEvent::TurnStart { .. } => "TurnStart",
            GameEvent::PhaseChange { .. } => "PhaseChange",
            GameEvent::Play { .. } => "Play",
            GameEvent::Digivolve { .. } => "Digivolve",
            GameEvent::Attack { .. } => "Attack",
            GameEvent::Trash { .. } => "Trash",
            GameEvent::Mill { .. } => "Mill",
            GameEvent::SecurityReveal { .. } => "SecurityReveal",
            GameEvent::GameOver { .. } => "GameOver",
        }
    }
}
```

- [ ] **Step 2.5: Declare the module in `lib.rs`**

Edit `digimon-engine/src/lib.rs` and add `pub mod events;` next to the other `pub mod` lines. Add `pub use crate::events::GameEvent;` alongside the other re-exports.

- [ ] **Step 2.6: Add event fields and helpers to `Game`**

In `digimon-engine/src/game.rs`, in the `Game` struct definition (lines 59–128), add two new fields at the end (just before `pub logger`):

```rust
    /// Event buffer drained per `step` by the runner. See
    /// `src/events.rs` for the event taxonomy.
    pub events: Vec<crate::events::GameEvent>,
    /// Monotonic counter for `GameEvent::seq`. Never decreases across the
    /// lifetime of a `Game`.
    pub event_seq: u64,
```

In `Game::new` (around line 134 — wherever the struct literal initializes fields), initialize both:

```rust
            events: Vec::new(),
            event_seq: 0,
```

Add these helper methods to `impl Game` (place near the memory helpers around line 430):

```rust
    /// Allocate the next monotonic event sequence number.
    pub fn next_event_seq(&mut self) -> u64 {
        let s = self.event_seq;
        self.event_seq += 1;
        s
    }

    /// Drain accumulated events, returning them in emission order. The
    /// `HeadlessRunner::step` wrapper calls this after each action so the
    /// PyO3 layer can expose a per-step event list.
    pub fn drain_events(&mut self) -> Vec<crate::events::GameEvent> {
        std::mem::take(&mut self.events)
    }
```

Wire `MemoryChange` emission — edit the existing `pay_memory`, `gain_memory`, `set_memory` methods:

```rust
    pub fn pay_memory(&mut self, cost: u16) -> bool {
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

    pub fn gain_memory(&mut self, amount: i16) {
        let before = self.memory;
        self.memory = (self.memory + amount).min(self.rules.memory_range.1);
        let delta = self.memory - before;
        let seq = self.next_event_seq();
        let player = self.turn_player();
        self.events.push(crate::events::GameEvent::MemoryChange {
            seq,
            player,
            delta,
            total: self.memory,
        });
    }

    pub fn set_memory(&mut self, value: i16) {
        let before = self.memory;
        self.memory = value.clamp(self.rules.memory_range.0, self.rules.memory_range.1);
        let delta = self.memory - before;
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

Wire `GameOver` emission — edit `declare_winner`, `handle_deckout`, and `eliminate_player` (the three paths that set `self.game_over = true;`). After each `self.game_over = true;`, push:

```rust
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::GameOver {
            seq,
            winner: self.winner,
        });
```

- [ ] **Step 2.7: Run tests — expect partial pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity`
Expected: `memory_change_event_emitted_on_gain_memory`, `events_have_monotonic_seq_numbers`, `drain_events_clears_buffer` PASS. `play_and_memory_events_emitted_on_play` FAIL (no Play event yet).

- [ ] **Step 2.8: Emit `Play` from the action decoder**

Open `digimon-engine/src/action/decode.rs` at line 32. The `decode_action` function dispatches actions; find the branch that handles playing a card from hand (look for calls to `player.play_from_hand` or similar). Immediately after a successful play (after the new permanent is installed and before OnPlay effects fire), push:

```rust
let seq = self.next_event_seq();
let field_index = <the index returned by play_from_hand>;
let card_id = self.players[pid as usize].battle_area[field_index].top_card().card_id().to_string();
self.events.push(crate::events::GameEvent::Play {
    seq,
    player: pid,
    card_id,
    field_index: field_index as u8,
});
```

If the exact call site needs adjustment (e.g., `play_from_hand` returns `Option<usize>`), match on the result and only emit on `Some(idx)`. Use whichever field name actually exposes the card_id string on `CardSource` (check `card_source.rs`).

- [ ] **Step 2.9: Run tests — expect all pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity`
Expected: all 4 tests PASS.

- [ ] **Step 2.10: Run full Rust suite for regressions**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: everything green. If the memory-emission changes break an existing behavioral test that counts events by side channel, update that test too.

- [ ] **Step 2.11: Commit**

```bash
git add digimon-engine/src/events.rs digimon-engine/src/lib.rs \
        digimon-engine/src/game.rs digimon-engine/src/action/decode.rs \
        digimon-engine/tests/ffi_parity/events.rs \
        digimon-engine/tests/ffi_parity/main.rs
git commit -m "feat(engine): structured GameEvent enum with per-step accumulator"
```

---

## Task 3: Add `to_ui_json` on the Rust side

**Files:**
- Create: `digimon-engine/src/serialization.rs`
- Modify: `digimon-engine/src/lib.rs` (declare module)
- Create: `digimon-engine/tests/ffi_parity/ui_json.rs`
- Modify: `digimon-engine/tests/ffi_parity/main.rs` (add mod)

**Rationale:** Python's `serialization.to_ui_json` (digimon_gym/engine/game/serialization.py:187) returns a nested dict the frontend + `state_filter.py` both consume. Rust must produce the same key set so a WebSocket server can call `rust_game.to_ui_json()` and pipe the result through `state_filter` without per-engine branches. The value tree is built in Rust as `serde_json::Value` so the PyO3 layer in Task 5 converts it mechanically. Player-ID translation (Rust 0/1 → Python 1/2) happens at this layer so the dict is already in Python's convention by the time PyO3 sees it.

**Design note:** Card-script-specific per-permanent fields (`mainEffectText`, `keywordBreakdown`, `dpBreakdown.sources`, etc.) are populated with empty / neutral defaults for the alpha (no card scripts are migrated yet). The shape matches so `state_filter.py` and the frontend don't crash; richness catches up when card migration lands.

- [ ] **Step 3.1: Write the failing shape test**

Create `digimon-engine/tests/ffi_parity/ui_json.rs`:

```rust
//! Shape parity: Rust `to_ui_json` must produce every top-level key the
//! Python side produces, plus the correct player-ID convention (1/2).

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::serialization::to_ui_json;

#[test]
fn to_ui_json_has_all_top_level_keys() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .add_card(make_test_card("ST1-01", "Egg"))
        .add_card(make_test_card("ST1-03", "Filler"))
        .hand(0, &["TEST-001"])
        .start();
    let value = to_ui_json(&r.game);
    let obj = value.as_object().expect("root is an object");
    for key in [
        "turnCount",
        "currentPhase",
        "currentPlayer",
        "memoryGauge",
        "isGameOver",
        "winner",
        "player1",
        "player2",
        "revealedCards",
        "pendingSelection",
        "pendingAttack",
    ] {
        assert!(obj.contains_key(key), "missing top-level key {:?}", key);
    }
}

#[test]
fn player_ids_use_python_convention() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .start();
    let value = to_ui_json(&r.game);
    assert_eq!(value["player1"]["id"], serde_json::json!(1));
    assert_eq!(value["player2"]["id"], serde_json::json!(2));
    let cp = value["currentPlayer"].as_i64().unwrap();
    assert!(cp == 1 || cp == 2, "currentPlayer must be 1 or 2, got {}", cp);
}

#[test]
fn player_ui_data_has_full_key_set() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .start();
    let value = to_ui_json(&r.game);
    let p1 = value["player1"].as_object().expect("player1 is object");
    for key in [
        "id",
        "memory",
        "handCount",
        "handIds",
        "handCards",
        "securityCount",
        "securityIds",
        "securityFaceUp",
        "deckCount",
        "eggDeckCount",
        "battleAreaCount",
        "battleArea",
        "breedingArea",
        "trashIds",
    ] {
        assert!(p1.contains_key(key), "player1 missing {:?}", key);
    }
}

#[test]
fn pending_selection_serializes_when_installed() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-010", "PilotDelete"))
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["TEST-010"])
        .memory(3)
        .start();
    r.place_on_field(1, "ALLY", Some(0));
    r.play(0, 0);

    let value = to_ui_json(&r.game);
    let ps = value["pendingSelection"].as_object().expect("pendingSelection is object");
    assert_eq!(ps["selectingPlayer"], serde_json::json!(1));
    assert!(ps["validIndices"].is_array());
    assert_eq!(ps["isOptional"], serde_json::json!(true));
    assert!(ps["prompt"].is_string());
    assert_eq!(ps["kind"], serde_json::json!("OppField"));
}

#[test]
fn pending_selection_null_when_no_selection() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .start();
    let value = to_ui_json(&r.game);
    assert!(
        value["pendingSelection"].is_null(),
        "pendingSelection must be null without a prompt; got {}",
        value["pendingSelection"]
    );
}
```

- [ ] **Step 3.2: Add `mod ui_json` to the test binary**

Edit `digimon-engine/tests/ffi_parity/main.rs` to add `mod ui_json;`.

- [ ] **Step 3.3: Run tests — expect compile failure**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity ui_json`
Expected: FAIL — `no function 'to_ui_json' in module 'serialization'`.

- [ ] **Step 3.4: Create the serialization module**

Create `digimon-engine/src/serialization.rs`:

```rust
//! UI-state serialization — builds a `serde_json::Value` tree that
//! matches the dict shape emitted by Python's
//! `digimon_gym/engine/game/serialization.py::to_ui_json`.
//!
//! Consumed by the PyO3 layer (PyDict conversion) and, transitively,
//! by `state_filter.py` and the React frontend.
//!
//! Player IDs are translated to the Python 1/2 convention at this layer
//! so downstream consumers don't need to know about the Rust 0-indexed
//! internal convention.

use serde_json::{json, Map, Value};

use crate::card_data::CardData;
use crate::card_source::CardSource;
use crate::enums::GamePhase;
use crate::game::Game;
use crate::permanent::Permanent;
use crate::player::Player;

/// Translate a Rust `PlayerId` (0 / 1) into the Python 1 / 2 convention.
fn py_pid(rust_pid: u8) -> i64 {
    (rust_pid as i64) + 1
}

/// Stable string for a phase variant. `serialization.py` uses the Python
/// `GamePhase.value` (lowercase with underscores); here we use the Rust
/// `Debug` spelling (PascalCase) and document the convention for the
/// frontend. Consumers can normalize case if needed.
fn phase_str(p: GamePhase) -> String {
    format!("{:?}", p)
}

/// Build the player-level dict. Matches `player_ui_data()` in Python.
fn player_ui_data(player: &Player, data: &[CardData], game: &Game) -> Value {
    let find_data = |cs: &CardSource| -> Option<&CardData> {
        data.iter().find(|d| d.card_id == cs.card_id)
    };

    let hand_ids: Vec<&str> = player.hand.iter().map(|c| c.card_id.as_str()).collect();
    let hand_cards: Vec<Value> = player
        .hand
        .iter()
        .map(|cs| {
            let cd = find_data(cs);
            json!({
                "cardId": cs.card_id,
                "cardName": cd.map(|d| d.card_name.as_str()).unwrap_or(""),
                "playCost": cd.and_then(|d| d.play_cost).unwrap_or(0),
                "level": cd.and_then(|d| d.level),
                "dp": cd.and_then(|d| d.dp),
                "colors": cd.map(|d| colors_of(d)).unwrap_or_default(),
                "cardKind": cd.map(|d| kind_int(d)).unwrap_or(0),
                "evoCosts": cd.map(|d| evo_costs_of(d)).unwrap_or_default(),
            })
        })
        .collect();

    let security_ids: Vec<Value> = player
        .security
        .iter()
        .enumerate()
        .map(|(i, cs)| {
            if player.face_up_security.contains(&(i as u16)) {
                json!(cs.card_id)
            } else {
                Value::Null
            }
        })
        .collect();
    let security_face_up: Vec<bool> = (0..player.security.len())
        .map(|i| player.face_up_security.contains(&(i as u16)))
        .collect();

    let battle_area: Vec<Value> = player
        .battle_area
        .iter()
        .map(|p| perm_data(p, data, game))
        .collect();
    let breeding_area = player
        .breeding_area
        .as_ref()
        .map(|p| perm_data(p, data, game))
        .unwrap_or(Value::Null);

    let trash_ids: Vec<&str> = player.trash.iter().map(|c| c.card_id.as_str()).collect();

    json!({
        "id": py_pid(player.id),
        "memory": game.memory,
        "handCount": player.hand.len(),
        "handIds": hand_ids,
        "handCards": hand_cards,
        "securityCount": player.security.len(),
        "securityIds": security_ids,
        "securityFaceUp": security_face_up,
        "deckCount": player.deck.len(),
        "eggDeckCount": player.digitama_deck.len(),
        "battleAreaCount": player.battle_area.len(),
        "battleArea": battle_area,
        "breedingArea": breeding_area,
        "trashIds": trash_ids,
    })
}

fn colors_of(cd: &CardData) -> Vec<String> {
    cd.colors.iter().map(|c| format!("{:?}", c)).collect()
}

fn kind_int(cd: &CardData) -> i64 {
    // Match Python CardKind int values. 0=Digimon, 1=Tamer, 2=Option, 3=DigiEgg.
    use crate::enums::CardKind;
    match cd.card_kind {
        CardKind::Digimon => 0,
        CardKind::Tamer => 1,
        CardKind::Option => 2,
        CardKind::DigiEgg => 3,
    }
}

fn evo_costs_of(cd: &CardData) -> Vec<Value> {
    cd.evo_costs
        .iter()
        .map(|ec| json!({
            "color": format!("{:?}", ec.color),
            "level": ec.level,
            "cost": ec.cost,
        }))
        .collect()
}

/// Per-permanent dict. Card-script-specific fields (keyword breakdown,
/// dp breakdown, effect text) are populated with neutral defaults —
/// shape parity is the goal; richness arrives with card migration.
fn perm_data(perm: &Permanent, data: &[CardData], game: &Game) -> Value {
    let top = perm.top_card();
    let top_data = data.iter().find(|d| d.card_id == top.card_id);
    let base_dp = top_data.and_then(|d| d.dp).unwrap_or(0);
    let level = top_data.and_then(|d| d.level).unwrap_or(0);
    let colors = top_data.map(|d| colors_of(d)).unwrap_or_default();

    let sources: Vec<Value> = perm
        .card_sources
        .iter()
        .enumerate()
        .map(|(i, cs)| {
            let cd = data.iter().find(|d| d.card_id == cs.card_id);
            json!({
                "cardId": cs.card_id,
                "cardName": cd.map(|d| d.card_name.as_str()).unwrap_or(""),
                "isTop": i + 1 == perm.card_sources.len(),
                "optState": 0.0,
                "dpContribution": 0,
                "mainEffectText": "",
                "inheritedEffectText": "",
                "colors": cd.map(|d| colors_of(d)).unwrap_or_default(),
            })
        })
        .collect();

    json!({
        "topCardId": top.card_id,
        "topCardName": top_data.map(|d| d.card_name.as_str()).unwrap_or(""),
        "dp": base_dp,
        "level": level,
        "isSuspended": perm.is_suspended,
        "sourceCount": perm.card_sources.len(),
        "keywords": Vec::<String>::new(),
        "keywordBreakdown": json!({ "innate": [], "gained": [] }),
        "securityAttackModifier": 0,
        "linkedCardIds": perm.linked_cards.iter().map(|c| &c.card_id).collect::<Vec<_>>(),
        "sources": sources,
        "mainEffectText": "",
        "inheritedEffects": [],
        "dpBreakdown": json!({
            "base": base_dp,
            "sources": [],
            "temporary": 0.0,
            "aura": 0,
            "total": base_dp,
        }),
        "turnPlayed": perm.turn_played,
        "colors": colors,
    })
}

/// Build the full UI-state dict. `state_filter.py` consumes this directly.
pub fn to_ui_json(game: &Game) -> Value {
    let ps = game.pending_selection.as_ref().map(|s| s.view());
    let pending_sel_value = match ps {
        None => Value::Null,
        Some(v) => {
            let mut m = Map::new();
            m.insert("kind".into(), Value::String(v.kind_str()));
            m.insert("phase".into(), Value::String(v.previous_phase_str()));
            m.insert(
                "selectingPlayer".into(),
                Value::from(py_pid(v.selecting_player)),
            );
            m.insert(
                "validIndices".into(),
                Value::Array(v.valid_action_ids.iter().map(|i| json!(*i)).collect()),
            );
            m.insert("isOptional".into(), Value::from(v.is_optional));
            m.insert("prompt".into(), Value::String(v.prompt.clone()));
            if let Some(choices) = v.effect_choices.as_ref() {
                m.insert(
                    "effectChoices".into(),
                    Value::Array(
                        choices
                            .iter()
                            .map(|c| json!({"label": c.label, "actionId": c.action_id}))
                            .collect(),
                    ),
                );
            }
            Value::Object(m)
        }
    };

    let pending_attack = game.pending_attack.as_ref().map(|pa| {
        json!({
            "attackerPlayer": py_pid(pa.attacker.player),
            "attackerFieldIndex": pa.attacker.index,
            "isBlocked": pa.is_blocked,
            "state": format!("{:?}", pa.state),
        })
    }).unwrap_or(Value::Null);

    let revealed: Vec<Value> = game
        .revealed_cards
        .iter()
        .map(|cs| json!({"cardId": cs.card_id, "owner": py_pid(cs.owner)}))
        .collect();

    json!({
        "turnCount": game.turn_count,
        "currentPhase": phase_str(game.current_phase),
        "currentPlayer": py_pid(game.turn_player()),
        "memoryGauge": game.memory,
        "isGameOver": game.game_over,
        "winner": game.winner.map(py_pid),
        "player1": player_ui_data(&game.players[0], &game.card_data, game),
        "player2": player_ui_data(&game.players[1], &game.card_data, game),
        "revealedCards": revealed,
        "pendingSelection": pending_sel_value,
        "pendingAttack": pending_attack,
    })
}
```

**Important:** the exact field names on `CardSource`, `CardData`, and `Permanent` may differ slightly — if `card_id` is actually a method rather than a field, or `owner` isn't on `CardSource`, adjust the accessor calls accordingly. Look at `digimon-engine/src/card_source.rs` and `digimon-engine/src/card_data.rs` for the real names. If a name is different, update it consistently everywhere in this file and the Rust tests.

- [ ] **Step 3.5: Declare the module in `lib.rs`**

Edit `digimon-engine/src/lib.rs`:

```rust
pub mod serialization;
```

- [ ] **Step 3.6: Run tests — expect pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity ui_json`
Expected: all 5 tests PASS.

If compilation errors mention missing fields on `CardData` / `CardSource`, align the accessor names to what actually exists — the dict *shape* is what must match; the *source* of each value is local.

- [ ] **Step 3.7: Run full Rust suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: everything green.

- [ ] **Step 3.8: Commit**

```bash
git add digimon-engine/src/serialization.rs digimon-engine/src/lib.rs \
        digimon-engine/tests/ffi_parity/ui_json.rs \
        digimon-engine/tests/ffi_parity/main.rs
git commit -m "feat(engine): to_ui_json with Python-matching dict shape"
```

---

## Task 4: Add `GameRecorder` in Rust

**Files:**
- Create: `digimon-engine/src/runners/recorder.rs`
- Modify: `digimon-engine/src/runners/mod.rs` (add `pub mod recorder;`)
- Modify: `digimon-engine/src/runners/headless.rs` (embed recorder, record on step)
- Modify: `digimon-engine/src/lib.rs` (re-export)
- Create: `digimon-engine/tests/ffi_parity/recorder.rs`
- Modify: `digimon-engine/tests/ffi_parity/main.rs`

**Rationale:** Python's `digimon_gym/engine/recording.py::GameRecorder` produces a dict with `{initial_state, actions[], total_actions, tensor_snapshots_count, tensor_snapshots}` consumed by the replay viewer. Mirror that shape in Rust, wire it into `HeadlessRunner::step`, and expose a serializable dict via `get_recording()`. Keeps replays interchangeable regardless of which engine generated them.

- [ ] **Step 4.1: Write the failing test**

Create `digimon-engine/tests/ffi_parity/recorder.rs`:

```rust
//! Recorder wires into HeadlessRunner::step. N steps → N recorded
//! actions, dict shape matches Python's `GameRecorder.to_dict`.

use digimon_engine::card_data::CardData;
use digimon_engine::HeadlessRunner;
use std::collections::HashMap;

fn minimal_db() -> HashMap<String, CardData> {
    // Load the real cards.json if available; otherwise skip — the recorder
    // tests only care about shape, not card semantics.
    let path = std::env::var("DIGIMON_CARDS_JSON")
        .unwrap_or_else(|_| "../digimon_gym/engine/data/cards.json".into());
    CardData::load_from_file(&std::path::Path::new(&path))
        .expect("cards.json load failed; set DIGIMON_CARDS_JSON")
}

#[test]
fn recording_has_expected_top_level_keys() {
    let db = minimal_db();
    let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect();
    let mut r =
        HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(42)).unwrap();
    r.step(62); // PASS
    r.step(62);
    r.step(62);

    let rec = r.get_recording().expect("recorder enabled → recording present");
    for key in [
        "initial_state",
        "actions",
        "total_actions",
        "tensor_snapshots_count",
        "tensor_snapshots",
    ] {
        assert!(rec.as_object().unwrap().contains_key(key), "missing {:?}", key);
    }
    assert_eq!(rec["total_actions"], serde_json::json!(3));
    assert_eq!(rec["actions"].as_array().unwrap().len(), 3);
}

#[test]
fn recording_initial_state_has_both_players() {
    let db = minimal_db();
    let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect();
    let r = HeadlessRunner::new(deck.clone(), deck, &db, false, true, false, Some(7)).unwrap();
    let rec = r.get_recording().expect("recorder enabled");
    let init = &rec["initial_state"];
    assert!(init["player1"].is_object());
    assert!(init["player2"].is_object());
    assert!(init["first_player_id"].is_i64());
    assert!(init["timestamp"].is_string());
    for key in [
        "player_id",
        "deck_list",
        "library_order",
        "digitama_library_order",
        "security_order",
        "initial_hand",
    ] {
        assert!(
            init["player1"].as_object().unwrap().contains_key(key),
            "initial_state.player1 missing {:?}",
            key
        );
    }
}

#[test]
fn recording_returns_none_when_disabled() {
    let db = minimal_db();
    let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect();
    // record_actions=false → no recorder
    let r =
        HeadlessRunner::new(deck.clone(), deck, &db, false, false, false, Some(1)).unwrap();
    assert!(r.get_recording().is_none());
}
```

- [ ] **Step 4.2: Add `mod recorder` to the test binary**

Edit `digimon-engine/tests/ffi_parity/main.rs` to add `mod recorder;`.

- [ ] **Step 4.3: Run tests — expect compile failure**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity recorder`
Expected: FAIL — `HeadlessRunner::get_recording` returns `Option<()>`, not `Option<serde_json::Value>`.

- [ ] **Step 4.4: Create the recorder module**

Create `digimon-engine/src/runners/recorder.rs`:

```rust
//! Game-state recorder. Captures initial state + action stream so a
//! replay viewer can reconstruct the game turn-by-turn. Shape mirrors
//! Python's `digimon_gym/engine/recording.py::GameRecorder.to_dict`.

use serde_json::{json, Value};

use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;

#[derive(Debug, Clone)]
pub struct PlayerInitialState {
    pub player_id: PlayerId,
    pub deck_list: Vec<String>,
    pub library_order: Vec<String>,
    pub digitama_library_order: Vec<String>,
    pub security_order: Vec<String>,
    pub initial_hand: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InitialState {
    pub first_player_id: PlayerId,
    pub player1: PlayerInitialState,
    pub player2: PlayerInitialState,
    pub timestamp: String,
}

#[derive(Debug, Clone)]
pub struct RecordedAction {
    pub step_number: u32,
    pub player_id: PlayerId,
    pub action_id: u16,
    pub phase: GamePhase,
    pub memory_before: i16,
    pub memory_after: i16,
    pub turn_number: u16,
    pub is_game_over: bool,
    pub winner_id: Option<PlayerId>,
}

#[derive(Debug, Clone)]
pub struct TensorSnapshot {
    pub step: u32,
    pub player_id: PlayerId,
    pub tensor: Vec<f32>,
    pub action_mask: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct GameRecorder {
    pub record_tensors: bool,
    pub initial_state: Option<InitialState>,
    pub actions: Vec<RecordedAction>,
    pub tensor_snapshots: Vec<TensorSnapshot>,
    step_counter: u32,
}

impl GameRecorder {
    pub fn new(record_tensors: bool) -> Self {
        Self {
            record_tensors,
            initial_state: None,
            actions: Vec::new(),
            tensor_snapshots: Vec::new(),
            step_counter: 0,
        }
    }

    /// Capture deck lists + initial hands. Call once, after `game.start_game()`.
    pub fn capture_initial_state(
        &mut self,
        game: &Game,
        deck_lists: (&[String], &[String]),
    ) {
        let p1 = &game.players[0];
        let p2 = &game.players[1];
        let to_ids = |v: &[crate::card_source::CardSource]| -> Vec<String> {
            v.iter().map(|c| c.card_id.clone()).collect()
        };
        self.initial_state = Some(InitialState {
            first_player_id: game.turn_order[0],
            timestamp: timestamp_iso8601(),
            player1: PlayerInitialState {
                player_id: p1.id,
                deck_list: deck_lists.0.to_vec(),
                library_order: to_ids(&p1.deck),
                digitama_library_order: to_ids(&p1.digitama_deck),
                security_order: to_ids(&p1.security),
                initial_hand: to_ids(&p1.hand),
            },
            player2: PlayerInitialState {
                player_id: p2.id,
                deck_list: deck_lists.1.to_vec(),
                library_order: to_ids(&p2.deck),
                digitama_library_order: to_ids(&p2.digitama_deck),
                security_order: to_ids(&p2.security),
                initial_hand: to_ids(&p2.hand),
            },
        });
    }

    /// Record the action about to be applied. Call BEFORE `game.decode_action`.
    /// Returns an index into `self.actions` so `finalize_action` can update it.
    pub fn begin_action(&mut self, game: &Game, action_id: u16, player_id: PlayerId) -> usize {
        self.step_counter += 1;
        let rec = RecordedAction {
            step_number: self.step_counter,
            player_id,
            action_id,
            phase: game.current_phase,
            memory_before: game.memory,
            memory_after: game.memory,
            turn_number: game.turn_count,
            is_game_over: false,
            winner_id: None,
        };
        self.actions.push(rec);
        self.actions.len() - 1
    }

    /// Update memory_after / is_game_over / winner_id. Call AFTER `decode_action`.
    pub fn finalize_action(&mut self, idx: usize, game: &Game) {
        let rec = &mut self.actions[idx];
        rec.memory_after = game.memory;
        rec.is_game_over = game.game_over;
        rec.winner_id = game.winner;
    }

    pub fn record_tensor(&mut self, player_id: PlayerId, tensor: Vec<f32>, mask: Vec<f32>) {
        if !self.record_tensors {
            return;
        }
        self.tensor_snapshots.push(TensorSnapshot {
            step: self.step_counter,
            player_id,
            tensor,
            action_mask: mask,
        });
    }

    /// Serialize to JSON matching Python's `GameRecorder.to_dict`. Player
    /// IDs are translated to the Python 1/2 convention at this layer.
    pub fn to_json(&self) -> Value {
        let py_pid = |p: PlayerId| -> i64 { (p as i64) + 1 };
        let py_opt_pid = |p: Option<PlayerId>| -> Value {
            match p {
                None => Value::Null,
                Some(pid) => json!(py_pid(pid)),
            }
        };

        let initial_state = match &self.initial_state {
            None => Value::Null,
            Some(is) => json!({
                "first_player_id": py_pid(is.first_player_id),
                "timestamp": is.timestamp,
                "player1": player_initial_json(&is.player1, py_pid),
                "player2": player_initial_json(&is.player2, py_pid),
            }),
        };

        let actions: Vec<Value> = self
            .actions
            .iter()
            .map(|a| {
                json!({
                    "step": a.step_number,
                    "player_id": py_pid(a.player_id),
                    "action_id": a.action_id,
                    "phase": format!("{:?}", a.phase),
                    "memory_before": a.memory_before,
                    "memory_after": a.memory_after,
                    "turn": a.turn_number,
                    "is_game_over": a.is_game_over,
                    "winner_id": py_opt_pid(a.winner_id),
                })
            })
            .collect();

        let tensors: Vec<Value> = self
            .tensor_snapshots
            .iter()
            .map(|ts| {
                json!({
                    "step": ts.step,
                    "player_id": py_pid(ts.player_id),
                    "tensor": ts.tensor,
                    "action_mask": ts.action_mask.iter().map(|m| *m as i64).collect::<Vec<_>>(),
                })
            })
            .collect();

        json!({
            "initial_state": initial_state,
            "actions": actions,
            "total_actions": self.actions.len(),
            "tensor_snapshots_count": self.tensor_snapshots.len(),
            "tensor_snapshots": tensors,
        })
    }
}

fn player_initial_json(
    p: &PlayerInitialState,
    py_pid: impl Fn(PlayerId) -> i64,
) -> Value {
    json!({
        "player_id": py_pid(p.player_id),
        "deck_list": p.deck_list,
        "library_order": p.library_order,
        "digitama_library_order": p.digitama_library_order,
        "security_order": p.security_order,
        "initial_hand": p.initial_hand,
    })
}

/// RFC3339 timestamp without external deps. Uses the system clock at second
/// resolution — matches Python's `datetime.now().isoformat()` granularity
/// closely enough for replay identification.
fn timestamp_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Minimal format: "<unix-seconds>". Replaces full ISO-8601 for the
    // alpha; good enough for identification. Upgrade if/when chrono lands.
    format!("{}", secs)
}
```

- [ ] **Step 4.5: Declare module + re-export**

Edit `digimon-engine/src/runners/mod.rs`:

```rust
pub mod headless;
pub mod recorder;
pub use headless::HeadlessRunner;
pub use recorder::GameRecorder;
```

Edit `digimon-engine/src/lib.rs` to re-export `GameRecorder` alongside existing exports:

```rust
pub use crate::runners::GameRecorder;
```

- [ ] **Step 4.6: Wire recorder into `HeadlessRunner`**

Edit `digimon-engine/src/runners/headless.rs`. Add import:

```rust
use crate::runners::recorder::GameRecorder;
```

Extend the struct:

```rust
pub struct HeadlessRunner {
    pub game: Game,
    registry: CardRegistry,
    #[allow(dead_code)]
    verbose: bool,
    record_actions: bool,
    #[allow(dead_code)]
    record_tensors: bool,
    deck1_ids: Vec<String>,
    deck2_ids: Vec<String>,
    recorder: Option<GameRecorder>,
}
```

Update `new`:

```rust
pub fn new(
    deck1_ids: Vec<String>,
    deck2_ids: Vec<String>,
    all_card_data: &HashMap<String, CardData>,
    verbose: bool,
    record_actions: bool,
    record_tensors: bool,
    seed: Option<u64>,
) -> Result<Self, String> {
    let registry = CardRegistry::from_cards(all_card_data);
    let decks = vec![deck1_ids.clone(), deck2_ids.clone()];
    let game = Game::new(&decks, all_card_data, Rules::standard(), seed)?;

    let recorder = if record_actions {
        let mut rec = GameRecorder::new(record_tensors);
        rec.capture_initial_state(&game, (&deck1_ids, &deck2_ids));
        Some(rec)
    } else {
        None
    };

    Ok(Self {
        game,
        registry,
        verbose,
        record_actions,
        record_tensors,
        deck1_ids,
        deck2_ids,
        recorder,
    })
}
```

Update `step`:

```rust
pub fn step(&mut self, action_id: u16) {
    if self.game.game_over {
        return;
    }
    let pid = self.current_decision_player();
    let idx = self
        .recorder
        .as_mut()
        .map(|r| r.begin_action(&self.game, action_id, pid));
    self.game.decode_action(action_id, pid);
    if let (Some(i), Some(r)) = (idx, self.recorder.as_mut()) {
        r.finalize_action(i, &self.game);
    }
}
```

Replace `get_recording`:

```rust
pub fn get_recording(&self) -> Option<serde_json::Value> {
    self.recorder.as_ref().map(|r| r.to_json())
}
```

- [ ] **Step 4.7: Run tests — expect pass**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml --test ffi_parity recorder`
Expected: all 3 tests PASS.

If the deck IDs `ST1-01` and `ST1-03` don't exist in the local `cards.json`, adjust the test fixture to any two cards that do exist.

- [ ] **Step 4.8: Run full Rust suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: everything green. If any existing test called `runner.get_recording()` expecting `Option<()>`, update its type.

- [ ] **Step 4.9: Commit**

```bash
git add digimon-engine/src/runners/recorder.rs \
        digimon-engine/src/runners/mod.rs \
        digimon-engine/src/runners/headless.rs \
        digimon-engine/src/lib.rs \
        digimon-engine/tests/ffi_parity/recorder.rs \
        digimon-engine/tests/ffi_parity/main.rs
git commit -m "feat(engine): GameRecorder with replay-compatible dict shape"
```

---

## Task 5: PyO3 bindings for all four surfaces

**Files:**
- Modify: `digimon-engine-py/Cargo.toml` (add serde_json dep)
- Modify: `digimon-engine-py/src/lib.rs` (expose 4 new methods)

**Rationale:** With the Rust-side surfaces in place and tested, the PyO3 layer is mechanical: walk a `serde_json::Value` tree and build a `PyDict` / `PyList`. The one non-mechanical bit is event serialization — each `GameEvent` variant maps to a dict with `type`/`seq`/`player`/... matching Python's `GameEvent.to_dict` shape.

- [ ] **Step 5.1: Add serde_json to PyO3 crate**

Edit `digimon-engine-py/Cargo.toml`:

```toml
[dependencies]
digimon-engine = { path = "../digimon-engine" }
pyo3 = { version = "0.22", features = ["extension-module", "abi3-py311"] }
numpy = "0.22"
serde_json = "1"
```

- [ ] **Step 5.2: Add a `json_to_pyobject` helper and event conversion**

Open `digimon-engine-py/src/lib.rs`. Add these imports at the top alongside existing ones:

```rust
use pyo3::types::{PyDict, PyFloat, PyList};
use serde_json::Value;
use ::digimon_engine::events::GameEvent;
```

Add this helper near the bottom of the file (before the `#[pymodule]` block):

```rust
/// Recursively convert a `serde_json::Value` into a Python object.
/// Objects become `PyDict`, arrays become `PyList`, numbers become
/// `int` or `float`, null becomes `None`.
fn json_to_pyobject(py: Python, v: &Value) -> PyResult<PyObject> {
    Ok(match v {
        Value::Null => py.None(),
        Value::Bool(b) => b.into_py(py),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.into_py(py)
            } else if let Some(u) = n.as_u64() {
                u.into_py(py)
            } else {
                n.as_f64().unwrap_or(0.0).into_py(py)
            }
        }
        Value::String(s) => s.into_py(py),
        Value::Array(a) => {
            let list = PyList::empty_bound(py);
            for item in a {
                list.append(json_to_pyobject(py, item)?)?;
            }
            list.into_py(py)
        }
        Value::Object(o) => {
            let dict = PyDict::new_bound(py);
            for (k, val) in o {
                dict.set_item(k.as_str(), json_to_pyobject(py, val)?)?;
            }
            dict.into_py(py)
        }
    })
}

/// Convert a single `GameEvent` into a dict matching Python's
/// `GameEvent.to_dict` shape — keys: `type`, `seq`, `player`,
/// `source_card_id`, `source_slot`, `target_card_id`, `target_slot`,
/// `meta`.
fn event_to_pydict<'py>(py: Python<'py>, ev: &GameEvent) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new_bound(py);
    d.set_item("type", ev.type_str())?;
    d.set_item("seq", ev.seq())?;
    d.set_item("meta", PyDict::new_bound(py))?;
    // defaults
    d.set_item("source_card_id", py.None())?;
    d.set_item("source_slot", py.None())?;
    d.set_item("target_card_id", py.None())?;
    d.set_item("target_slot", py.None())?;
    d.set_item("player", 0)?;

    let py_pid = |p: u8| -> i64 { (p as i64) + 1 };

    match ev {
        GameEvent::MemoryChange { player, delta, total, .. } => {
            d.set_item("player", py_pid(*player))?;
            let meta = PyDict::new_bound(py);
            meta.set_item("delta", *delta)?;
            meta.set_item("total", *total)?;
            d.set_item("meta", meta)?;
        }
        GameEvent::TurnStart { player, turn_count, .. } => {
            d.set_item("player", py_pid(*player))?;
            let meta = PyDict::new_bound(py);
            meta.set_item("turn_count", *turn_count)?;
            d.set_item("meta", meta)?;
        }
        GameEvent::PhaseChange { player, phase, .. } => {
            d.set_item("player", py_pid(*player))?;
            let meta = PyDict::new_bound(py);
            meta.set_item("phase", format!("{:?}", phase))?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Play { player, card_id, field_index, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
            d.set_item("source_slot", *field_index)?;
        }
        GameEvent::Digivolve { player, top_card_id, field_index, from_stack_top, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", top_card_id.as_str())?;
            d.set_item("source_slot", *field_index)?;
            let meta = PyDict::new_bound(py);
            meta.set_item("from_stack_top", from_stack_top.as_str())?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Attack {
            player,
            attacker_field_index,
            target_field_index,
            target_player,
            ..
        } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_slot", *attacker_field_index)?;
            if let Some(t) = target_field_index {
                d.set_item("target_slot", *t)?;
            }
            let meta = PyDict::new_bound(py);
            meta.set_item(
                "target_player",
                target_player.map(|p| py_pid(p)),
            )?;
            d.set_item("meta", meta)?;
        }
        GameEvent::Trash { player, card_id, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::Mill { player, card_id, .. } => {
            d.set_item("player", py_pid(*player))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::SecurityReveal { defender, card_id, .. } => {
            d.set_item("player", py_pid(*defender))?;
            d.set_item("source_card_id", card_id.as_str())?;
        }
        GameEvent::GameOver { winner, .. } => {
            let meta = PyDict::new_bound(py);
            meta.set_item(
                "winner",
                winner.map(|w| py_pid(w)),
            )?;
            d.set_item("meta", meta)?;
        }
    }
    Ok(d)
}
```

- [ ] **Step 5.3: Replace stub methods with real implementations**

In `digimon-engine-py/src/lib.rs`, replace the `get_recording` stub (currently at line ~180) and add three new methods inside the `#[pymethods] impl RustHeadlessGame` block:

```rust
    /// Full UI-state dict. Matches Python's
    /// `digimon_gym.engine.game.serialization.to_ui_json`. Consumed by
    /// `state_filter.py` and the React frontend.
    fn to_ui_json(&self, py: Python) -> PyResult<PyObject> {
        let value = ::digimon_engine::serialization::to_ui_json(&self.inner.game);
        json_to_pyobject(py, &value)
    }

    /// Snapshot of the currently installed `PendingSelection`, or `None`
    /// if no prompt is pending. Keys: `kind`, `phase`, `selectingPlayer`
    /// (Python 1/2 convention), `validIndices`, `isOptional`, `prompt`,
    /// optional `effectChoices`.
    fn get_pending_selection(&self, py: Python) -> PyResult<PyObject> {
        let game = &self.inner.game;
        match game.pending_selection.as_ref() {
            None => Ok(py.None()),
            Some(sel) => {
                let v = sel.view();
                let d = PyDict::new_bound(py);
                d.set_item("kind", v.kind_str())?;
                d.set_item("phase", v.previous_phase_str())?;
                d.set_item(
                    "selectingPlayer",
                    (v.selecting_player as i64) + 1,
                )?;
                d.set_item("validIndices", v.valid_action_ids.clone())?;
                d.set_item("isOptional", v.is_optional)?;
                d.set_item("prompt", v.prompt.clone())?;
                if let Some(choices) = v.effect_choices.as_ref() {
                    let list = PyList::empty_bound(py);
                    for c in choices {
                        let cd = PyDict::new_bound(py);
                        cd.set_item("label", c.label.as_str())?;
                        cd.set_item("actionId", c.action_id)?;
                        list.append(cd)?;
                    }
                    d.set_item("effectChoices", list)?;
                }
                Ok(d.into_py(py))
            }
        }
    }

    /// Drain accumulated `GameEvent`s since the last call. Each dict has
    /// `type`, `seq`, `player` (Python 1/2), `source_card_id`,
    /// `source_slot`, `target_card_id`, `target_slot`, `meta`. Matches
    /// Python `GameEvent.to_dict`.
    fn get_events_since_last_step<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyList>> {
        let drained = self.inner.game.drain_events();
        let list = PyList::empty_bound(py);
        for ev in drained {
            list.append(event_to_pydict(py, &ev)?)?;
        }
        Ok(list)
    }

    /// Recording dict, or `None` if `record_actions=False` at
    /// construction. Shape matches Python `GameRecorder.to_dict`.
    fn get_recording(&self, py: Python) -> PyResult<PyObject> {
        match self.inner.get_recording() {
            None => Ok(py.None()),
            Some(v) => json_to_pyobject(py, &v),
        }
    }
```

**Remove** the old stub `get_recording` that returns `py.None()` unconditionally.

- [ ] **Step 5.4: Build the wheel**

Run: `cd digimon-engine-py && maturin develop`
Expected: clean build, `digimon_engine` installed in the active Python environment.

- [ ] **Step 5.5: Smoke-check from Python**

Run:
```bash
python -c "from digimon_engine import RustHeadlessGame; g = RustHeadlessGame(['ST1-01']*5 + ['ST1-03']*45, ['ST1-01']*5 + ['ST1-03']*45, record_actions=True); print(sorted(g.to_ui_json().keys())); print('pend:', g.get_pending_selection()); print('events type:', type(g.get_events_since_last_step())); print('rec keys:', sorted(g.get_recording().keys()))"
```

Expected output: a sorted list of UI-JSON keys including `turnCount`, `player1`, `player2`; `pend: None`; `events type: <class 'list'>`; rec keys including `initial_state`, `actions`, `total_actions`.

- [ ] **Step 5.6: Commit**

```bash
git add digimon-engine-py/Cargo.toml digimon-engine-py/src/lib.rs
git commit -m "feat(pyo3): to_ui_json, pending_selection, events, recording"
```

---

## Task 6: Python-side parity tests

**Files:**
- Modify: `tests/engine/test_rust_backend_parity.py` (add 4 new test functions)

**Rationale:** The Rust side proves the shapes internally; the Python side proves the PyO3 translation is correct and that both engines agree well enough for the WS layer. Assert key sets match, player-ID conventions are Python-1/2, event dicts have the same fields as Python `GameEvent.to_dict`, and recording dicts are interchangeable.

- [ ] **Step 6.1: Add parity tests**

Append to `tests/engine/test_rust_backend_parity.py`:

```python
def test_to_ui_json_top_level_keys_match():
    py, rs = _build()
    py_ui = py.to_ui_json() if hasattr(py, "to_ui_json") else None
    # HeadlessGame wraps Game — reach through if needed
    if py_ui is None:
        from digimon_gym.engine.game.serialization import to_ui_json
        py_ui = to_ui_json(py.game)
    rs_ui = rs.to_ui_json()
    assert set(rs_ui.keys()) == set(py_ui.keys()), (
        f"UI-JSON key set differs.\nRust only: {set(rs_ui) - set(py_ui)}\n"
        f"Python only: {set(py_ui) - set(rs_ui)}"
    )


def test_to_ui_json_player_ids_use_python_convention():
    _, rs = _build()
    ui = rs.to_ui_json()
    assert ui["player1"]["id"] == 1
    assert ui["player2"]["id"] == 2
    assert ui["currentPlayer"] in (1, 2)


def test_player_ui_data_key_set_matches():
    _, rs = _build()
    from digimon_gym.engine.game.serialization import to_ui_json
    from digimon_gym.engine.runners.headless_game import HeadlessGame
    py = HeadlessGame(DECK1, DECK2)
    py_ui = to_ui_json(py.game)
    rs_ui = rs.to_ui_json()
    assert set(rs_ui["player1"].keys()) == set(py_ui["player1"].keys()), (
        f"player1 key set differs.\nRust only: "
        f"{set(rs_ui['player1']) - set(py_ui['player1'])}\n"
        f"Python only: {set(py_ui['player1']) - set(rs_ui['player1'])}"
    )


def test_pending_selection_returns_none_at_start():
    _, rs = _build()
    assert rs.get_pending_selection() is None


def test_get_events_returns_list():
    _, rs = _build()
    rs.step(62)  # PASS
    events = rs.get_events_since_last_step()
    assert isinstance(events, list)
    for ev in events:
        assert "type" in ev
        assert "seq" in ev
        assert "player" in ev
        assert "meta" in ev


def test_events_drained_between_calls():
    _, rs = _build()
    rs.step(62)
    _ = rs.get_events_since_last_step()
    # No step between calls → no new events
    assert rs.get_events_since_last_step() == []


def test_recording_returns_none_without_record_flag():
    rs = RustHeadlessGame(DECK1, DECK2)
    assert rs.get_recording() is None


def test_recording_has_expected_shape():
    rs = RustHeadlessGame(DECK1, DECK2, record_actions=True)
    rs.step(62)
    rs.step(62)
    rec = rs.get_recording()
    assert rec is not None
    for key in (
        "initial_state",
        "actions",
        "total_actions",
        "tensor_snapshots_count",
        "tensor_snapshots",
    ):
        assert key in rec, f"missing {key} in recording"
    assert rec["total_actions"] == 2
    assert rec["initial_state"]["player1"]["player_id"] == 1
    assert rec["initial_state"]["player2"]["player_id"] == 2
```

- [ ] **Step 6.2: Run parity tests — expect pass**

Run: `DIGIMON_BACKEND=rust python -m pytest tests/engine/test_rust_backend_parity.py -v`
Expected: all existing + 8 new tests PASS.

If the `test_to_ui_json_top_level_keys_match` assertion fails, the top-level key set is the spec — the resolution is in the Rust `serialization.rs` (adjust naming) or the Python `serialization.py` if an obvious drift is present. Do NOT paper over mismatches at the PyO3 layer — surface them.

- [ ] **Step 6.3: Run the full engine test suite**

Run: `python -m pytest tests/engine -v`
Expected: all green.

- [ ] **Step 6.4: Commit**

```bash
git add tests/engine/test_rust_backend_parity.py
git commit -m "test(parity): Rust backend parity for WS-bound surfaces"
```

---

## Task 7: Final verification

- [ ] **Step 7.1: Full Rust suite**

Run: `cargo test --manifest-path digimon-engine/Cargo.toml`
Expected: all green.

- [ ] **Step 7.2: Full Python engine suite under Rust backend**

Run: `DIGIMON_BACKEND=rust python -m pytest tests/engine -v`
Expected: all green.

- [ ] **Step 7.3: Full Python engine suite under Python backend (regression check)**

Run: `python -m pytest tests/engine -v`
Expected: all green — nothing in the Python engine should have changed.

- [ ] **Step 7.4: PyO3 crate smoke test**

Run: `cargo test --manifest-path digimon-engine-py/Cargo.toml`
Expected: compiles cleanly (this crate has no internal tests but compilation is the sanity check).

- [ ] **Step 7.5: Open PR**

Commit messages form the PR summary. PR body should list the four new PyO3 methods (`to_ui_json`, `get_pending_selection`, `get_events_since_last_step`, `get_recording`), call out the unwired `GameEvent` variants (Digivolve/Attack/Trash/Mill/SecurityReveal — schema landed, emission follows), and link to the parity test file.

---

## Self-review checklist (addressed during authoring)

- **Spec coverage:** All four priority items covered (pending_selection view → Task 1, events → Task 2, to_ui_json → Task 3, recording → Task 4); PyO3 surface in Task 5; parity tests in Task 6.
- **Constraints honored:** Player-ID convention translated at PyO3 + serialization boundaries (rule #20); action masking untouched (rule #3); TDD order — failing test → impl → pass → commit (rule #18); no card scripts authored (constraint).
- **Type consistency:** `PendingSelectionView`, `GameEvent`, `GameRecorder` names identical across every task that references them. `kind_str`/`previous_phase_str` used consistently.
- **Unwired emission points** (`Digivolve`, `Attack`, `Trash`, `Mill`, `SecurityReveal`) are documented as future work — the enum variants and PyO3 conversion exist so emission can be added without schema churn.
