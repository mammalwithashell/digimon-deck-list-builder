# Gap Group 2 Selection Primitives Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reusable selection and action-mask primitives so source picks, DP-budget multi-picks, branch choices, breeding-area targets, and empty-selection continuations are all visible through `PendingSelection` and legal action masks.

**Architecture:** Reuse the current pending-selection state machine and existing action ranges wherever possible. Add stable selection references for choices whose action IDs are positional, then lower DSL steps onto the same engine helpers used by hand-written Rust effects.

**Tech Stack:** Rust engine in `code/digimon-engine`, DSL compiler crate in `code/digimon-dsl`, Cargo integration tests, existing `DebugRunner`, `EffectContext`, `PendingSelection`, and action-mask infrastructure.

---

This plan changes shared action/selection internals. Do not run it in parallel with breeding permanent handle work, action-space resizing, or replacement nested-selection work.

## File Structure

Create:

- `code/digimon-engine/tests/selection/source_multi.rs`: engine tests for stable cross-permanent source picks, exact-N, up-to-N, PASS gating, and mutation by stable handles.
- `code/digimon-engine/tests/selection/dp_budget.rs`: engine tests for opponent permanent multi-select by aggregate DP budget.
- `code/digimon-engine/tests/selection/breeding_permanent.rs`: engine tests for selecting breeding permanents without fake battle-area handles.
- `code/digimon-engine/tests/mask_and_tensor/source_selection_mask.rs`: mask and decoder contract tests for source-selection actions.
- `code/digimon-engine/tests/mask_and_tensor/dp_budget_selection_mask.rs`: mask contract tests for DP-budget pending selections.
- `code/digimon-engine/tests/mask_and_tensor/breeding_selection_mask.rs`: mask contract tests for breeding permanent selections.
- `code/digimon-engine/tests/dsl/phase2g_select_sources.rs`: DSL tests for cross-permanent source selection and source trashing.
- `code/digimon-engine/tests/dsl/phase2g_dp_budget.rs`: DSL tests for DP-budget opponent permanent selection.
- `code/digimon-engine/tests/dsl/phase2g_breeding_selection.rs`: DSL tests for breeding permanent selection.

Modify:

- `code/digimon-engine/src/selection.rs`: add stable selection-reference structs and new selection-kind metadata.
- `code/digimon-engine/src/action/space.rs`: add small encoder helpers for existing ranges; keep `ACTION_SPACE_SIZE` unchanged.
- `code/digimon-engine/src/action/mask.rs`: preserve pending-selection masks and add regression tests for PASS gating.
- `code/digimon-engine/src/action/decode.rs`: route all new selection phases through `resolve_selection`.
- `code/digimon-engine/src/action/explain.rs`: explain source, DP-budget, and breeding target selection actions.
- `code/digimon-engine/src/debug_runner.rs`: add deterministic fixture helpers used by the new selection tests.
- `code/digimon-engine/src/effect_context/mod.rs`: re-export stable selection reference types.
- `code/digimon-engine/src/effect_context/selections.rs`: implement helper APIs for cross-permanent source selection, DP-budget permanent selection, and breeding permanent selection.
- `code/digimon-dsl/src/step.rs`: parse and serialize new DSL step specs.
- `code/digimon-dsl/src/compiled.rs`: add compiled step variants and binding value shapes for source refs.
- `code/digimon-dsl/src/compile.rs`: compile new selection steps.
- `code/digimon-engine/src/dsl_cards/bindings.rs`: add binding conversions for source-selection refs and permanent refs when needed.
- `code/digimon-engine/src/dsl_cards/step/selections.rs`: lower new compiled selection steps to `EffectContext` helpers.
- `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`: lower selected-source trashing to `EffectContext::trash_card_source`.
- `docs/ACTION_SPEC.md`: add a short note that Group 2 reuses existing action ranges and keeps `ACTION_SPACE_SIZE = 2168`.
- `docs/RUST_ENGINE_GAPS.md`: mark the resolved source-selection, DP-budget, breeding-selection, and empty-selection continuation gaps precisely.
- `qa/archetype-qa/engine-gaps.md`: narrow or close Rocks and Royal Knights selection blockers after tests pass.

Action-space decision:

- Do not change `ACTION_SPACE_SIZE` in this plan.
- Reuse `SOURCE_SELECT_START..SOURCE_SELECT_END` for source-card selections across player battle-area permanents.
- Reuse field target action IDs for DP-budget opponent permanent selections.
- Reuse an existing selection-only action range for breeding permanents by adding a phase-scoped encoder helper; the meaning is only active while `GamePhase::SelectBreedingPermanent` is pending.
- If a slice discovers more legal choices than fit in an existing range, stop that slice before implementation, update this plan with an action-space resize task, and include `docs/ACTION_SPEC.md`, `docs/TENSOR_SPEC.md` if tensor semantics change, `code/digimon-engine-py/src/lib.rs`, and RL env constants in the same commit.

## Task 1: Action Helpers and Stable Selection References

**Files:**

- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/action/space.rs`
- Modify: `code/digimon-engine/src/debug_runner.rs`
- Modify: `code/digimon-engine/src/effect_context/mod.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/source_selection_mask.rs`

- [ ] **Step 1: Add failing action helper tests**

Create `code/digimon-engine/tests/mask_and_tensor/source_selection_mask.rs` with:

```rust
use digimon_engine::action::space::{
    decode_source_select, encode_source_select, ACTION_SPACE_SIZE, SOURCES_PER_FIELD,
    SOURCE_SELECT_END, SOURCE_SELECT_START,
};

#[test]
fn source_select_encoder_round_trips_existing_range() {
    let action = encode_source_select(3, 5).expect("field 3 source 5 fits");
    assert_eq!(action, SOURCE_SELECT_START + 3 * SOURCES_PER_FIELD + 5);
    assert_eq!(decode_source_select(action), (3, 5));
}

