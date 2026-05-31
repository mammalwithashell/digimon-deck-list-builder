## ADDED Requirements

### Requirement: Game is cloneable into an independent copy
`Game` SHALL implement `Clone`, producing a fully independent copy such that mutating the clone does not affect the original and vice versa.

#### Scenario: Clone is independent
- **WHEN** a `Game` is cloned and the clone is advanced by one or more actions
- **THEN** the original `Game` is unchanged

### Requirement: Clone replays identically
A cloned `Game` SHALL, given the same sequence of inputs (actions, selections, RNG), produce the same trajectory and terminal outcome as the original would have.

#### Scenario: Clone-then-replay equals original
- **WHEN** a `Game` is cloned at an arbitrary decision point and both copies are driven with the same input sequence
- **THEN** both reach an identical resulting state and outcome

### Requirement: Immutable state is shared, not deep-copied
Cloning a `Game` SHALL share the immutable registries (`card_data`, `effect_registry`, `formula_extensions`, `token_registry`, `alt_path_registry`, `rules`) by reference (`Arc`) rather than deep-copying them, while deep-copying mutable per-game data.

#### Scenario: Registries are shared on clone
- **WHEN** a `Game` is cloned
- **THEN** the immutable registries are reference-shared (no per-clone deep copy of `card_data` or the effect registry)

### Requirement: Behavior closures do not block clone
All retained behavior callbacks (modifier predicates and effects) SHALL be `Arc`-based shareable handles, so that no live behavior closure prevents `Game` from being `Clone`.

#### Scenario: Modifiers survive a clone
- **WHEN** a `Game` carrying active modifiers (DP buffs, granted keywords) is cloned
- **THEN** the clone carries equivalent active modifiers and applies them identically
