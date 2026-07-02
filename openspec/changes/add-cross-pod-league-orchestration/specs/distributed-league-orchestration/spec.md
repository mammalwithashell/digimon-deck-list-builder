## ADDED Requirements

### Requirement: Shared artifact store for league state
The Tier-2 orchestrator SHALL keep all cross-pod league state — the `SpecialistRegistry`, per-deck checkpoints, and per-round opponent-pool manifests — in a shared artifact store (reusing `code/server/storage/` adapters), because pods do not share local disk. Pods SHALL read their round pool from the store and write their checkpoint back to it.

#### Scenario: Pod reads pool and writes checkpoint via the store
- **WHEN** a pod is dispatched for round *k* of deck *d*
- **THEN** it downloads round *k*'s pool manifest for deck *d* from the store, trains, and uploads its `final.zip` + `.meta.json` back to the store under a deck/round-keyed path

#### Scenario: Registry is the durable source of truth
- **WHEN** the orchestrator restarts after a crash
- **THEN** it reloads the `SpecialistRegistry` from the shared store and resumes from the last completed round, with no league state held only in the orchestrator's memory

### Requirement: Per-round barrier across pods
The Tier-2 orchestrator SHALL enforce a per-round barrier: because round *k+1*'s opponent pool for every deck is built from ALL decks' round-*k* specialists, the orchestrator MUST wait for every deck's round-*k* pod to produce a verified checkpoint before emitting round *k+1*'s pools or dispatching any round-*k+1* pod.

#### Scenario: Round k+1 waits for all of round k
- **WHEN** five of six decks have finished round *k* and the sixth is still training
- **THEN** the orchestrator holds all round-*k+1* dispatch until the sixth deck's round-*k* checkpoint is harvested and verified, then runs the barrier and emits round *k+1*'s pools

#### Scenario: Barrier reuses the single-box primitives
- **WHEN** the orchestrator advances a round
- **THEN** it builds the next round's pools and updates the registry using the existing `write_round_pool` / `_barrier` building blocks, not a reimplementation of league pooling logic

### Requirement: Centralized single-writer registry updates
The Tier-2 orchestrator SHALL be the SOLE writer of the `SpecialistRegistry` — pods only write their own checkpoints — so concurrent pods cannot race on the registry. The barrier (registry update) MUST run on the orchestrator after harvesting a round's checkpoints.

#### Scenario: Concurrent pods do not race the registry
- **WHEN** multiple round-*k* pods finish near-simultaneously
- **THEN** each only uploads its own checkpoint, and the orchestrator alone folds all of them into the registry at the barrier, producing a single consistent round-*k* snapshot

### Requirement: Per-pod dispatch and completion detection on a durable control node
The Tier-2 orchestrator SHALL run on a durable control node (not on an ephemeral training pod), dispatch one deck per pod per round (provisioning via the `training-pod-provisioning` layer), and detect per-pod completion via a checkpoint/done-marker poll in the shared store — so a dying training pod never loses the round state machine.

#### Scenario: Orchestrator survives a pod dying
- **WHEN** a training pod dies mid-round
- **THEN** the orchestrator (running on the durable control node) detects the missing checkpoint, re-dispatches that deck's round to a fresh pod, and the round still completes

#### Scenario: Completion is detected from the store, not pod liveness
- **WHEN** a pod uploads its verified round checkpoint and done-marker
- **THEN** the orchestrator counts that deck's round as complete based on the store contents, independent of whether the pod is still alive