#[test]
fn source_select_encoder_rejects_values_outside_existing_range() {
    assert_eq!(encode_source_select(14, 0), None);
    assert_eq!(encode_source_select(0, SOURCES_PER_FIELD), None);
    assert_eq!(SOURCE_SELECT_END as usize, ACTION_SPACE_SIZE);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- source_select_encoder
```

Expected: FAIL with an unresolved import or missing function for `encode_source_select`.

- [ ] **Step 3: Implement action helpers and stable refs**

In `code/digimon-engine/src/action/space.rs`, add below `decode_source_select`:

```rust
pub fn encode_source_select(field: u16, source: u16) -> Option<u16> {
    if field >= MAX_FIELD_SLOTS || source >= SOURCES_PER_FIELD {
        return None;
    }
    Some(SOURCE_SELECT_START + field * SOURCES_PER_FIELD + source)
}
```

In `code/digimon-engine/src/selection.rs`, add these public reference types near the existing selection enums:

```rust
use crate::card_source::CardHandle;
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceSelectionRef {
    pub permanent: PermanentHandle,
    pub field_index: u8,
    pub source_index: u8,
    pub card: CardHandle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BreedingPermanentSelectionRef {
    pub player: u8,
    pub card: CardHandle,
}
```

Extend `SelectionKind` with:

```rust
SourceMulti {
    min: u8,
    max: u8,
    picked: u8,
},
DpBudget {
    remaining_dp: i32,
    picked: u8,
},
BreedingPermanent,
```

In `code/digimon-engine/src/effect_context/mod.rs`, re-export:

```rust
pub use crate::selection::{BreedingPermanentSelectionRef, SourceSelectionRef};
```

In `code/digimon-engine/src/debug_runner.rs`, add deterministic fixture helpers after `place_on_field`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugBreedingPermanent {
    pub player: PlayerId,
    pub card: crate::card_source::CardHandle,
}

impl DebugRunner {
    pub fn new() -> Self {
        Self::builder().start()
    }

    pub fn place_stack(&mut self, player: PlayerId, card_ids: &[&str]) -> PermanentHandle {
        assert!(!card_ids.is_empty(), "place_stack requires at least one card id");
        let top_id = card_ids[card_ids.len() - 1];
        let handle = self.place_on_field(player, top_id, Some(0));

        let mut source_cards = Vec::new();
        for card_id in &card_ids[..card_ids.len() - 1] {
            let data_idx = self
                .game
                .card_data
                .iter()
                .position(|c| c.card_id == *card_id)
                .unwrap_or_else(|| panic!("place_stack: unknown card_id {}", card_id));
            let next_idx = self.game.next_card_index();
            let mut card = CardSource::new(data_idx, player, next_idx);
            card.card_index = next_idx;
            source_cards.push(card);
        }

        let perm = self.game.players[player as usize]
            .battle_area
            .get_mut(handle.index as usize)
            .expect("fresh permanent exists");
        let top = perm.card_sources.pop().expect("top card exists");
        perm.card_sources.extend(source_cards);
        perm.card_sources.push(top);
        handle
    }

    pub fn place_in_breeding(&mut self, player: PlayerId, card_id: &str) -> DebugBreedingPermanent {
        let handle = self.place_on_field(player, card_id, Some(0));
        let perm = self.game.players[player as usize]
            .battle_area
            .remove(handle.index as usize);
        let card = perm.top_card().handle();
        self.game.players[player as usize].breeding_area = Some(perm);
        DebugBreedingPermanent { player, card }
    }

    pub fn force_base_dp(&mut self, card_id: &str, dp: i32) {
        let card = self
            .game
            .card_data
            .iter_mut()
            .find(|c| c.card_id == card_id)
            .unwrap_or_else(|| panic!("force_base_dp: unknown card_id {}", card_id));
        card.dp = Some(dp);
    }

    pub fn top_card(&self, handle: PermanentHandle) -> crate::card_source::CardHandle {
        self.game.players[handle.player as usize].battle_area[handle.index as usize]
            .top_card()
            .handle()
    }
}
```

- [ ] **Step 4: Run helper tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- source_select_encoder
```

Expected: PASS, with both source-selection encoder tests passing.

- [ ] **Step 5: Commit**

```bash
git add code/digimon-engine/src/action/space.rs code/digimon-engine/src/selection.rs code/digimon-engine/src/debug_runner.rs code/digimon-engine/src/effect_context/mod.rs code/digimon-engine/tests/mask_and_tensor/source_selection_mask.rs
git commit -m "feat: add stable selection reference types"
```

## Task 2: Cross-Permanent Source Selection Helper

**Files:**

- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-engine/src/action/mask.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/action/explain.rs`
- Modify: `code/digimon-engine/tests/selection/main.rs`
- Test: `code/digimon-engine/tests/selection/source_multi.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/source_selection_mask.rs`

- [ ] **Step 1: Add failing source-selection tests**

Create `code/digimon-engine/tests/selection/source_multi.rs` with:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_source_select, PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::{EffectContext, SourceSelectionRef};
use digimon_engine::game::GamePhase;
use digimon_engine::selection::SelectionKind;

fn ids(picks: &[SourceSelectionRef]) -> Vec<digimon_engine::CardHandle> {
    picks.iter().map(|p| p.card).collect()
}

#[test]
fn exact_two_sources_can_be_selected_across_own_battle_area() {
    let mut r = DebugRunner::new();
    let p0 = 0;
    let first = r.place_stack(p0, &["SRC-A", "SRC-B", "TOP-A"]);
    let second = r.place_stack(p0, &["SRC-C", "TOP-B"]);
    let first_top = r.top_card(first);
    let second_top = r.top_card(second);

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);
    {
        let mut ctx = EffectContext::new(&mut r.game, first_top, Some(first), p0);
        ctx.select_own_sources(
            "choose two sources",
            2,
            2,
            move |_, source| source.card != first_top && source.card != second_top,
            move |ctx, sources| {
                for source in sources.iter() {
                    ctx.trash_card_source(source.permanent, source.card);
                }
                *picked_slot.lock().unwrap() = sources;
            },
        );
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectSource);
    let sel = r.game.pending_selection.as_ref().expect("source selection");
    assert_eq!(sel.kind, SelectionKind::SourceMulti { min: 2, max: 2, picked: 0 });
    assert!(!sel.valid_action_ids.contains(&PASS));
    assert!(sel.valid_action_ids.contains(&encode_source_select(first.index as u16, 0).unwrap()));
    assert!(sel.valid_action_ids.contains(&encode_source_select(first.index as u16, 1).unwrap()));
    assert!(sel.valid_action_ids.contains(&encode_source_select(second.index as u16, 0).unwrap()));

    r.game
        .resolve_selection(p0, encode_source_select(first.index as u16, 1).unwrap())
        .expect("pick source B");
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::SourceMulti { min: 2, max: 2, picked: 1 }
    );
    assert!(!r.game.pending_selection.as_ref().unwrap().valid_action_ids.contains(&PASS));

    r.game
        .resolve_selection(p0, encode_source_select(second.index as u16, 0).unwrap())
        .expect("pick source C");

    let chosen = picked.lock().unwrap().clone();
    assert_eq!(ids(&chosen).len(), 2);
    assert!(
        r.game.player(p0).trash.iter().any(|c| c.handle() == chosen[0].card),
        "first selected source was trashed by stable handle"
    );
    assert!(
        r.game.player(p0).trash.iter().any(|c| c.handle() == chosen[1].card),
        "second selected source was trashed by stable handle"
    );
}

