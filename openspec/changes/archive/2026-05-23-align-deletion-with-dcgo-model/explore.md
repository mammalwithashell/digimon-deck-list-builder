# Explore — align Rust deletion semantics with DCGO's printed-rules model

**Status:** exploration only (no engine code, no proposal yet)
**Date:** 2026-05-23
**Trigger:** generalist pilot pretraining surfaced the nested-deferred-deletion panic (`G-DELETION-RESUME-NESTED`) — third sibling in the single-outstanding-invariant bug family. ~21+ panics/30-min run, ~0.7% of training samples lost via the Python-side crash wrapper.

> **Scope.** The user dispatched this as exploration. The deliverable is this document — design considerations, surface decisions, risks, alternatives, and a target-scope recommendation. **No engine changes, no card scripts, no test edits.** If the recommendation lands well, the follow-up is `opsx:propose`.

---

## 1. Context — three views of the same fact

### 1a. The bug as it fires in training

`commit_permanent_deletion` ([combat.rs:3318](../../../code/digimon-engine/src/combat.rs:3318)) enqueues `OnDeletion` and drains synchronously. If a handler parks a `pending_selection` (e.g. printed `<Save>` asks for a Tamer), the rest of the deletion sequence is stashed in a single-occupancy slot:

```rust
if self.pending_selection.is_some() {
    debug_assert!(
        self.pending_deletion_resume.is_none(),
        "nested deferred deletion not supported (single-outstanding invariant)"
    );
    self.pending_deletion_resume = Some((handle, deleted_top_card));
    return;
}
```

The mirror site lives at [replacement.rs:1381](../../../code/digimon-engine/src/replacement.rs:1381) for the no-replace path. The slot is `Option<(PermanentHandle, Option<CardHandle>)>`.

