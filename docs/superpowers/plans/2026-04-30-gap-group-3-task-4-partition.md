# Group 3 Task 4 Partition Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add BT16-025-style Partition replacement support where a deletion replacement selects one source for each printed requirement, plays those selected sources without cost, and cancels the original deletion.

**Architecture:** This plan owns the shared Partition API for Group 3. It adds a `PartitionRequirement` type, a `ReplacementSubject::permanent()` accessor, a partition-specific wrapper over existing source selection, and helpers to move selected sources from a stack to the battle area while committing the parked replacement outcome.

**Tech Stack:** Rust engine, native `CardEffect` test fixtures, replacement dispatcher, pending source selection.

---

## Session Boundary

Suggested branch: `codex/group-3-task-4-partition`.

This session owns:
- `code/digimon-engine/tests/replacements/partition.rs`
- `code/digimon-engine/tests/replacements/main.rs`
- `code/digimon-engine/src/effect_context/mod.rs`
- `code/digimon-engine/src/effect_context/selections.rs`
- `code/digimon-engine/src/replacement.rs`

Coordinate with Task 5 and Task 6 before changing any shared replacement helper names. The public helpers exported by this task are:

```rust
PartitionRequirement::new(...)
ReplacementSubject::permanent()
EffectContext::select_partition_sources(...)
EffectContext::play_selected_sources_without_cost(...)
EffectContext::cancel_current_replacement()
Game::remove_source_ref(...)
Game::play_card_from_effect_without_cost(...)
Game::cancel_parked_replacement()
```

---

### Task 1: Add Partition Regression Coverage

**Files:**
- Create: `code/digimon-engine/tests/replacements/partition.rs`
- Modify: `code/digimon-engine/tests/replacements/main.rs`

- [ ] **Step 1: Register the test module**

Add this module declaration to `code/digimon-engine/tests/replacements/main.rs`:

```rust
mod partition;
```

- [ ] **Step 2: Create the failing Partition test**

Create `code/digimon-engine/tests/replacements/partition.rs` with this content:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::effect::{CardEffect, Effect};
use digimon_engine::enums::CardColor;
use digimon_engine::replacement::ReplacementCause;

fn colored_card(id: &str, color: CardColor, level: u8) -> digimon_engine::CardData {
    let mut card = make_test_card(id, id);
    card.colors = vec![color];
    card.level = Some(level);
    card
}

struct PaildramonPartition(Arc<Mutex<u32>>);

impl CardEffect for PaildramonPartition {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        let count = self.0.clone();
        vec![Effect::when_would_be_deleted(card)
            .name("BT16-025 Partition")
            .replacement_condition(|ctx, _subject| {
                ctx.replacement_cause() == Some(ReplacementCause::OpponentEffect)
            })
            .replacement_process(move |rctx| {
                *count.lock().unwrap() += 1;
                rctx.effect.select_partition_sources(
                    rctx.subject.permanent().expect("Partition subject is a permanent"),
                    "Partition BT16-025",
                    vec![
                        digimon_engine::effect_context::PartitionRequirement::new(
                            "Blue Lv.4",
                            |game, source| {
                                game.card(source.card).is_color(CardColor::Blue)
                                    && game.card(source.card).level == Some(4)
                            },
                        ),
                        digimon_engine::effect_context::PartitionRequirement::new(
                            "Green Lv.4",
                            |game, source| {
                                game.card(source.card).is_color(CardColor::Green)
                                    && game.card(source.card).level == Some(4)
                            },
                        ),
                    ],
                    move |ctx, selected| {
                        ctx.play_selected_sources_without_cost(selected);
                        ctx.cancel_current_replacement();
                    },
                );
            })
            .build()]
    }
}

