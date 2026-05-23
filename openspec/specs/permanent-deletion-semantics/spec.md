# permanent-deletion-semantics Specification

## Purpose
TBD - created by archiving change align-deletion-with-dcgo-model. Update Purpose after archive.
## Requirements
### Requirement: Permanent deletion is batched across the kill list

The engine SHALL provide a batched deletion entrypoint that accepts a list of permanent handles and processes them as a single unit. The single-target deletion entrypoint SHALL be a one-element-list shim over the batched entrypoint. Callers that today invoke single-target deletion (effects, battle resolution, cost payment, DSL `DeleteBoundPermanents`) MUST observe identical end states when their list has exactly one element.

#### Scenario: Single-target deletion routes through batch entrypoint

- **WHEN** a card effect calls the single-target deletion API for one permanent
- **THEN** the engine processes it through the batched flow with a one-element kill list
- **AND** the observable end state (permanent removed from battle area, top card in trash, OnDeletion/OnAnyDeletion observers fired) matches the pre-batched behavior for non-pathological cases

#### Scenario: Mutual destruction in battle is one batch

- **WHEN** an attacker and defender tie on resolved DP and both must be deleted
- **THEN** the engine calls the batched deletion API once with both handles in the kill list
- **AND** neither permanent's OnDeletion fires before both are trashed
- **AND** OnAnyDeletion observers see both deletions in the same trigger drain

#### Scenario: AoE Option deleting multiple permanents

- **WHEN** an effect collects N target permanents and invokes batched deletion
- **THEN** all N targets are processed in one batch with shared replacement-window cut-in and shared OnDeletion drain

### Requirement: Two-stage replacement cut-in runs before any deletion commits

The engine SHALL run the deletion replacement window as two stacked stages over the kill list: first `WhenWouldLeaveBattleArea` for every survivor, drained as a unit; then `WhenWouldBeDeleted` for the remaining survivors, drained as a unit. Re-filtering of the kill list MUST happen after each stage so that cancelled/redirected permanents are removed before the next stage. Stage drains MUST run inside a deferred-drain scope so a parked selection inside one replacement handler does not block the others.

#### Scenario: Replacement handler cancels deletion for one of two targets

- **WHEN** a batch with two permanents enters the replacement cut-in and a `WhenWouldBeDeleted` handler cancels the deletion of one
- **THEN** the cancelled permanent is removed from the surviving kill list before the trash step
- **AND** the cancelled permanent remains on the battle area
- **AND** the other permanent proceeds to snapshot, trash, and OnDeletion drain

#### Scenario: Replacement handler redirects deletion to a different zone

- **WHEN** a `WhenWouldBeDeleted` handler redirects a permanent's removal to deck or hand
- **THEN** the redirected permanent is moved to the destination zone instead of trash
- **AND** the redirected permanent's OnDeletion does not fire
- **AND** other permanents in the batch continue through normal trash + OnDeletion

#### Scenario: Replacement handler parks a player selection

- **WHEN** a `WhenWouldLeaveBattleArea` replacement handler installs a player selection during the stage 1 drain of a multi-permanent batch
- **THEN** the batched flow pauses until the selection resolves
- **AND** no permanent in the batch has been trashed yet
- **AND** when the selection resolves, the batched flow resumes from the same stage with the same surviving kill list

### Requirement: Substitutes mutate the active batch's kill list

When a replacement handler substitutes the deletion target (DCGO `<Decoy>` pattern — "delete this Digimon instead"), the substitute SHALL be appended to the active batch's kill list rather than triggering a fresh recursive deletion call. The substitute MUST then go through the appropriate replacement stage before being committed.

#### Scenario: Decoy substitutes self for an ally being deleted

- **WHEN** an opposing effect targets permanent A for deletion, and permanent B has `<Decoy>` and the controller accepts the optional redirect
- **THEN** permanent A is removed from the active batch (its deletion is replaced)
- **AND** permanent B is appended to the active batch
- **AND** permanent B proceeds through the remaining replacement stage and the trash + OnDeletion drain
- **AND** no recursive call to the deletion API occurs