The panic fires when an `OnDeletion` handler triggers ANOTHER deletion (or that deletion's path eventually reaches a fresh `OnDeletion`-parking handler) before the first `pending_deletion_resume` is consumed. Today's training cards reach this through:

- A deletion handler that "deletes another permanent" (own-effect cascade).
- An `OnAnyDeletion` observer fired by a sibling that's itself triggering deletion.
- A bundled trigger order (`SelectionKind::TriggerOrder`) where Save+something else both run synchronously and the second sibling triggers a nested delete.

### 1b. The architectural shape

There are at least four siblings in the "single-outstanding-state-while-selection-parked" family on `Game`:

| Slot | Type | Cardinality | Comment in source |
|------|------|-------------|-------------------|
| `parked_replacement` | `Option<ParkedReplacement>` | single | "the dispatcher debug_asserts on duplicate install" |
| `pending_deletion_resume` | `Option<(handle, top_card)>` | single | "if this assumption ever breaks, replace with a stack" |
| `pending_post_deletion_replays` | `Vec<(player, card)>` | **multi** | already a stack; coexists with the above |
| `dsl_outer_tail` | `Option<(steps, bindings, runtime)>` | single | "future change that allows nested parks will need stack" |

Three are single-slot with `debug_assert!`, one is already a stack. The user's panic is the consequence of an observed nested case the substrate doesn't support.

### 1c. DCGO's model — same problem, different shape

DCGO doesn't have this problem because it doesn't have this shape. `DestroyPermanentsClass.Destroy()` ([CardController.cs:3692](../../../DCGO/Assets/Scripts/Script/CardController.cs:3692)) is a *batch* operation across an entire kill list, with two-phase event stacking:

```
1. Filter targets → mark willBeRemoveField=true on every survivor
2. autoProcessing_CutIn.StackSkillInfos(WhenPermanentWouldBeDeleted)
3. autoProcessing_CutIn.StackSkillInfos(WhenRemoveField)
4. autoProcessing_CutIn.TriggeredSkillProcess()         ← cut-in DRAIN
5. Re-filter (some perms may have been cancelled)
6. autoProcessing.StackSkillInfos(OnDestroyedAnyone)    ← NOT drained here
7. autoProcessing.StackSkillInfos(OnLeaveFieldAnyone)   ← NOT drained here
8. For each survivor: snapshot DPJustBeforeRemoveField, Level…, Cost…,
   CardNamesJustBeforeRemoveField, CardTraitsJustBeforeRemoveField,
   and per CardSource: PermanentJustBeforeRemoveField = permanent
9. For each survivor: DestroyPermanentEffect, DiscardEvoRoots,
   RemoveField, AddTrashCard(topCard)
10. Clear willBeRemoveField on all
   — exit Destroy(); outer autoProcessing later drains OnDestroyedAnyone
```

Two crucial properties:

- **Trash happens BEFORE the OnDeletion drain.** When Save fires, its predicate `IsTopCardInTrashOnDeletion` ([OnDeletion.cs:144](../../../DCGO/Assets/Scripts/Script/CardEffectCommons/CanUseEffects/OnDeletion.cs:144)) literally requires `IsExistOnTrash(TopCard)`. Save retrieves the card from trash via `AddDigivolutionCardsBottom`. The card is gone from the field.
- **Cross-permanent identity carried via snapshots.** Every `CardSource` under a dying permanent gets `PermanentJustBeforeRemoveField = permanent` and the dying perm gets `DPJustBeforeRemoveField`/`LevelJustBeforeRemoveField`/etc. These let post-trash handlers answer questions like "what was my DP just before I died?" and "did that card and I belong to the same stack?" (`IsTopCardSamePermanent`, [OnDeletion.cs:169](../../../DCGO/Assets/Scripts/Script/CardEffectCommons/CanUseEffects/OnDeletion.cs:169)) without needing the perm to still exist.

Rust today has neither property:

- `Keyword::Save` ([keyword_effects.rs:518](../../../code/digimon-engine/src/cards/keyword_effects.rs:518)) reaches into `players[owner].battle_area.get(subject.index)` for the live carrier and lifts the top card off `card_sources` in-place. Save's body comment: *"The card hasn't moved to trash yet (deletion is paused on this trigger); when the callback fires, the card is still in the carrier's `card_sources`."*
- `Keyword::Fortitude` ([keyword_effects.rs:743](../../../code/digimon-engine/src/cards/keyword_effects.rs:743)) reads `perm.card_sources.len() >= 2` while the carrier is still on field, then stashes the replay in `pending_post_deletion_replays` to fire post-finalize. This is a **partial** DCGO model — the replay timing matches DCGO, but the gate-check timing doesn't.
- `Keyword::Partition` ([keyword_effects.rs:743+](../../../code/digimon-engine/src/cards/keyword_effects.rs:743) onward) — similar pattern; same `pending_post_deletion_replays` slot.

The existing `pending_post_deletion_replays` slot is in fact a *retrofit* — its block comment explicitly explains it exists because Rust fires OnDeletion before trash, which means listeners that need post-trash semantics need a side-channel hook. There are already multiple workaround patches stacking up around the timing mismatch.

---

## 2. The bug family — why this keeps happening

```
                   ┌─────────────────────────────┐
                   │   single-outstanding slot   │
                   │   on Game while a player    │
                   │   selection is parked       │
                   └──────────────┬──────────────┘
                                  │
        ┌─────────────────┬───────┴───────┬─────────────────┐
        ▼                 ▼               ▼                 ▼
parked_replacement  pending_deletion  dsl_outer_tail  pending_post_
   (Option)         _resume (Option)    (Option)      deletion_replays
        │                 │               │            (Vec — STACK)
        │            ←—— PANICS HERE ——→  │
        │                 │               │
   "asserted          "asserted on    "asserted on
    on duplicate"      duplicate"      duplicate"
```

Each `debug_assert!` is a hardcoded "the system can't be in this state." Each was true at write-time and stops being true the first time a card combo / training rollout finds the corner. The Vec slot at the right exists because someone hit the corner once and refactored. That refactor never propagated to the other three slots.

The recurring symptom is the same: **one substrate slot, multiple coexisting effects can need to occupy it, panic on duplicate.** Three plausible underlying causes:

1. **Synchronous OnDeletion drain.** Rust enqueues *and* drains in the same call frame. The drain can re-enter the same code path before the outer frame unwinds. DCGO doesn't do this — its OnDestroyedAnyone is stacked, not drained, until an outer checkpoint.
2. **Permanent handles as positional indices.** `PermanentHandle { player, index }`. Any deletion shifts later indices. That's *the* reason Save needs to defer — the parked selection's `valid_action_ids` were computed against the pre-shift layout. The deferral is a workaround for the positional encoding.
3. **No conceptual "deletion batch."** `delete_permanent_with_effects(handle)` is the only API. Battles, multi-target effects, simultaneous deaths all decompose into N sequential calls. DCGO has a first-class batch concept (`DestroyPermanentsClass`); the unit of "what just died" is a list.

Each cause has a different remediation cost and reach.

---

## 3. The decision space

I'm going to surface the questions rather than pre-answer them. Each is a real fork; the right call depends on how much engine churn the team wants to take in one bite.

### Q1. Does the trash move *before* or *after* OnDeletion drains?

The crux. Today: after (carrier still in `battle_area` during the OnDeletion handler). DCGO: before (top card already in trash when handlers fire).

| | Trash-before-drain (DCGO) | Trash-after-drain (Rust today) |
|--|----------------------------|-------------------------------|
| Save's `self_card` lookup | Walk trash | Read `perm.card_sources.last()` |
| Save's "carrier on field" assertion | False | True |
| Fortitude's gate check | Snapshot needed (carrier is gone) | Live `perm.card_sources.len()` |
| `OnAnyDeletion` scope | Includes the just-dead carrier's snapshot effects? | Already excludes them — that's why `pending_post_deletion_replays` exists |
| OnDeletion handler reads "this Digimon's DP/level/etc." | Snapshot | Live `effective_dp(handle)` |
| Index-shift bug class (the original reason for `pending_deletion_resume`) | Gone — the deleted perm is already removed when the selection parks | Live |

The whole architecture pivots on this answer.

### Q2. What is the "pre-removal snapshot" type, and where does it live?

DCGO attaches snapshots to two places:

- **The permanent itself** (about to die): `DPJustBeforeRemoveField`, `LevelJustBeforeRemoveField`, `CostJustBeforeRemoveField`, `CardNamesJustBeforeRemoveField`, `CardTraitsJustBeforeRemoveField`. But the permanent is gone after Destroy() — so these snapshots survive on each `CardSource` underneath via `PermanentJustBeforeRemoveField = permanent` (a reference). The "permanent" object is a managed C# class; references to it stay valid even after it's no longer in any field collection.

- **Each CardSource**: `PermanentJustBeforeRemoveField`. So a digi source that lived under a dying perm carries an outgoing pointer back to the (now-orphaned) permanent object.

Rust's data model is the inverse — `Permanent` is owned `inline` in `Player::battle_area: Vec<Permanent>`, removed via `Vec::remove`. No persistent identity for a "just-died" permanent exists today. Options:

- **(2a) Arc-wrap the permanent.** Largest change. Every `card_sources` entry's `PermanentJustBeforeRemoveField` would be `Option<Arc<Permanent>>`. Touches a lot of ownership.
- **(2b) Snapshot struct.** Already exists in part: `crate::trigger_context::DeletedObjectSnapshot` carries former_controller, top_card, card_kind, traits, level, dp, cause. Today it's built *inside* `finalize_permanent_deletion_with_event_card` and threaded onto OnAnyDeletion. Could be promoted to "built once at start of deletion, attached to a `DeletionRecord` for the duration, drained alongside the trigger drain."
- **(2c) Per-deletion-record in a Game-side queue.** `Game.active_deletion_batch: Option<DeletionBatch>` where DeletionBatch holds `Vec<DeletionRecord>` and each record holds the snapshot. Handlers query through the batch context for "what was my predecessor."

**(2b) is the minimum required for DCGO semantics; (2c) is the cleaner home if multi-target batching also lands.**

### Q3. Where does the OnDeletion drain happen — and at what granularity?

Today: per-permanent, synchronous, inside `commit_permanent_deletion`. DCGO: per-batch, deferred to the next `autoProcessing` checkpoint outside `Destroy()`.

This is the part the user hinted at with "deferred-drain infrastructure" (`draining_deferred`, `enter_deferred_drain`, `exit_deferred_drain_and_flush`, `maybe_drain_effect_queue`). **That infrastructure does not exist in this branch.** I grepped — no matches. The user is describing a *target* substrate, not a landed one. Reads of `fire_on_play` / `fire_on_place_security` / `fire_on_attack` confirm they all call `drain_effect_queue()` synchronously. The deferred-drain abstraction would be new work.

If it lands, the question becomes: how wide is its scope?

- **(3a) Wrap only the deletion batch.** `delete_permanents_batch(handles)` enters deferred-drain, marks willBeRemove on all, fires replacement window, batch-stacks OnDeletion + OnLeaveField, snapshots, trashes all, exits deferred-drain and flushes the OnDeletion drain. Closest DCGO parity. Adds a new top-level API.
- **(3b) Make it a generic scope.** Every "complex multi-phase effect that wants to stack triggers without draining" wraps itself in `enter_deferred_drain` … `exit`. Fire sites that already drain inline (the `fire_*` helpers) keep doing that — this is purely additive. The Vec slot at line 519 of game.rs (`pending_post_deletion_replays`) becomes the model for many future slots.
- **(3c) Replace the synchronous drain entirely.** Every `drain_effect_queue()` becomes "request a drain at the next safe point." Largest churn. The current sync-drain pattern is load-bearing in a hundred places.

**(3a) is the minimum; (3c) is over-reach.**

### Q4. What is the right unit of mass-delete callers?

Today there is no batch API. Callers:

| Site | Cardinality | Notes |
|------|-------------|-------|
| `resolve_battle` ([combat.rs:3134](../../../code/digimon-engine/src/combat.rs:3134)) | 1 or 2 (mutual destruction) | The clearest case — both die simultaneously per DCGO |
| Card effects ("delete N permanents") | N (DSL `DeleteBoundPermanents` already iterates) | Index sorting at [permanent_mutations.rs:36-37](../../../code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs:36) is already a workaround for the index-shift bug |
| Security check (DP→0 from security skill) | 1 | The "rare; defensive coverage" Partition cause |
| Cost payment (`delete_permanent_with_cause(Cost)`) | 1 | Trashing self for digivolve / cost |
| Replacement substitute (e.g., `Decoy` redirecting deletion to self) | 1 (recursive) | The depth-guarded recursion at [combat.rs:3290](../../../code/digimon-engine/src/combat.rs:3290) |

Battle tie at [combat.rs:3146-3163](../../../code/digimon-engine/src/combat.rs:3146) is already *manually* sequenced (delete defender first, then attacker if handle still valid). That's the cleanest demo of where a `delete_permanents_batch([defender, attacker])` would be a clarifying primitive. The "delete N permanents" effect family (Partition's replays excluded — those are *plays* not deletes) is the other clear case.