#[test]
fn bt16_025_partition_requires_one_each_matching_source() {
    let fired = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-L4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-L4", CardColor::Green, 4))
        .add_card(colored_card("RED-L4", CardColor::Red, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition(fired.clone())));

    let host = r.place_on_field(0, "BT16-025", Some(0));
    r.add_source(host, "BLUE-L4");
    r.add_source(host, "GREEN-L4");
    r.add_source(host, "RED-L4");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);

    assert_eq!(*fired.lock().unwrap(), 1);
    assert!(r.game.pending_selection.is_some(), "Partition source prompt is exposed");

    r.game
        .resolve_selection(0, encode_source_select(0, 0))
        .expect("select blue Lv.4 source");
    r.game
        .resolve_selection(0, encode_source_select(0, 0))
        .expect("select green Lv.4 source after first source leaves candidate list");

    assert_eq!(r.battle_area_len(0), 3, "host survives and two sources are played");
    assert_eq!(r.trash_size(0), 0, "Partition selected sources are played, not trashed");
}

#[test]
fn bt16_025_partition_decline_allows_deletion() {
    let fired = Arc::new(Mutex::new(0));
    let mut r = DebugRunner::builder()
        .add_card(colored_card("BT16-025", CardColor::Blue, 5))
        .add_card(colored_card("BLUE-L4", CardColor::Blue, 4))
        .add_card(colored_card("GREEN-L4", CardColor::Green, 4))
        .start();
    r.register_effect("BT16-025", Arc::new(PaildramonPartition(fired)));

    let host = r.place_on_field(0, "BT16-025", Some(0));
    r.add_source(host, "BLUE-L4");
    r.add_source(host, "GREEN-L4");

    r.game
        .delete_permanent_with_cause(host, ReplacementCause::OpponentEffect);
    r.game.resolve_selection(0, PASS).expect("decline Partition");

    assert_eq!(r.battle_area_len(0), 0, "declining Partition allows deletion");
    assert_eq!(r.trash_size(0), 3, "host and sources go to trash");
}
```

- [ ] **Step 3: Verify the test fails for missing API**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition --nocapture
```

Expected: FAIL with missing symbols for `PartitionRequirement`, `select_partition_sources`, `play_selected_sources_without_cost`, `cancel_current_replacement`, or `ReplacementSubject::permanent`.

---

### Task 2: Add Partition API and Selection Enforcement

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-engine/src/replacement.rs`

- [ ] **Step 1: Add `ReplacementSubject::permanent`**

In `code/digimon-engine/src/replacement.rs`, extend the existing `impl ReplacementSubject`:

```rust
impl ReplacementSubject {
    pub fn permanent(self) -> Option<PermanentHandle> {
        match self {
            ReplacementSubject::Permanent(handle) => Some(handle),
            _ => None,
        }
    }
}
```

- [ ] **Step 2: Add `PartitionRequirement`**

In `code/digimon-engine/src/effect_context/mod.rs`, add imports for `Game` and `SourceSelectionRef` if the file does not already have them in scope, then add:

```rust
pub struct PartitionRequirement {
    pub label: &'static str,
    pub matches: Box<dyn Fn(&Game, SourceSelectionRef) -> bool + Send + Sync>,
}

