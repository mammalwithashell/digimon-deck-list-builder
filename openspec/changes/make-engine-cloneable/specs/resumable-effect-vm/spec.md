## ADDED Requirements

### Requirement: Data-state effect execution
DSL card effects SHALL execute through a resumable interpreter over the compiled DSL AST whose entire in-flight state is plain data — an instruction pointer, a binding/value stack, and an explicit frame stack — with no `Box<dyn Fn>`/`Box<dyn FnOnce>` capturing the suspended computation.

#### Scenario: In-flight effect state is data
- **WHEN** an effect is paused mid-resolution awaiting a player choice
- **THEN** its complete suspended state is representable as plain data (card, effect slot, instruction pointer, bindings, frame stack) with no boxed closure holding the continuation

### Requirement: Selections are data yields
A player choice point (`select_*`) SHALL halt the interpreter with a `PendingSelection` data record and, on resolution, push the chosen value onto the binding stack and resume at the saved instruction pointer, preserving the no-approximations rule that every choice surfaces through `pending_selection`.

#### Scenario: Choice resumes from data
- **WHEN** a selection is resolved
- **THEN** the interpreter resumes the effect from the saved instruction pointer using the pushed choice, with no captured callback invoked

#### Scenario: Every choice is still exposed
- **WHEN** an effect requires a player decision
- **THEN** that decision is exposed via `pending_selection` exactly as before (no auto-selection, no stubbing)

### Requirement: Nested and parked computations are frames
Pay-cost continuations, parked replacements, granted-effect bodies, and the effect queue SHALL be represented as typed interpreter frames (data), not boxed closures, so that nested multi-pick, pay-cost, and replacement flows are fully described by data.

#### Scenario: A paused pay-cost flow is data
- **WHEN** an effect is parked while a pay-cost or replacement sub-flow awaits input
- **THEN** the parked state is a typed frame on the interpreter's frame stack with no `Box<dyn FnOnce>`

### Requirement: Behavioral parity with the legacy executor
The data-VM SHALL reproduce the exact resolution behavior (ordering, nesting, timing, replacement outcomes) of the legacy closure executor across the full card pool, validated by the per-set behavioral suite, archetype interaction tests, and the DCGO recording parity harness.

#### Scenario: Migrated card matches its behavioral tests
- **WHEN** a card is migrated to the data-VM
- **THEN** its `cards_behavioral` tests and any archetype interaction tests pass unchanged

#### Scenario: VM matches the DCGO oracle
- **WHEN** a DCGO recording is replayed through the data-VM
- **THEN** the resolution matches the DCGO differential oracle as it did under the legacy executor