### Q5. Does Save's predicate change shape?

Today Save's body says, paraphrasing: "find me in `players[owner].battle_area[subject.index].card_sources.last()`; lift me out via `place_card_under_permanent_bottom`."

Under DCGO model: "I'm in trash, find me there; place me under the chosen Tamer."

`place_card_under_permanent_bottom` already calls `remove_card_from_any_zone` (per the doc comment at keyword_effects.rs:570-582 — "the zone-walker finds it in the carrier's `card_sources`" — but the function name is "from_any_zone," strongly suggesting trash is one of the search zones). If the helper already walks trash, the behavioral change is *just* the predicate shift — `self_card` is captured before the deletion (its handle is stable), and the lookup site goes wherever the card actually is at fire time.

**This is small surface area. The hard part isn't Save's body — it's the test churn.**

### Q6. How do replacement effects (`<Decoy>`, `<Barrier>`) interact with DCGO's two-stage cut-in?

DCGO's `Destroy()` runs the replacement window (`WhenPermanentWouldBeDeleted`, `WhenRemoveField`) as a *batched* CutIn drain at step 4, before the trash loop at step 9. A replacement that cancels removal sets `willBeRemoveField = false`; the re-filter at step 5 drops it from the kill list.

Rust today runs the replacement window *per-permanent* in `delete_permanent_with_cause` ([combat.rs:3214-3299](../../../code/digimon-engine/src/combat.rs:3214)). Each permanent goes through `try_replace(WhenWouldLeaveBattleArea)` then `try_replace(WhenWouldBeDeleted)` synchronously, then commits.