impl PartitionRequirement {
    pub fn new<F>(label: &'static str, matches: F) -> Self
    where
        F: Fn(&Game, SourceSelectionRef) -> bool + Send + Sync + 'static,
    {
        Self {
            label,
            matches: Box::new(matches),
        }
    }
}
```

- [ ] **Step 3: Add exact one-source-per-requirement matching**

In `code/digimon-engine/src/effect_context/selections.rs`, add this helper near the source multi-select helpers:

```rust
fn selected_sources_satisfy_partition(
    game: &Game,
    selected: &[SourceSelectionRef],
    requirements: &[PartitionRequirement],
) -> bool {
    fn search(
        game: &Game,
        selected: &[SourceSelectionRef],
        requirements: &[PartitionRequirement],
        requirement_index: usize,
        used_sources: &mut Vec<bool>,
    ) -> bool {
        if requirement_index == requirements.len() {
            return true;
        }

        for source_index in 0..selected.len() {
            if used_sources[source_index] {
                continue;
            }
            if !(requirements[requirement_index].matches)(game, selected[source_index]) {
                continue;
            }
            used_sources[source_index] = true;
            if search(game, selected, requirements, requirement_index + 1, used_sources) {
                return true;
            }
            used_sources[source_index] = false;
        }

        false
    }

    if selected.len() != requirements.len() {
        return false;
    }

    let mut used_sources = vec![false; selected.len()];
    search(game, selected, requirements, 0, &mut used_sources)
}
```

- [ ] **Step 4: Add `select_partition_sources`**

In the same file, add this method to `impl<'a> EffectContext<'a>`:

```rust
pub fn select_partition_sources<C>(
    &mut self,
    host: PermanentHandle,
    prompt: &str,
    requirements: Vec<PartitionRequirement>,
    callback: C,
) where
    C: FnOnce(&mut EffectContext<'_>, Vec<SourceSelectionRef>) + Send + Sync + 'static,
{
    let required_count = requirements.len() as u8;
    self.select_own_sources(
        prompt,
        required_count,
        required_count,
        move |game, source_ref| {
            source_ref.permanent == host
                && requirements
                    .iter()
                    .any(|requirement| (requirement.matches)(game, source_ref))
        },
        move |ctx, selected| {
            if selected_sources_satisfy_partition(ctx.game, &selected, &requirements) {
                callback(ctx, selected);
            }
        },
    );
}
```

- [ ] **Step 5: Run the compile/test gate**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition --nocapture
```

Expected: still FAIL until the move/cancel helpers exist, but the missing `PartitionRequirement`, `ReplacementSubject::permanent`, and `select_partition_sources` errors are gone.

---

### Task 3: Add Source Play and Replacement Cancel Helpers

**Files:**
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Modify: `code/digimon-engine/src/replacement.rs`

- [ ] **Step 1: Add effect-context helpers**

Add these methods to `impl<'a> EffectContext<'a>` in `code/digimon-engine/src/effect_context/mod.rs`:

```rust
pub fn play_selected_sources_without_cost(&mut self, selected: Vec<SourceSelectionRef>) {
    for source_ref in selected {
        if let Some(card) = self.game.remove_source_ref(source_ref) {
            self.game.play_card_from_effect_without_cost(self.player, card);
        }
    }
}

pub fn cancel_current_replacement(&mut self) {
    self.game.cancel_parked_replacement();
}
```

- [ ] **Step 2: Add game helpers for source removal and no-cost play**

Add these methods to the `impl Game` block that already owns source movement helpers:

```rust
pub fn remove_source_ref(&mut self, source_ref: SourceSelectionRef) -> Option<CardHandle> {
    let owner = source_ref.permanent.player;
    let permanent_index = source_ref.permanent.index as usize;
    let source_index = source_ref.source_index as usize;
    let permanent = self.player_mut(owner).battle_area.get_mut(permanent_index)?;
    if source_index >= permanent.card_sources.len().saturating_sub(1) {
        return None;
    }
    Some(permanent.card_sources.remove(source_index))
}

pub fn play_card_from_effect_without_cost(&mut self, player: PlayerId, card: CardHandle) {
    self.place_card_in_battle_area_from_effect(player, card, true);
}
```

- [ ] **Step 3: Add parked replacement cancellation**

In `code/digimon-engine/src/replacement.rs`, add this method on `Game`:

```rust
pub fn cancel_parked_replacement(&mut self) {
    if let Some(parked) = self.parked_replacement.as_mut() {
        parked.outcome = ReplacementOutcome::Cancelled;
    }
}
```

- [ ] **Step 4: Run focused tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- partition --nocapture
cargo test --manifest-path code/digimon-engine/Cargo.toml --test replacements -- nested_select --nocapture
```

Expected: PASS.

---

### Task 4: Commit

**Files:**
- Stage all files from this plan.

- [ ] **Step 1: Check status**

Run:

```bash
git status --short
```

Expected: only files listed in this plan are modified or created.

- [ ] **Step 2: Commit**

Run:

```bash
git add code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/src/replacement.rs code/digimon-engine/tests/replacements/main.rs code/digimon-engine/tests/replacements/partition.rs
git commit -m "feat: add partition source replacement flow"
```

Expected: commit succeeds.