#[test]
fn up_to_sources_enables_pass_only_after_minimum_is_met() {
    let mut r = DebugRunner::new();
    let p0 = 0;
    let source_stack = r.place_stack(p0, &["SRC-A", "SRC-B", "TOP-A"]);
    let source_stack_top = r.top_card(source_stack);

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);
    {
        let mut ctx = EffectContext::new(&mut r.game, source_stack_top, Some(source_stack), p0);
        ctx.select_own_sources(
            "choose up to two sources",
            1,
            2,
            move |_, source| source.card != source_stack_top,
            move |_, sources| {
                *picked_slot.lock().unwrap() = sources;
            },
        );
    }

    assert!(!r.game.pending_selection.as_ref().unwrap().valid_action_ids.contains(&PASS));
    r.game
        .resolve_selection(p0, encode_source_select(source_stack.index as u16, 0).unwrap())
        .expect("pick one");
    assert!(r.game.pending_selection.as_ref().unwrap().valid_action_ids.contains(&PASS));
    r.game.resolve_selection(p0, PASS).expect("commit early");
    assert_eq!(picked.lock().unwrap().len(), 1);
}
```

Add this line to `code/digimon-engine/tests/selection/main.rs`:

```rust
mod source_multi;
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- source
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- source_select_encoder
```

Expected: FAIL with missing `select_own_sources` or missing `SelectionKind::SourceMulti`.

- [ ] **Step 3: Implement the source-selection helper**

In `code/digimon-engine/src/effect_context/selections.rs`, import:

```rust
use crate::action::space::{encode_source_select, PASS};
use crate::selection::SourceSelectionRef;
```

Add this helper to the main `impl EffectContext<'_>` block:

```rust
pub fn select_own_sources<F, C>(
    &mut self,
    prompt: &str,
    min: u8,
    max: u8,
    filter: F,
    callback: C,
) where
    F: Fn(&Game, SourceSelectionRef) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, Vec<SourceSelectionRef>) + Send + Sync + 'static,
{
    assert!(min <= max, "select_own_sources min must be <= max");
    assert!(max > 0, "select_own_sources max must be > 0");

    let filter = Arc::new(filter);
    let final_callback = Arc::new(Mutex::new(Some(callback)));
    install_source_multi_selection(
        self,
        self.player,
        prompt.to_string(),
        min,
        max,
        Vec::new(),
        filter,
        final_callback,
    );
}
```

Add the private installer below the existing count-capped installer:

```rust
fn source_candidates(
    game: &Game,
    player: u8,
    picked: &[SourceSelectionRef],
    filter: &Arc<dyn Fn(&Game, SourceSelectionRef) -> bool + Send + Sync>,
) -> Vec<(u16, SourceSelectionRef)> {
    let mut out = Vec::new();
    for (field_index, perm) in game.player(player).battle_area.iter().enumerate() {
        if perm.card_sources.len() <= 1 {
            continue;
        }
        for source_index in 0..(perm.card_sources.len() - 1) {
            let card = perm.card_sources[source_index].handle();
            let permanent = PermanentHandle {
                player,
                index: field_index as u8,
            };
            let source_ref = SourceSelectionRef {
                permanent,
                field_index: field_index as u8,
                source_index: source_index as u8,
                card,
            };
            if picked.iter().any(|p| p.card == card) {
                continue;
            }
            if !(filter)(game, source_ref) {
                continue;
            }
            if let Some(action_id) = encode_source_select(field_index as u16, source_index as u16) {
                out.push((action_id, source_ref));
            }
        }
    }
    out
}

fn install_source_multi_selection<C>(
    ctx: &mut EffectContext<'_>,
    selecting_player: u8,
    prompt: String,
    min: u8,
    max: u8,
    picked: Vec<SourceSelectionRef>,
    filter: Arc<dyn Fn(&Game, SourceSelectionRef) -> bool + Send + Sync>,
    final_callback: Arc<Mutex<Option<C>>>,
) where
    C: FnOnce(&mut EffectContext<'_>, Vec<SourceSelectionRef>) + Send + Sync + 'static,
{
    let candidates = source_candidates(ctx.game, selecting_player, &picked, &filter);
    if candidates.is_empty() || picked.len() == max as usize {
        if picked.len() >= min as usize || candidates.is_empty() {
            if let Some(cb) = final_callback.lock().unwrap().take() {
                cb(ctx, picked);
            }
        }
        return;
    }

    let mut valid_action_ids: Vec<u16> = candidates.iter().map(|(action_id, _)| *action_id).collect();
    if picked.len() >= min as usize {
        valid_action_ids.push(PASS);
    }
    let action_to_source = Arc::new(candidates);
    let callback_prompt = prompt.clone();
    let filter_next = Arc::clone(&filter);
    let final_next = Arc::clone(&final_callback);
    let picked_for_pick = picked.clone();

    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    ctx.game.pending_selection = Some(crate::selection::PendingSelection {
        kind: crate::selection::SelectionKind::SourceMulti {
            min,
            max,
            picked: picked.len() as u8,
        },
        selecting_player,
        previous_phase: ctx.game.current_phase,
        valid_action_ids,
        is_optional: picked.len() >= min as usize,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        callback: Box::new(move |game, action_id| {
            let (_, source_ref) = action_to_source
                .iter()
                .find(|(candidate_action, _)| *candidate_action == action_id)
                .expect("source action must have been in valid_action_ids");
            let mut next_picked = picked_for_pick.clone();
            next_picked.push(*source_ref);
            let mut next_ctx =
                EffectContext::new(game, source_ref.card, source_permanent, selecting_player);
            install_source_multi_selection(
                &mut next_ctx,
                selecting_player,
                callback_prompt,
                min,
                max,
                next_picked,
                filter_next,
                final_next,
            );
        }),
        on_decline: Some(Box::new(move |game| {
            if let Some(cb) = final_callback.lock().unwrap().take() {
                let mut next_ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
                cb(&mut next_ctx, picked);
            }
        })),
    });
    ctx.game.current_phase = GamePhase::SelectSource;
}
```

- [ ] **Step 4: Add source-selection mask checks**

Append to `code/digimon-engine/tests/mask_and_tensor/source_selection_mask.rs`:

```rust
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;

#[test]
fn source_multi_mask_only_exposes_selecting_players_pending_actions() {
    let mut r = DebugRunner::new();
    let p0 = 0;
    let p1 = 1;
    let stack = r.place_stack(p0, &["SRC-A", "TOP-A"]);
    let stack_top = r.top_card(stack);
    {
        let mut ctx = EffectContext::new(&mut r.game, stack_top, Some(stack), p0);
        ctx.select_own_sources(
            "pick one source",
            1,
            1,
            move |_, source| source.card != stack_top,
            |_, _| {},
        );
    }

    let p0_mask = build_action_mask(&r.game, p0);
    let p1_mask = build_action_mask(&r.game, p1);
    assert!(p0_mask.iter().any(|v| *v > 0.5), "selecting player sees source action");
    assert!(p1_mask.iter().all(|v| *v == 0.0), "non-selecting player sees empty mask");
    assert_eq!(p0_mask[PASS as usize], 0.0, "exact one source cannot PASS before picking");
}
```