If batching lands, the replacement window becomes batched too. Two sub-questions:

- **(6a) Are replacement substitutions allowed to add to the kill list mid-batch?** Partition-style: deleting A substitutes into deleting B. DCGO doesn't allow mid-batch growth — the kill list is frozen at step 1. New deletions caused by replacement substitution would go through a fresh `Destroy()` call (recursive). Rust today goes through `delete_permanent_with_cause` recursion ([combat.rs:3290](../../../code/digimon-engine/src/combat.rs:3290)). The depth guard at line 3289 is a current safety net.
- **(6b) Decoy redirecting to self.** Decoy on permanent A is fired when permanent B would be deleted; A is added to the kill list, B is dropped. DCGO handles this as a CutIn-stack mutation during step 4. Rust today: the Decoy handler calls `rctx.substitute(...)` which routes through `delete_permanent_with_cause(A, cause)` — synchronous recursion. Under a batch model, "substitute" might need to mutate the active batch's kill list rather than recurse. Unclear without prototyping.

### Q7. Does the `pending_post_deletion_replays: Vec<...>` slot survive, get absorbed, or get parallel-treated?

The slot ([game.rs:562](../../../code/digimon-engine/src/game.rs:562)) is already DCGO-shaped — it stashes work to fire AFTER trash. Under a full DCGO batch model, Fortitude and Partition no longer need a separate slot: their bodies could just be "play me from trash" / "play N picks from trash," running in the normal OnDeletion drain that itself runs post-trash.

So either:
- **Absorbed.** Fortitude and Partition rewrite to read snapshot+trash instead of pre-removal `card_sources.len()`; `pending_post_deletion_replays` slot deleted.
- **Survives.** Slot stays for any "I have a side-channel of work to flush at finalize-time" pattern. Cleaner if we're going for minimal scope.

I lean toward absorbed *if* a full DCGO batch lands, *survives* if only Save changes. (This is one place to consciously decide rather than drift.)

---

## 4. The three named alternatives

### Option A — stop-gap stack

Change `pending_deletion_resume` from `Option<...>` to `Vec<...>`. Push on park, pop LIFO on resume.

```rust
pub(crate) pending_deletion_resume: Vec<(
    PermanentHandle,
    Option<CardHandle>,
)>,
```

Both park sites push; both resume sites pop. The `debug_assert!` becomes a debug-only depth bound. Total change: maybe 40-60 lines including doc updates.

**What it solves.** The panic. Nested deletions that park no longer trip the assert.

**What it doesn't solve.**
- Save still fires pre-trash (Rust model preserved).
- The behavioral test assertions about "carrier still on field while Tamer-pick parked" stay true.
- The architectural mismatch with DCGO — the long-running source of small parity bugs — is unchanged.
- The single-outstanding-invariant pattern is still load-bearing on three other slots (`parked_replacement`, `dsl_outer_tail`, …). The next time training finds a corner in one of those, this conversation happens again.
- The `pending_post_deletion_replays` workaround slot stays. Future post-trash-detection keywords (e.g., something that reads "this Digimon's DP just before deletion") need yet another bespoke channel.

**Risk.** Lowest in the short term. Highest of the three in the long term — it's an accumulating-tech-debt fix that doesn't address why the family of bugs exists.

### Option B — full DCGO batch

Introduce `delete_permanents_batch(handles)` as the primary multi-target API. Rebuild the per-permanent path on top:

```
delete_permanents_batch:
  1. Filter (zone-walker valid, not protected by CanNotBeAffected)
  2. Mark all targets → enter "deletion batch in flight"
  3. enter_deferred_drain(); enqueue WhenWouldBeDeleted; flush; exit
  4. Re-filter (some may have been redirected/cancelled)
  5. Snapshot each survivor's DP/level/cost/names/traits
       — attach to surviving DeletionRecord-per-perm
  6. enter_deferred_drain():
       - Linked-card cascade per perm
       - Trash top + sources for each perm
       - enqueue OnDeletion for each (key by snapshot, not live handle)
       - enqueue OnAnyDeletion / OnLeaveField with snapshots
       - flush
       - drain post-deletion replays
       - exit
```

Save's body becomes "find me in trash via my snapshot handle, place me under Tamer." Fortitude becomes "look up my snapshot.source_count; if ≥2, play me from trash." Partition becomes "pick N from snapshot.digisources, play them." All three lose their `pending_post_deletion_replays` dependency.

