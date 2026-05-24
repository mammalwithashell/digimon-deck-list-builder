## ADDED Requirements

### Requirement: BT17-081 activation-cost gate enforces per-trigger suspend payability

BT17-081 Tai Kamiya & Matt Ishida's `[All Turns]` triggered clause SHALL encode its "by suspending this Tamer" cost as a leading `activation_cost: { suspend_self: true }` body step (the BT13-101 / P-136 idiom), which the engine's `lower_triggered::lower_for_kind_with_clause_index` lifts onto `EffectBuilder::activation_cost(...)`. The clause SHALL NOT use a body-step `suspend: { target: source }` for this cost. The engine SHALL evaluate the lifted activation cost per-queued-trigger via `EffectContext::suspend_self_as_cost`, so simultaneous triggers from the same event chain grant the printed memory reward **at most once** — subsequent triggers inert when BT17-081 is already suspended and cannot pay the cost.

#### Scenario: Single trigger pays cost and grants memory

- **WHEN** one of the controller's Digimon is played or digivolves AND BT17-081 is unsuspended
- **AND** the trigger is resolved (either by picking it from a `TriggerOrder` bundle, or via the engine's auto-fire path when only one trigger is queued for the active chooser)
- **THEN** `EffectContext::suspend_self_as_cost` returns true and BT17-081 suspends as the cost payment
- **AND** the controller gains +1 memory for each Greymon-name Digimon present on their battle area at resolution time
- **AND** the controller gains +1 memory for each Garurumon-name Digimon present on their battle area at resolution time

#### Scenario: Second sequential trigger inerts when cost cannot be paid

- **WHEN** two BT17-081 `[All Turns]` triggers fire sequentially on the same turn (e.g. two own Digimon plays, or a play plus a same-chain digivolve)
- **AND** the first trigger resolves: `suspend_self_as_cost` returns true, BT17-081 suspends, body runs, memory is granted
- **AND** the second trigger then resolves
- **THEN** `EffectContext::suspend_self_as_cost` returns false because BT17-081 is already suspended
- **AND** the second trigger's body does not run — no additional memory is granted, BT17-081 is not double-suspended

#### Scenario: Trigger inerts when source is pre-suspended

- **WHEN** BT17-081 is suspended at the moment a play or digivolve event fires its `[All Turns]` trigger
- **AND** the trigger's activation_cost_fn is invoked (`suspend_self_as_cost`)
- **THEN** the cost call returns false and the body silently skips
- **AND** no memory is granted, no state corruption occurs, the engine does not panic

#### Scenario: Test fixture has memory headroom for gains

- **WHEN** a behavioral test exercises BT17-081's `[All Turns]` memory grants
- **THEN** the test fixture sets the starting memory to a value with at least 2 points of headroom inside `Rules::standard().memory_range` (e.g. memory == 0), so `gain_memory(+1)` calls don't clamp at the seesaw boundary and produce false-positive zero deltas

### Requirement: BT17-081 simultaneous-trigger behavior has regression coverage

A behavioral test in `code/digimon-engine/tests/cards_behavioral/bt17/bt17_081.rs` SHALL exercise the simultaneous-trigger case end-to-end, asserting the controller's memory delta is exactly +2 (not +4) when two BT17-081 triggers queue from the same event chain with one Greymon-name and one Garurumon-name Digimon on field.

#### Scenario: Two simultaneous triggers grant memory once

- **WHEN** a behavioral test constructs a board with BT17-081 (unsuspended), a Greymon-named Digimon, and a Garurumon-named Digimon on the controller's field
- **AND** the test triggers two BT17-081 `[All Turns]` activations in a single event chain
- **AND** the test resolves both triggers in TriggerOrder
- **THEN** the controller's memory delta attributable to BT17-081 is exactly +2
- **AND** BT17-081 is suspended exactly once