- [ ] **Step 5: Run source-selection tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- source_multi
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- source_multi_mask
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/src/action/mask.rs code/digimon-engine/src/action/decode.rs code/digimon-engine/src/action/explain.rs code/digimon-engine/tests/selection/main.rs code/digimon-engine/tests/selection/source_multi.rs code/digimon-engine/tests/mask_and_tensor/source_selection_mask.rs
git commit -m "feat: add cross-permanent source selection"
```

## Task 3: DSL Source Selection and Source Trashing

**Files:**

- Modify: `code/digimon-dsl/src/step.rs`
- Modify: `code/digimon-dsl/src/compiled.rs`
- Modify: `code/digimon-dsl/src/compile.rs`
- Modify: `code/digimon-engine/src/dsl_cards/bindings.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/selections.rs`
- Modify: `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`
- Test: `code/digimon-engine/tests/dsl/phase2g_select_sources.rs`

- [ ] **Step 1: Add failing DSL test**

Create `code/digimon-engine/tests/dsl/phase2g_select_sources.rs` with:

```rust
use digimon_dsl::compiled::CompiledStep;
use digimon_engine::action::space::encode_source_select;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::runtime::run_compiled_steps;
use digimon_engine::game::GamePhase;
use digimon_engine::selection::SelectionKind;

#[test]
fn select_own_sources_binds_source_refs_and_trashes_selected_sources() {
    let mut runner = DebugRunner::new();
    let p0 = 0;
    let stack_a = runner.place_stack(p0, &["SRC-A", "TOP-A"]);
    let stack_b = runner.place_stack(p0, &["SRC-B", "TOP-B"]);
    let source_card = runner.top_card(stack_a);

    let steps = vec![
        CompiledStep::SelectOwnSources {
            min: 2,
            max: 2,
            bind_as: Some("chosen_sources".to_string()),
            prompt: "Choose two sources".to_string(),
            then: vec![CompiledStep::TrashSelectedSources {
                source_refs: "chosen_sources".to_string(),
            }],
        },
    ];

    run_compiled_steps(&mut runner.game, p0, source_card, &steps).expect("install source prompt");
    assert_eq!(runner.game.current_phase, GamePhase::SelectSource);
    assert_eq!(
        runner.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::SourceMulti { min: 2, max: 2, picked: 0 }
    );

    runner
        .game
        .resolve_selection(p0, encode_source_select(stack_a.index as u16, 0).unwrap())
        .expect("pick source a");
    runner
        .game
        .resolve_selection(p0, encode_source_select(stack_b.index as u16, 0).unwrap())
        .expect("pick source b");

    assert_eq!(runner.game.player(p0).trash.len(), 2);
    assert_eq!(runner.game.player(p0).battle_area[stack_a.index as usize].card_sources.len(), 1);
    assert_eq!(runner.game.player(p0).battle_area[stack_b.index as usize].card_sources.len(), 1);
}
```

- [ ] **Step 2: Run DSL source test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_own_sources_binds_source_refs
```

Expected: FAIL with missing `CompiledStep::SelectOwnSources` or `CompiledStep::TrashSelectedSources`.

- [ ] **Step 3: Add DSL step specs and compiled variants**

In `code/digimon-dsl/src/step.rs`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectOwnSourcesSpec {
    pub min: u8,
    pub max: u8,
    pub bind_as: Option<String>,
    #[serde(default = "default_select_sources_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub then: Vec<StepSpec>,
}