**What it solves.**
- The panic (multiple deletions in one batch are normal; nested batches across calls are still possible but no longer the bug pattern).
- Index-shift bugs (snapshots are by `card_handle`, not field index).
- The behavioral oddity where Save inspects mid-deletion live state.
- The `pending_post_deletion_replays` workaround — can be absorbed.
- The Save+Fortitude race documented in `integration_smoke.rs:409` becomes a normal TriggerOrder choice (player picks who runs first; first one to grab the card from trash wins) — and that's the *no-approximations* outcome the RL agent should learn from.

**What it doesn't solve.**
- The OTHER single-slot families (`parked_replacement`, `dsl_outer_tail`). Those should probably be stacks too, but that's separate work.
- Synchronous-drain assumption baked into other call sites. Replacement window batching is plausibly a deep change.

**Risk.** Highest churn. Many behavioral tests need rewriting. Replacement-mid-batch interactions (Decoy, Barrier) need a deliberate design.

### Option C — hybrid

Batch the multi-target callers (`resolve_battle` tie, `DeleteBoundPermanents`, multi-pick deletion effects), keep the single-target path on the current model with an Option-A stack underneath.

`delete_permanents_batch([h1, h2])` is the DCGO-faithful path. `delete_permanent_with_cause(h)` is the legacy single-target path, but `pending_deletion_resume` becomes a `Vec` so the panic stops firing.

**What it solves.** The panic + the most-visible parity gap (mutual destruction in battle). Card-text "delete N" effects get correct simultaneous-death semantics.

**What it doesn't solve.** Save still fires pre-trash on single-target deletions, so the test assertions about live-carrier-during-Save persist. The architectural mismatch persists for the most common case (single-target effect deletion).

**Risk.** Middle. Has the worst property of *two* models coexisting in the same engine — which has historically been the source of the parity gaps we're chasing.

---

## 5. Risk register

### 5a. Behavioral test churn

Tests that explicitly assert "carrier still on field while selection parked":

| Test | File | Assertion |
|------|------|-----------|
| `save_accept_places_card_under_tamer_post_deletion` | [save.rs:149-229](../../../code/digimon-engine/tests/keyword_phase_d/save.rs:149) | `battle_area.len() == 2` with carrier + Tamer mid-Save |
| `save_under_decoy_decline_defers_via_no_replace_path` | [save.rs:395-496](../../../code/digimon-engine/tests/keyword_phase_d/save.rs:395) | Same pattern, Decoy-decline route |
| `partition_plays_two_picked_sources_on_opponent_effect_deletion` | [partition.rs:135-266](../../../code/digimon-engine/tests/keyword_phase_d/partition.rs:135) | `battle_area.len() == 1` with carrier mid-Partition pick |
| `save_and_fortitude_compose_when_save_is_accepted` | [integration_smoke.rs:409-509](../../../code/digimon-engine/tests/keyword_phase_d/integration_smoke.rs:409) | Pins "Save wins the race for the top card" — under DCGO model, this is a TriggerOrder choice |

Option A: zero churn — assertions stay true.
Option B: all four need rewrites. The Save+Fortitude end-state assertion in particular pins an *implementation-specific* race outcome; under DCGO this is a player choice the test should make explicit. That's *more* faithful to the no-approximations policy ("every choice must surface through pending_selection"), but it's a test redesign.

There are likely more behavioral tests across `tests/cards_behavioral/` that read pre-removal state via the live handle. A naïve estimate is **30-80 tests** touching the pattern, but I haven't surveyed. A spike to count would inform B's true cost.

### 5b. "This Digimon" inside OnDeletion bodies

Cards whose OnDeletion handler reads "this Digimon's DP/level/traits/whatever." Today that's a live handle lookup. Under DCGO the perm is gone — the body must consult the snapshot.

`EffectContext` would need snapshot-aware accessors: `ctx.deleted_self_dp()`, `ctx.deleted_self_level()`, `ctx.deleted_self_traits()`. The migration story for existing handlers that do `ctx.game.player(handle.player).battle_area.get(handle.index)` is mechanical but not free.

Greppable: any OnDeletion handler that touches `battle_area.get(handle.index)` is suspect. The `keyword_effects.rs` Save body already does this at line 537. A scan across `cards/` is in scope before going past Option A.

### 5c. Replacement-batch ordering

DCGO runs `WhenPermanentWouldBeDeleted` and `WhenRemoveField` as two SEPARATE drains, both before re-filter. Rust today fires `WhenWouldLeaveBattleArea` then `WhenWouldBeDeleted` per-permanent. Under Option B, both become batched two-stage drains.

The subtle one: a `Barrier` (cancel removal) on perm A and a `Decoy` (substitute another perm) on perm B — when both fire in the same WhenPermanentWouldBeDeleted batch, does the order matter? DCGO uses a CutIn UI (player picks resolution order). Rust today recurses. Need to decide: does Option B preserve TriggerOrder bundling for replacements too, or do replacements stay synchronous and only post-replacement triggers become batched?

### 5d. `pending_post_deletion_replays` parallel slot