#### Scenario: Decoy substituted during stage 2

- **WHEN** a `WhenWouldBeDeleted` (stage 2) handler substitutes a new target into the active batch
- **THEN** the substitute is treated as already-past-stage-1 and joins the stage 2 pass before the snapshot step
- **AND** the substitute is included in the trash step and the OnDeletion drain

### Requirement: Pre-removal snapshots are captured before trash

After the replacement cut-in stages complete and the surviving kill list is finalized, the engine SHALL capture a per-permanent `DeletedObjectSnapshot` for every surviving permanent before any trash mutation occurs. The snapshot MUST record the permanent's effective DP, level, play cost, top-card card names, top-card card traits, the count of card sources beneath the top, and the digivolution-card handles. The snapshot MUST be threaded into the trigger context for the subsequent OnDeletion, OnAnyDeletion, and OnLeaveField drains.

#### Scenario: Snapshot records pre-removal DP

- **WHEN** a permanent with effective DP modified by a continuous effect is deleted
- **THEN** the snapshot's `dp_just_before` field records the modifier-aware DP at the moment before trash
- **AND** any OnDeletion handler that reads `deleted_self_dp()` receives that value
- **AND** the same value is available to OnAnyDeletion observers via the trigger context

#### Scenario: Snapshot records pre-removal digivolution-card list

- **WHEN** a permanent with multiple digivolution cards beneath the top is deleted
- **THEN** the snapshot's `digisources_just_before` lists every card source below the top in stack order
- **AND** the snapshot's `source_count_just_before` equals the number of cards in `digisources_just_before`

### Requirement: OnDeletion fires after the top card is in trash

The engine SHALL run the trash mutation (DiscardEvoRoots, RemoveField, AddTrashCard) for every surviving permanent in the batch *before* enqueueing or draining any `OnDeletion`-timed effect. Handlers attached to a deleted permanent's effects MUST observe the deleted permanent as absent from the battle area, and MUST find the deleted permanent's top card in the controller's trash.

#### Scenario: Save handler reads card from trash

- **WHEN** a permanent with `<Save>` is deleted and its OnDeletion handler runs
- **THEN** the carrier's top card is already in the controller's trash
- **AND** the carrier is no longer in the battle area
- **AND** the handler locates the saved card by the snapshot's `top_card` handle and walks the trash zone

#### Scenario: Fortitude handler plays card from trash

- **WHEN** a permanent with `<Fortitude>` is deleted from a stack with ≥1 digi source and its OnDeletion handler runs
- **THEN** the carrier's top card is already in the controller's trash
- **AND** the handler reads `deleted_self_source_count() >= 2` (top + ≥1 source) from the snapshot
- **AND** the handler plays the snapshot's `top_card` from trash via the free-unsuspended path
- **AND** subsequent OnAnyDeletion observers see the replayed permanent on the battle area

#### Scenario: OnDeletion handler that parks a selection

- **WHEN** an OnDeletion handler installs a player selection during the batch's OnDeletion drain
- **THEN** the batch's drain pauses until the selection resolves
- **AND** other surviving permanents in the same batch may have already trashed but their own OnDeletion handlers (if not yet drained) wait their turn
- **AND** when the selection resolves, the drain continues until the queue is empty

### Requirement: Multiple OnDeletion-parking permanents in one batch resolve sequentially

Two or more permanents in the same batch whose OnDeletion handlers each install a player selection (e.g., two `<Save>` permanents in an AoE-Option kill) SHALL resolve their selections one at a time without panicking on the substrate's outstanding-resume slot. The engine MUST NOT trip a `debug_assert!` when the second permanent's handler attempts to park while the first is still resolving.

#### Scenario: AoE Option deletes two Save permanents