fn default_select_sources_prompt() -> String {
    "Choose source cards".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrashSelectedSourcesSpec {
    pub source_refs: String,
}
```

Add enum arms:

```rust
SelectOwnSources(SelectOwnSourcesSpec),
TrashSelectedSources(TrashSelectedSourcesSpec),
```

Add serializer keys:

```rust
StepSpec::SelectOwnSources(v) => kv!(s, "select_own_sources", v),
StepSpec::TrashSelectedSources(v) => kv!(s, "trash_selected_sources", v),
```

Add deserializer keys:

```rust
"select_own_sources" => StepSpec::SelectOwnSources(map.next_value()?),
"trash_selected_sources" => StepSpec::TrashSelectedSources(map.next_value()?),
```

In `code/digimon-dsl/src/compiled.rs`, add:

```rust
SelectOwnSources {
    min: u8,
    max: u8,
    bind_as: Option<String>,
    prompt: String,
    then: Vec<CompiledStep>,
},
TrashSelectedSources {
    source_refs: String,
},
```

Keep engine-handle binding values in `code/digimon-engine/src/dsl_cards/bindings.rs`; `digimon-dsl` remains schema-only and does not depend on `digimon-engine`.

- [ ] **Step 4: Compile and lower DSL source steps**

In `code/digimon-dsl/src/compile.rs`, add:

```rust
StepSpec::SelectOwnSources(spec) => CompiledStep::SelectOwnSources {
    min: spec.min,
    max: spec.max,
    bind_as: spec.bind_as.clone(),
    prompt: spec.prompt.clone(),
    then: compile_steps(&spec.then, errors),
},
StepSpec::TrashSelectedSources(spec) => CompiledStep::TrashSelectedSources {
    source_refs: spec.source_refs.clone(),
},
```

In `code/digimon-engine/src/dsl_cards/bindings.rs`, add binding values:

```rust
SourceRefs(Vec<crate::selection::SourceSelectionRef>),
SourceRef(crate::selection::SourceSelectionRef),
```

Add accessors:

```rust
pub fn get_source_refs(&self, name: &str) -> Option<Vec<crate::selection::SourceSelectionRef>> {
    match self.get(name) {
        Some(BindingValue::SourceRefs(refs)) => Some(refs.clone()),
        Some(BindingValue::SourceRef(source_ref)) => Some(vec![*source_ref]),
        _ => None,
    }
}
```

In `code/digimon-engine/src/dsl_cards/step/selections.rs`, lower:

```rust
CompiledStep::SelectOwnSources {
    min,
    max,
    bind_as,
    prompt,
    then,
} => {
    let bind_name = bind_as.clone();
    let tail = then.clone();
    ctx.select_own_sources(
        prompt,
        *min,
        *max,
        |_, _| true,
        move |ctx, refs| {
            let mut b = bindings.clone();
            if let Some(name) = &bind_name {
                b.insert_source_refs(name, refs);
            }
            run_tail_preserving_trigger_context(
                ctx,
                trigger_context,
                &Arc::new(tail.clone()),
                &mut b,
                &runtime,
            );
        },
    );
}
```

In `code/digimon-engine/src/dsl_cards/step/zone_moves.rs`, lower:

```rust
CompiledStep::TrashSelectedSources { source_refs } => {
    if let Some(refs) = ctx.bindings().get_source_refs(source_refs) {
        for source_ref in refs {
            ctx.trash_card_source(source_ref.permanent, source_ref.card);
        }
    }
}
```

Add `insert_source_refs` and `get_source_refs` to `Bindings` in this same task so the lowering code above has concrete accessors. Missing bindings produce no mutation and do not block the tail.

- [ ] **Step 5: Run DSL source tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_own_sources_binds_source_refs
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- source_multi
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/bindings.rs code/digimon-engine/src/dsl_cards/step/selections.rs code/digimon-engine/src/dsl_cards/step/zone_moves.rs code/digimon-engine/tests/dsl/phase2g_select_sources.rs
git commit -m "feat: lower DSL source selections"
```

## Task 4: DP-Budget Opponent Permanent Multi-Select

**Files:**

- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-engine/src/action/explain.rs`
- Modify: `code/digimon-engine/tests/selection/main.rs`
- Test: `code/digimon-engine/tests/selection/dp_budget.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/dp_budget_selection_mask.rs`
- Test: `code/digimon-engine/tests/dsl/phase2g_dp_budget.rs`

- [ ] **Step 1: Add failing DP-budget selection tests**

Create `code/digimon-engine/tests/selection/dp_budget.rs` with:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::game::GamePhase;
use digimon_engine::selection::SelectionKind;

#[test]
fn dp_budget_selection_picks_multiple_opponent_digimon_until_pass() {
    let mut r = DebugRunner::new();
    let p0 = 0;
    let p1 = 1;
    let source = r.place_on_field(p0, "SRC", Some(0));
    r.force_base_dp("LOW", 3000);
    r.force_base_dp("MID", 4000);
    r.force_base_dp("HIGH", 8000);
    let low = r.place_on_field(p1, "LOW", Some(0));
    let mid = r.place_on_field(p1, "MID", Some(0));
    let high = r.place_on_field(p1, "HIGH", Some(0));

    let picked = Arc::new(Mutex::new(Vec::new()));
    let picked_slot = Arc::clone(&picked);
    {
        let mut ctx = EffectContext::new(&mut r.game, r.top_card(source), Some(source), p0);
        ctx.select_opponent_permanents_by_dp_budget(
            "delete up to 7000 DP",
            7000,
            0,
            |_, _| true,
            move |_, handles| {
                *picked_slot.lock().unwrap() = handles;
            },
        );
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectBudgeted);
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::DpBudget { remaining_dp: 7000, picked: 0 }
    );
    assert!(r.game.pending_selection.as_ref().unwrap().valid_action_ids.contains(&encode_attack(0, low.index as u16)));
    assert!(r.game.pending_selection.as_ref().unwrap().valid_action_ids.contains(&encode_attack(0, mid.index as u16)));
    assert!(!r.game.pending_selection.as_ref().unwrap().valid_action_ids.contains(&encode_attack(0, high.index as u16)));
    assert!(r.game.pending_selection.as_ref().unwrap().valid_action_ids.contains(&PASS));

    r.game
        .resolve_selection(p0, encode_attack(0, low.index as u16))
        .expect("pick low");
    assert_eq!(
        r.game.pending_selection.as_ref().unwrap().kind,
        SelectionKind::DpBudget { remaining_dp: 4000, picked: 1 }
    );
    r.game
        .resolve_selection(p0, encode_attack(0, mid.index as u16))
        .expect("pick mid");

    assert_eq!(picked.lock().unwrap().as_slice(), &[low, mid]);
}
```

Add this line to `code/digimon-engine/tests/selection/main.rs`:

```rust
mod dp_budget;
```

- [ ] **Step 2: Run DP-budget test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- dp_budget_selection
```

Expected: FAIL with missing `select_opponent_permanents_by_dp_budget` or missing DP fixture helper.

- [ ] **Step 3: Implement DP-budget selection helper**

In `code/digimon-engine/src/effect_context/selections.rs`, add:

```rust
pub fn select_opponent_permanents_by_dp_budget<F, C>(
    &mut self,
    prompt: &str,
    dp_budget: i32,
    min_picks: u8,
    filter: F,
    callback: C,
) where
    F: Fn(&Game, PermanentHandle) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, Vec<PermanentHandle>) + Send + Sync + 'static,
{
    let filter = Arc::new(filter);
    let final_callback = Arc::new(Mutex::new(Some(callback)));
    install_dp_budget_selection(
        self,
        self.player,
        prompt.to_string(),
        dp_budget,
        min_picks,
        Vec::new(),
        filter,
        final_callback,
    );
}
```

Add a private installer that computes candidates from the opponent battle area:

```rust
fn install_dp_budget_selection<C>(
    ctx: &mut EffectContext<'_>,
    selecting_player: u8,
    prompt: String,
    remaining_dp: i32,
    min_picks: u8,
    picked: Vec<PermanentHandle>,
    filter: Arc<dyn Fn(&Game, PermanentHandle) -> bool + Send + Sync>,
    final_callback: Arc<Mutex<Option<C>>>,
) where
    C: FnOnce(&mut EffectContext<'_>, Vec<PermanentHandle>) + Send + Sync + 'static,
{
    let opponent = 1 - selecting_player;
    let mut candidates = Vec::new();
    for (index, perm) in ctx.game.player(opponent).battle_area.iter().enumerate() {
        let handle = PermanentHandle {
            player: opponent,
            index: index as u8,
        };
        if picked.contains(&handle) {
            continue;
        }
        if !(filter)(ctx.game, handle) {
            continue;
        }
        let dp = ctx.game.effective_dp(handle).unwrap_or(0);
        if dp <= remaining_dp {
            candidates.push((crate::action::space::encode_attack(0, index as u16), handle, dp));
        }
    }

    if candidates.is_empty() {
        if picked.len() >= min_picks as usize {
            if let Some(cb) = final_callback.lock().unwrap().take() {
                cb(ctx, picked);
            }
        }
        return;
    }

    let mut valid_action_ids: Vec<u16> = candidates.iter().map(|(action_id, _, _)| *action_id).collect();
    if picked.len() >= min_picks as usize {
        valid_action_ids.push(PASS);
    }
    let candidates = Arc::new(candidates);
    let prompt_next = prompt.clone();
    let filter_next = Arc::clone(&filter);
    let final_next = Arc::clone(&final_callback);
    let picked_next_base = picked.clone();

    let source_card = ctx.source_card;
    let source_permanent = ctx.source_permanent;
    ctx.game.pending_selection = Some(crate::selection::PendingSelection {
        kind: crate::selection::SelectionKind::DpBudget {
            remaining_dp,
            picked: picked.len() as u8,
        },
        selecting_player,
        previous_phase: ctx.game.current_phase,
        valid_action_ids,
        is_optional: picked.len() >= min_picks as usize,
        prompt,
        effect_choices: None,
        source_card,
        source_permanent,
        callback: Box::new(move |game, action_id| {
            let (_, chosen, dp) = candidates
                .iter()
                .find(|(candidate_action, _, _)| *candidate_action == action_id)
                .expect("DP-budget action must have been valid");
            let mut next_picked = picked_next_base.clone();
            next_picked.push(*chosen);
            let mut next_ctx =
                EffectContext::new(game, source_card, source_permanent, selecting_player);
            install_dp_budget_selection(
                &mut next_ctx,
                selecting_player,
                prompt_next,
                remaining_dp - *dp,
                min_picks,
                next_picked,
                filter_next,
                final_next,
            );
        }),
        on_decline: Some(Box::new(move |game| {
            if let Some(cb) = final_callback.lock().unwrap().take() {
                let mut next_ctx =
                    EffectContext::new(game, source_card, source_permanent, selecting_player);
                cb(&mut next_ctx, picked);
            }
        })),
    });
    ctx.game.current_phase = GamePhase::SelectBudgeted;
}
```

- [ ] **Step 4: Add mask test**

Create `code/digimon-engine/tests/mask_and_tensor/dp_budget_selection_mask.rs` with:

```rust
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::{encode_attack, PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;

#[test]
fn dp_budget_mask_exposes_only_candidates_within_remaining_budget() {
    let mut r = DebugRunner::new();
    let p0 = 0;
    let p1 = 1;
    let source = r.place_on_field(p0, "SRC", Some(0));
    r.force_base_dp("LOW", 3000);
    r.force_base_dp("HIGH", 9000);
    let low = r.place_on_field(p1, "LOW", Some(0));
    let high = r.place_on_field(p1, "HIGH", Some(0));

    {
        let mut ctx = EffectContext::new(&mut r.game, r.top_card(source), Some(source), p0);
        ctx.select_opponent_permanents_by_dp_budget("budget", 5000, 0, |_, _| true, |_, _| {});
    }

    let mask = build_action_mask(&r.game, p0);
    assert_eq!(mask[encode_attack(0, low.index as u16) as usize], 1.0);
    assert_eq!(mask[encode_attack(0, high.index as u16) as usize], 0.0);
    assert_eq!(mask[PASS as usize], 1.0);
}
```

- [ ] **Step 5: Add DSL DP-budget lowering test**

Create `code/digimon-engine/tests/dsl/phase2g_dp_budget.rs` with:

```rust
use digimon_dsl::compiled::CompiledStep;
use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::runtime::run_compiled_steps;

#[test]
fn dsl_select_dp_budget_binds_opponent_permanents() {
    let mut runner = DebugRunner::new();
    let p0 = 0;
    let p1 = 1;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.force_base_dp("TARGET", 4000);
    let target = runner.place_on_field(p1, "TARGET", Some(0));

    let steps = vec![CompiledStep::SelectOpponentDpBudget {
        dp_budget: 5000,
        min_picks: 1,
        bind_as: Some("targets".to_string()),
        prompt: "Choose opponents".to_string(),
        then: vec![CompiledStep::DeleteBoundPermanents {
            binding: "targets".to_string(),
        }],
    }];

    run_compiled_steps(&mut runner.game, p0, source_card, &steps).expect("install DP prompt");
    runner
        .game
        .resolve_selection(p0, encode_attack(0, target.index as u16))
        .expect("pick target");

    assert!(runner.game.player(p1).battle_area.is_empty(), "target deleted after bound tail");
}
```

Add `CompiledStep::SelectOpponentDpBudget` and `CompiledStep::DeleteBoundPermanents` in the same files touched for Task 3:

```rust
SelectOpponentDpBudget {
    dp_budget: i32,
    min_picks: u8,
    bind_as: Option<String>,
    prompt: String,
    then: Vec<CompiledStep>,
},
DeleteBoundPermanents {
    binding: String,
},
```

Lower `DeleteBoundPermanents` by reading a `Vec<PermanentHandle>` from bindings and calling `ctx.delete_permanent(handle)` for each handle sorted by descending `(player, index)` so battle-area removals cannot shift unprocessed lower indexes.

- [ ] **Step 6: Run DP-budget tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- dp_budget
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- dp_budget_mask
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_select_dp_budget
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/src/action/explain.rs code/digimon-engine/tests/selection/main.rs code/digimon-engine/tests/selection/dp_budget.rs code/digimon-engine/tests/mask_and_tensor/dp_budget_selection_mask.rs code/digimon-engine/tests/dsl/phase2g_dp_budget.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/bindings.rs code/digimon-engine/src/dsl_cards/step/selections.rs
git commit -m "feat: add DP-budget permanent selection"
```

## Task 5: Breeding Permanent Selection

**Files:**

- Modify: `code/digimon-engine/src/enums.rs`
- Modify: `code/digimon-engine/src/selection.rs`
- Modify: `code/digimon-engine/src/action/space.rs`
- Modify: `code/digimon-engine/src/action/decode.rs`
- Modify: `code/digimon-engine/src/action/explain.rs`
- Modify: `code/digimon-engine/src/effect_context/selections.rs`
- Modify: `code/digimon-engine/tests/selection/main.rs`
- Test: `code/digimon-engine/tests/selection/breeding_permanent.rs`
- Test: `code/digimon-engine/tests/mask_and_tensor/breeding_selection_mask.rs`
- Test: `code/digimon-engine/tests/dsl/phase2g_breeding_selection.rs`

- [ ] **Step 1: Add failing breeding selection tests**

Create `code/digimon-engine/tests/selection/breeding_permanent.rs` with:

```rust
use std::sync::{Arc, Mutex};

use digimon_engine::action::space::encode_breeding_select;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::game::GamePhase;
use digimon_engine::selection::SelectionKind;

#[test]
fn breeding_permanent_selection_targets_breeding_without_fake_battle_handle() {
    let mut r = DebugRunner::new();
    let p0 = 0;
    let source = r.place_on_field(p0, "SRC", Some(0));
    let source_card = r.top_card(source);
    let breeding = r.place_in_breeding(p0, "KING-DRASIL");

    let picked = Arc::new(Mutex::new(None));
    let picked_slot = Arc::clone(&picked);
    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.select_own_breeding_permanent("choose breeding", |_, _| true, move |_, target| {
            *picked_slot.lock().unwrap() = Some(target);
        });
    }

    assert_eq!(r.game.current_phase, GamePhase::SelectBreedingPermanent);
    assert_eq!(r.game.pending_selection.as_ref().unwrap().kind, SelectionKind::BreedingPermanent);
    assert!(
        r.game
            .player(p0)
            .battle_area
            .iter()
            .all(|perm| perm.top_card().handle() != breeding.card)
    );

    r.game
        .resolve_selection(p0, encode_breeding_select(p0).expect("p0 breeding action"))
        .expect("pick breeding");

    let selected = picked.lock().unwrap().expect("selection callback fired");
    assert_eq!(selected.player, p0);
    assert_eq!(selected.card, breeding.card);
}
```

Add this line to `code/digimon-engine/tests/selection/main.rs`:

```rust
mod breeding_permanent;
```

- [ ] **Step 2: Run breeding selection test to verify it fails**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_permanent_selection
```

Expected: FAIL with missing `GamePhase::SelectBreedingPermanent`, `encode_breeding_select`, or `select_own_breeding_permanent`.

- [ ] **Step 3: Implement breeding selection range and phase**

In `code/digimon-engine/src/action/space.rs`, add:

```rust
pub const BREEDING_SELECT_PLAYER_0: u16 = BREEDING_TARGET;
pub const BREEDING_SELECT_PLAYER_1: u16 = BREEDING_TARGET + 1;

pub fn encode_breeding_select(player: u8) -> Option<u16> {
    match player {
        0 => Some(BREEDING_SELECT_PLAYER_0),
        1 => Some(BREEDING_SELECT_PLAYER_1),
        _ => None,
    }
}
```

In `code/digimon-engine/src/enums.rs`, add `SelectBreedingPermanent` to `GamePhase`, `is_selection_phase`, and `py_name`:

```rust
GamePhase::SelectBreedingPermanent => "SelectBreedingPermanent",
```

In `code/digimon-engine/src/action/decode.rs`, route it with the other selection phases:

```rust
GamePhase::SelectBreedingPermanent => self.resolve_selection(self.current_player, action_id),
```

Call `resolve_selection` with the pending selection's `selecting_player`, matching the routing used by the other selection phases in `code/digimon-engine/src/action/decode.rs`.

- [ ] **Step 4: Implement breeding selection helper**

In `code/digimon-engine/src/effect_context/selections.rs`, add:

```rust
pub fn select_own_breeding_permanent<F, C>(
    &mut self,
    prompt: &str,
    filter: F,
    callback: C,
) where
    F: Fn(&Game, BreedingPermanentSelectionRef) -> bool + Send + Sync + 'static,
    C: FnOnce(&mut EffectContext<'_>, BreedingPermanentSelectionRef) + Send + Sync + 'static,
{
    let Some(card) = self
        .game
        .player(self.player)
        .breeding_area
        .as_ref()
        .map(|p| p.top_card().handle())
    else {
        return;
    };
    let selection_ref = BreedingPermanentSelectionRef {
        player: self.player,
        card,
    };
    if !filter(self.game, selection_ref) {
        return;
    }
    let Some(action_id) = crate::action::space::encode_breeding_select(self.player) else {
        return;
    };
    let selecting_player = self.player;
    let source_card = self.source_card;
    let source_permanent = self.source_permanent;
    self.game.pending_selection = Some(crate::selection::PendingSelection {
        kind: crate::selection::SelectionKind::BreedingPermanent,
        selecting_player,
        previous_phase: self.game.current_phase,
        valid_action_ids: vec![action_id],
        is_optional: false,
        prompt: prompt.to_string(),
        effect_choices: None,
        source_card,
        source_permanent,
        callback: Box::new(move |game, _| {
            let mut ctx = EffectContext::new(game, source_card, source_permanent, selecting_player);
            callback(&mut ctx, selection_ref);
        }),
        on_decline: None,
    });
    self.game.current_phase = GamePhase::SelectBreedingPermanent;
}
```

- [ ] **Step 5: Add breeding mask test**

Create `code/digimon-engine/tests/mask_and_tensor/breeding_selection_mask.rs` with:

```rust
use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::encode_breeding_select;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::effect_context::EffectContext;

#[test]
fn breeding_selection_mask_exposes_only_breeding_select_action() {
    let mut r = DebugRunner::new();
    let p0 = 0;
    let source = r.place_on_field(p0, "SRC", Some(0));
    let source_card = r.top_card(source);
    r.place_in_breeding(p0, "BREED");
    {
        let mut ctx = EffectContext::new(&mut r.game, source_card, Some(source), p0);
        ctx.select_own_breeding_permanent("pick breeding", |_, _| true, |_, _| {});
    }

    let mask = build_action_mask(&r.game, p0);
    let legal: Vec<usize> = mask
        .iter()
        .enumerate()
        .filter_map(|(idx, value)| if *value > 0.5 { Some(idx) } else { None })
        .collect();
    assert_eq!(legal, vec![encode_breeding_select(p0).unwrap() as usize]);
}
```

- [ ] **Step 6: Add DSL breeding lowering test**

Create `code/digimon-engine/tests/dsl/phase2g_breeding_selection.rs` with:

```rust
use digimon_dsl::compiled::CompiledStep;
use digimon_engine::action::space::encode_breeding_select;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::runtime::run_compiled_steps;

#[test]
fn dsl_select_breeding_permanent_binds_target() {
    let mut runner = DebugRunner::new();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "KING-DRASIL");

    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose breeding".to_string(),
        then: vec![CompiledStep::GainMemory { amount: 1 }],
    }];

    run_compiled_steps(&mut runner.game, p0, source_card, &steps).expect("install breeding prompt");
    runner
        .game
        .resolve_selection(p0, encode_breeding_select(p0).unwrap())
        .expect("pick breeding");

    assert_eq!(runner.game.memory, 1);
}
```

Add `CompiledStep::SelectOwnBreedingPermanent { bind_as, prompt, then }`, parser support, and lowering that binds `BindingValue::BreedingPermanentRef` then runs `then`.

- [ ] **Step 7: Run breeding tests**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- breeding_permanent
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor -- breeding_selection_mask
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- dsl_select_breeding
```