Already a Vec — already handles multiple Fortitude/Partition outstanding. If Option B absorbs it, the absorption needs to preserve the "drain BEFORE OnAnyDeletion" ordering at the [combat.rs:3490](../../../code/digimon-engine/src/combat.rs:3490) site. That ordering keeps OnAnyDeletion observers seeing replayed permanents on field — a subtle parity property.

### 5e. Replays during a deletion batch

What happens if a Fortitude replay during finalize itself triggers a deletion (e.g., the replayed permanent has an ETB that deletes opponent's stuff)? Today: the recursion already handles this via `pending_post_deletion_replays`'s "fresh Vec swapped in" comment (line 3486). Under Option B: the replay's deletion is just a fresh `delete_permanents_batch` call, recursing into the same code path. Cleaner — but the test in `integration_smoke.rs:409` that pins this ordering needs to be re-read with that in mind.

### 5f. Forensic compatibility

Existing GameRecorder recordings reference action IDs computed against the current substrate. If batching shifts what actions get exposed during a Save (e.g., the Tamer-pick happens at a different turn-state), old recordings may not replay deterministically. Replay parity is an explicit goal of the new `LiveGame` work (commit `eec791dd`); this is worth checking against any saved recordings before merging Option B.

---

## 6. Surfaceable open questions

These are the ones I'd want a verbal answer on before going past explore:

1. **Is the team willing to take behavioral-test churn for architectural clarity?** Option B is correct; Option A is fast. The trade-off is visible.

2. **Is the no-approximations policy strict enough to break the "Save automatically wins over Fortitude" race?** Under Option B, that's a player choice. Today it's an engine quirk. The strict reading of the policy says: yes, player picks; today's behavior is an approximation.

3. **How many other single-outstanding slots are we willing to leave?** `parked_replacement`, `dsl_outer_tail` — if either becomes the next training panic, do we batch-stack them too, or are they fundamentally different?

4. **Does `delete_permanents_batch` deserve to be the public API and `delete_permanent_with_effects(handle)` becomes a one-element-list shim?** That's cleaner but it's a breaking surface change for card scripts. Or: keep both, document the batch as preferred.

5. **`Permanent` identity post-removal — pointer/Arc or snapshot?** DCGO uses pointer identity (C# managed refs survive removal from the field collection). Rust could go Arc, or it could go snapshot-only with `CardHandle` as the durable ID. Snapshot-only is simpler and matches existing `DeletedObjectSnapshot`; Arc is more DCGO-faithful but a deeper refactor.

6. **Replacement-batch ordering — recurse or batch-mutate?** When a Decoy substitutes mid-batch, does it append to the active batch or trigger a sub-batch?

7. **Is the `pending_post_deletion_replays` slot kept (as a workaround) or absorbed (under the DCGO model)?** If kept, scope is narrower; if absorbed, the bigger picture is cleaner.

---

## 7. Recommendation

**Target scope: Option B (full DCGO model), starting from `commit_permanent_deletion` and Save+Fortitude+Partition, NOT going as wide as DCGO `DestroyPermanentsClass` in v1.**

Rationale:

- The user's pattern is a *family*. Option A fixes one member of the family; the next member will reproduce this conversation in a month. Pay the design cost once.
- The no-approximations policy ([CLAUDE.md](../../../CLAUDE.md) §"Project Vision") says: every choice must be exposed to the RL action space. Save+Fortitude's racing-on-engine-internals is an approximation. Option B converts it to a TriggerOrder pick that the agent can learn.
- `pending_post_deletion_replays` is the canary. It already exists *because* Rust's OnDeletion-before-trash ordering is wrong. Adopting DCGO ordering retires it as a special case. That's progress on the architectural mismatch the parity doc tracks.
- The DCGO `DestroyPermanentsClass` batch is internally consistent because it's grown organically. A Rust v1 doesn't need full feature parity (e.g., batched WhenPermanentWouldBeDeleted CutIn UI). Single-target deletions can route through a one-element batch; the batched form lights up `resolve_battle`'s mutual destruction + DSL `DeleteBoundPermanents` as the user-visible wins.

Suggested v1 scope:

- New `Game::delete_permanents_batch(Vec<PermanentHandle>, cause)` API.
- Build `DeletionRecord { handle, snapshot, top_card }` as the per-permanent unit inside the batch.
- Replacement window stays per-permanent (recursion + parked_replacement unchanged) — that's Option-C-shaped for the replacement layer, full-B for the post-replacement flow.
- OnDeletion drain runs once at end-of-batch over the surviving records. Save reads snapshot+trash; Fortitude reads snapshot.source_count + plays from trash; Partition reads snapshot.digisources + plays N from trash.
- `pending_post_deletion_replays` slot absorbed.
- `pending_deletion_resume` slot retained (but only used when an OnDeletion handler inside the batched drain parks a selection). Becomes `Vec<...>` to support nested-park-during-batched-drain.
- Behavioral tests under `keyword_phase_d/` rewrite the "carrier still on field" assertions to "carrier already trashed; selection still parks."
- Survey for other OnDeletion handlers reading live state (`grep -rn 'battle_area.get(' code/digimon-engine/src/cards/`) — convert to snapshot accessors.

What stays out of v1:

- Batched replacement window. Keep per-permanent recursion. Substitute-into-batch mutation is the gnarliest part; defer until there's a card that actually demands it.
- `parked_replacement` and `dsl_outer_tail` stack-ification. Separate change.
- Snapshot-on-CardSource (DCGO `PermanentJustBeforeRemoveField` pointers). Not needed for the Save/Fortitude/Partition cohort; punt until a card asks for the cross-source "we belonged to the same stack" predicate.

**Whether to proceed to `opsx:propose`:** yes, with the explicit goal of pinning down Q1, Q5, and Q6 above before writing tasks. Q2 and Q7 are settled by this recommendation (snapshot-only; absorb the slot). Q4 leans toward keeping both APIs with the batch as preferred — propose-mode is the right place to commit.

---

## 8. Appendix — relevant references

- Bug fire sites: [combat.rs:3338-3344](../../../code/digimon-engine/src/combat.rs:3338), [replacement.rs:1381-1388](../../../code/digimon-engine/src/replacement.rs:1381)
- Sibling slots: [game.rs:483](../../../code/digimon-engine/src/game.rs:483) (`parked_replacement`), [game.rs:526](../../../code/digimon-engine/src/game.rs:526) (`pending_deletion_resume`), [game.rs:562](../../../code/digimon-engine/src/game.rs:562) (`pending_post_deletion_replays`), [game.rs:585](../../../code/digimon-engine/src/game.rs:585) (`dsl_outer_tail`)
- Snapshot type: `crate::trigger_context::DeletedObjectSnapshot` (used at [combat.rs:3429-3442](../../../code/digimon-engine/src/combat.rs:3429))
- DCGO batch deletion: [CardController.cs:3673-3899](../../../DCGO/Assets/Scripts/Script/CardController.cs:3673)
- DCGO Save body: [Save.cs](../../../DCGO/Assets/Scripts/Script/CardEffectCommons/KeyWordEffects/Save.cs)
- DCGO predicates: [OnDeletion.cs:144](../../../DCGO/Assets/Scripts/Script/CardEffectCommons/CanUseEffects/OnDeletion.cs:144) (`IsTopCardInTrashOnDeletion`), [OnDeletion.cs:169](../../../DCGO/Assets/Scripts/Script/CardEffectCommons/CanUseEffects/OnDeletion.cs:169) (`IsTopCardSamePermanent`)
- Tests anchoring current "carrier-still-on-field" semantics: [save.rs:149](../../../code/digimon-engine/tests/keyword_phase_d/save.rs:149), [save.rs:395](../../../code/digimon-engine/tests/keyword_phase_d/save.rs:395), [partition.rs:135](../../../code/digimon-engine/tests/keyword_phase_d/partition.rs:135), [integration_smoke.rs:409](../../../code/digimon-engine/tests/keyword_phase_d/integration_smoke.rs:409)
- Existing post-trash workaround: `pending_post_deletion_replays` slot ([game.rs:531-563](../../../code/digimon-engine/src/game.rs:531)), drained in `finalize_permanent_deletion` at [combat.rs:3490](../../../code/digimon-engine/src/combat.rs:3490)
- **Deferred-drain infrastructure (pulled in from `claude/epic-perlman-29ac0f`, 2026-05-23):**
  - `Game::draining_deferred: u32` at [game.rs:646](../../../code/digimon-engine/src/game.rs:646)
  - `enter_deferred_drain()` / `exit_deferred_drain_and_flush()` / `maybe_drain_effect_queue()` at [effect_queue.rs:650-672](../../../code/digimon-engine/src/effect_queue.rs:650)
  - Already wired through `fire_on_play`, `fire_on_leave_field`, `fire_on_place_security`, `fire_on_link_after_option_placed`, and [combat::fire_on_attack:3007](../../../code/digimon-engine/src/combat.rs:3007)
  - `fire_digivolution_card_trashed` and `place_permanent_on_security`'s `OnDigivolutionCardTrashed`/`OnLinkedCardTrashed` fires INTENTIONALLY retain inline drain (`ex10_036` behavioral test depends on between-source observer firing)
- **Sibling-bug status post-pull:** `G-DSL-OUTER-TAIL-NESTED-PARK` and `G-OPTION-PLAY-REENTRANT` are RESOLVED (the family's two other open panics). `G-DELETION-RESUME-NESTED` is the last one standing. The team chose Option B (deferred-drain) over Option A (stack) for the `dsl_outer_tail` fix — precedent.
- **`pending_post_deletion_replays` slot ([game.rs:531-563](../../../code/digimon-engine/src/game.rs:531))** is still a `Vec` (the model for stacking).

---

## 9. Post-pull update — recommendation refinement

When this doc was first drafted, I noted the deferred-drain infra didn't exist in this worktree. After pulling `claude/epic-perlman-29ac0f`, it does. That changes the cost equation:

- The "deferred drain a deletion batch" mechanic Option B requires already has a working implementation pattern in the engine — five `fire_on_*` helpers are already calling `maybe_drain_effect_queue()` inside `enter_deferred_drain` scopes.
- The two precedents the team set in the same fix family (the `dsl_outer_tail` and `pending_end_turn_resume` fixes) both went architectural rather than stack-the-slot. The appetite is there.
- The gap entry's own "Suggested change" line proposes Option A (Vec stack) as the immediate fix, with the rationale "the fix is mechanical." That's correct as an immediate panic-closer, but it leaves the architectural mismatch in place. The Save+Fortitude race documented in `integration_smoke.rs:409` is the visible parity artifact.

**Refined recommendation:** Option B remains the target, but the path can be incremental:

1. **First commit — close the panic on the existing model (Option A).** Convert `pending_deletion_resume: Option<...>` → `Vec<...>`. ~40-60 lines, mechanical, no test churn. Panic stops firing in training.
2. **Second commit (or follow-up PR) — adopt the DCGO trash-before-drain model.** Use the deferred-drain infra that already exists. Wrap `commit_permanent_deletion`/`delete_permanents_batch` in `enter_deferred_drain` … `exit_deferred_drain_and_flush`. Move trash before OnDeletion drain. Rewrite Save/Fortitude/Partition handlers to read snapshot+trash. Update behavioral tests.
3. **Defer for v1.x:** snapshot-on-CardSource for cross-source identity predicates.

This staging gets the user out of training crashes immediately while still committing to the architectural fix. The Option-A patch in step 1 isn't throwaway — when step 2 lands, the stack stays useful for nested-park-during-batched-drain (an OnDeletion handler in the batched drain that itself parks a fresh selection).

---

## 10. Decisions (2026-05-23 — user closeout)

The seven open questions in §6 were closed verbally before moving to propose. Captured here so propose-mode doesn't re-open them.

| # | Question | Decision | Rationale |
|---|----------|----------|-----------|
| Q1 | Accept behavioral-test churn for architectural clarity? | **Yes.** | DCGO is treated as correctness. |
| Q2 | Save+Fortitude race — expose to agent via TriggerOrder? | **N/A — drop.** | No card prints both keywords. `integration_smoke.rs:409` becomes either deletable or rewritten as a unit fixture with no real-card claim. |
| Q3 | Other single-outstanding slots (`parked_replacement`, etc.) — batch-stack them too? | **Out of scope.** | Address when/if they panic in training. |
| Q4 | `delete_permanents_batch` as primary API, single-target as shim? | **Yes — match DCGO.** | Mirrors `DestroyPermanentsClass`. Single-target callers re-route through a one-element list. |
| Q5 | Permanent post-removal identity — Arc-wrap or snapshot? | **Snapshot.** | Mimics DCGO predicate *semantics* (what was my predecessor's DP/level/stack); avoids the Arc-wrap refactor. Extend the existing `DeletedObjectSnapshot` struct. |
| Q6 | Replacement batch ordering — recurse on substitute, or batch-mutate? | **Mimic DCGO.** | Two-stage cut-in (`WhenWouldLeaveBattleArea` → drain → `WhenWouldBeDeleted` → drain → re-filter). Substitutes mutate the active batch's kill list rather than recursing. |
| Q7 | `pending_post_deletion_replays` slot — keep or absorb? | **Absorbed.** | DCGO doesn't have this slot; Fortitude/Partition become normal post-trash OnDeletion handlers. |

### Final v1 scope (post-decisions)

Wider than the §7 recommendation because Q6 pulled batched replacement window in-scope.

**In scope:**

- `Game::delete_permanents_batch(handles, cause)` as the primary deletion API.
- Single-target `delete_permanent_with_cause(handle)` reduces to a one-element-list shim.
- Two-stage batched replacement cut-in: enqueue `WhenWouldLeaveBattleArea` for the whole kill list, drain via the deferred-drain scope, then enqueue `WhenWouldBeDeleted` for the surviving list, drain, re-filter. Substitutions append the substitute to the kill list (DCGO `willBeRemoveField = true` shape) rather than recursing.
- Trash happens *before* the OnDeletion drain.
- `DeletedObjectSnapshot` extended with `dp_just_before`, `level_just_before`, `cost_just_before`, `names_just_before`, `traits_just_before`. Snapshot is attached to a per-deletion record and threaded into the OnDeletion / OnAnyDeletion / OnLeaveField trigger contexts.
- `EffectContext` snapshot accessors (`deleted_self_dp`, `deleted_self_level`, etc.) — needed by OnDeletion handlers that today read live state via the handle.
- `Keyword::Save` rewrite — body finds `self_card` in trash via the snapshot, lifts via `place_card_under_permanent_bottom`.
- `Keyword::Fortitude` rewrite — gate reads `snapshot.source_count`, body plays `self_card` from trash. `pending_post_deletion_replays` push removed.
- `Keyword::Partition` rewrite — pick N from `snapshot.digisources`, play them from trash. `pending_post_deletion_replays` push removed.
- `Game::pending_post_deletion_replays` field deleted along with its drain site in `finalize_permanent_deletion`.
- `Game::pending_deletion_resume`: stays as a `Vec<...>` stack (an OnDeletion handler inside the batched drain that parks a selection still uses this).
- Behavioral test rewrites: at minimum the four named tests in §5a (Save, Save-under-Decoy, Partition, Save+Fortitude integration_smoke). Survey for other OnDeletion handlers reading live-handle state.

**Out of v1 (deferred):**

- Snapshot-on-`CardSource` (DCGO `PermanentJustBeforeRemoveField` cross-source ref-equality). Wait for a card that needs the "we belonged to the same stack" predicate.
- Stack-ification of `parked_replacement` / other Option-shaped slots.