- **WHEN** an Option deletes two permanents simultaneously, each carrying `<Save>` with at least one valid Tamer candidate
- **THEN** the first permanent's Save handler parks its Tamer-pick selection
- **AND** when the controller resolves the first selection (accept or decline), the second permanent's Save handler parks its own Tamer-pick selection
- **AND** when the controller resolves the second selection, the batched flow completes
- **AND** no `pending_deletion_resume` overwrite assertion fires

#### Scenario: Save under Decoy decline path with multiple permanents

- **WHEN** an AoE deletion includes two permanents that both have `<Save>` and one also has `<Decoy>`, and the Decoy is declined
- **THEN** both `<Save>` handlers park sequentially without substrate panic
- **AND** the cancelled-Decoy permanent (if Decoy was accepted) is the substitute that's also batched

### Requirement: OnAnyDeletion and OnLeaveField receive snapshot-carrying contexts

After the batch's OnDeletion drain settles, the engine SHALL enqueue `OnAnyDeletion` and `OnLeaveField` for each surviving permanent in the batch and drain them in the same deferred-drain scope. Each enqueued trigger MUST carry the corresponding permanent's `DeletedObjectSnapshot` in its trigger context so observer handlers (e.g., Puppet self-refire, BG Imperial substrate observers) can read `event_target_*` predicates against the deleted permanent.

#### Scenario: OnAnyDeletion observer reads deleted permanent's traits

- **WHEN** a batch deletes a permanent and an opposing observer subscribes to `OnAnyDeletion` with a trait-matching predicate
- **THEN** the observer's trigger context carries the deleted permanent's `traits_just_before`
- **AND** the predicate evaluates against those traits even though the permanent is no longer on the battle area

#### Scenario: OnLeaveField for two deletions in one batch

- **WHEN** two permanents are deleted in one batch
- **THEN** two `OnLeaveField` events are enqueued and drained in turn
- **AND** each carries its own snapshot

### Requirement: Snapshot accessors expose deleted-self state to handlers

The engine SHALL expose snapshot fields to OnDeletion / OnAnyDeletion / OnLeaveField handlers through `EffectContext` accessors: `deleted_self_dp`, `deleted_self_level`, `deleted_self_cost`, `deleted_self_names`, `deleted_self_traits`, `deleted_self_source_count`, `deleted_self_digisources`. Each accessor MUST return the snapshot's pre-removal value when the trigger context carries a snapshot, and `None` (or an empty slice) otherwise.

#### Scenario: Handler reads self DP from snapshot

- **WHEN** an OnDeletion handler runs against a snapshot-carrying trigger context
- **THEN** `ctx.deleted_self_dp()` returns the modifier-aware DP value the permanent had immediately before its trash move
- **AND** the value is independent of whether the handler queries before or after other batch-mates have been trashed

#### Scenario: Accessor on a non-snapshot trigger context

- **WHEN** an effect fires on a trigger context with no `deleted_object` (e.g., a draw observer)
- **THEN** `ctx.deleted_self_*()` accessors return `None` or empty
- **AND** the handler is responsible for guarding against missing-snapshot states

### Requirement: `pending_post_deletion_replays` slot is retired

The previously-used `Game::pending_post_deletion_replays` slot SHALL be removed. Fortitude- and Partition-style replays MUST run inline within the OnDeletion handler against the snapshot+trash, not via a deferred slot.

#### Scenario: Fortitude no longer pushes to the retired slot

- **WHEN** a `<Fortitude>` permanent is deleted with sufficient digi sources
- **THEN** the OnDeletion handler plays the card from trash directly during the batch's OnDeletion drain
- **AND** the engine has no `pending_post_deletion_replays` field to drain
- **AND** subsequent `OnAnyDeletion` observers see the replayed permanent on field

#### Scenario: Partition picks digisources from snapshot

- **WHEN** a `<Partition>` permanent is deleted by an opponent's effect with ≥2 digi sources
- **THEN** the OnDeletion handler reads `deleted_self_digisources` from the snapshot
- **AND** the handler installs a 2-pick selection over those handles
- **AND** the picked cards are played from trash to the battle area via the free-unsuspended path