Expected: PASS.

- [ ] **Step 8: Commit**

```bash
git add code/digimon-engine/src/enums.rs code/digimon-engine/src/selection.rs code/digimon-engine/src/action/space.rs code/digimon-engine/src/action/decode.rs code/digimon-engine/src/action/explain.rs code/digimon-engine/src/effect_context/selections.rs code/digimon-engine/tests/selection/main.rs code/digimon-engine/tests/selection/breeding_permanent.rs code/digimon-engine/tests/mask_and_tensor/breeding_selection_mask.rs code/digimon-engine/tests/dsl/phase2g_breeding_selection.rs code/digimon-dsl/src/step.rs code/digimon-dsl/src/compiled.rs code/digimon-dsl/src/compile.rs code/digimon-engine/src/dsl_cards/bindings.rs code/digimon-engine/src/dsl_cards/step/selections.rs
git commit -m "feat: add breeding permanent selection"
```

## Task 6: Existing Selection Regression Gates

**Files:**

- Modify: `code/digimon-engine/tests/selection/kinds.rs`
- Modify: `code/digimon-engine/tests/selection/behavioral_end_to_end.rs`
- Modify: `code/digimon-engine/tests/dsl/phase2e_select_material.rs`
- Modify: `code/digimon-engine/tests/dsl/phase2e_select_ordered_permutation.rs`
- Modify: `code/digimon-engine/tests/dsl/phase2f3_as_selecting_player.rs`

- [ ] **Step 1: Add empty-inner-selection tail regression**

Append these imports and the test to `code/digimon-engine/tests/dsl/phase2e_select_material.rs`:

```rust
use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_cards::runtime::run_compiled_steps;

#[test]
fn empty_inner_select_material_runs_outer_tail() {
    let mut runner = DebugRunner::new();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);

    let steps = vec![
        CompiledStep::SelectMaterial {
            of_permanent: "missing_perm".to_string(),
            bind_as: Some("material".to_string()),
            prompt: "Pick material".to_string(),
            then: vec![CompiledStep::GainMemory { amount: 2 }],
        },
        CompiledStep::GainMemory { amount: 3 },
    ];

    run_compiled_steps(&mut runner.game, p0, source_card, &steps).expect("run steps");
    assert_eq!(runner.game.memory, 3, "inner empty selection no-ops and outer tail continues");
}
```

- [ ] **Step 2: Run empty-inner-selection regression**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- empty_inner_select_material_runs_outer_tail
```

Expected: PASS. The required behavior is that an empty inner selection returns control to the outer continuation and does not drop the remaining steps.

- [ ] **Step 3: Run existing representative selection gates**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- effect_choice
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- opponent_selector
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- ordered
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection -- union_zone
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_ordered_permutation_empty_runs_tail_synchronously
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- as_selecting_player
```

Expected: PASS for all commands. These commands pin effect-choice branch selection, opponent-as-selecting-player, ordered permutation, union-zone hand-or-trash selection, ordered empty-tail behavior, and DSL selector routing.

- [ ] **Step 4: Commit**

```bash
git add code/digimon-engine/tests/selection/kinds.rs code/digimon-engine/tests/selection/behavioral_end_to_end.rs code/digimon-engine/tests/dsl/phase2e_select_material.rs code/digimon-engine/tests/dsl/phase2e_select_ordered_permutation.rs code/digimon-engine/tests/dsl/phase2f3_as_selecting_player.rs
git commit -m "test: lock selection regression gates"
```

## Task 7: Documentation, Trackers, and Full Verification

**Files:**

- Modify: `docs/ACTION_SPEC.md`
- Modify: `docs/RUST_ENGINE_GAPS.md`
- Modify: `qa/archetype-qa/engine-gaps.md`
- Modify: `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`

- [ ] **Step 1: Document action-space decision**

Add this section to `docs/ACTION_SPEC.md` near the action-space table:

```markdown
### Selection Primitive Reuse

Group 2 selection primitives reuse existing action ranges and keep `ACTION_SPACE_SIZE = 2168`.

- Cross-permanent source selections use `SOURCE_SELECT_START..SOURCE_SELECT_END`.
- Up-to-N source selections expose `PASS` only after the minimum pick count is satisfied.
- DP-budget permanent selections reuse field-target action IDs during `SelectBudgeted`.
- Breeding permanent selections use phase-scoped breeding selection IDs only while `SelectBreedingPermanent` is pending.

Any future expansion that requires more source slots, more breeding targets, or additional simultaneous selection surfaces must update this document, Rust constants, PyO3 constants, and RL environment constants in the same change.
```

- [ ] **Step 2: Update gap trackers**

In `docs/RUST_ENGINE_GAPS.md`, move these entries to resolved or narrow them to any uncovered card-specific blocker:

```markdown
- `G-ROCKS-SOURCE-SELECTION-DSL`: resolved by cross-permanent `select_own_sources`, source-ref bindings, and `trash_selected_sources`.
- `G-MULTI-SELECT-OPP-DP-SUM`: resolved by `select_opponent_permanents_by_dp_budget`.
- `G-BREEDING-PERMANENT-SELECTION`: resolved by `select_own_breeding_permanent` without fake battle-area handles.
- `G-SELECT-EMPTY-OUTER-TAIL`: covered by DSL empty-selection tail regression tests.
```

In `qa/archetype-qa/engine-gaps.md`, update Rocks and Royal Knights blockers with the exact passing test names from this plan.

- [ ] **Step 3: Mark parent roadmap task complete**

In `docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md`, mark Task 3 steps complete:

```markdown
- [x] **Step 1: Create the child plan file**
- [x] **Step 2: Require an action-space decision record**
- [x] **Step 3: Define selection slices**
- [x] **Step 4: Require test commands**
- [x] **Step 5: Commit the child plan**
```

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo test --manifest-path code/digimon-engine/Cargo.toml --test selection
cargo test --manifest-path code/digimon-engine/Cargo.toml --test mask_and_tensor
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- phase2g
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_material
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_ordered
cargo test --manifest-path code/digimon-engine/Cargo.toml --test dsl -- select_union
```

Expected: PASS for all commands.

- [ ] **Step 5: Commit docs and tracker updates**

```bash
git add docs/ACTION_SPEC.md docs/RUST_ENGINE_GAPS.md qa/archetype-qa/engine-gaps.md docs/superpowers/plans/2026-04-29-archetype-engine-dsl-gap-roadmap.md
git commit -m "docs: update selection primitive gaps"
```

## Self-Review

Spec coverage:

- Cross-permanent count-capped source selection is covered by Tasks 1-3.
- `G-ROCKS-SOURCE-SELECTION-DSL` is covered by Task 3.
- `G-MULTI-SELECT-OPP-DP-SUM` is covered by Task 4.
- Ordered permutation, effect-choice branch selection, opponent-as-selecting-player, union-zone selection, and empty inner-selection tail behavior are covered by Task 6 regression gates.
- `G-BREEDING-PERMANENT-SELECTION` is covered by Task 5.
- Action-mask visibility is covered in Tasks 1, 2, 4, and 5.

Placeholder scan:

- This plan contains concrete file paths, test names, commands, expected outcomes, and code snippets for each implementation slice.

Type consistency:

- `SourceSelectionRef` is introduced in Task 1 and used by Tasks 2 and 3.
- `BreedingPermanentSelectionRef` is introduced in Task 1 and used by Task 5.
- `SelectionKind::SourceMulti`, `SelectionKind::DpBudget`, and `SelectionKind::BreedingPermanent` are introduced before helper tests assert on them.
