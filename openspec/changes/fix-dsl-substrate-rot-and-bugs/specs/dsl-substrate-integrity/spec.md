## ADDED Requirements

### Requirement: Unregistered `raw_rust` references fail card load

The engine SHALL validate, when loading the card pack, that every `raw_rust` reference (declarative fn, step fn, and formula fn) named by a card resolves to a registered implementation. An unregistered name SHALL be a hard load error, not a silent no-op or debug-only warning.

#### Scenario: Unregistered raw_rust formula fails load

- **WHEN** a card declares `amount_fn: { raw_rust: <name> }` (or any `raw_rust` step/declarative) whose `<name>` is not in the engine registry
- **THEN** card-pack load fails with an error naming the card and the missing reference
- **AND** the effect never silently resolves to a zero/no-op value at runtime

#### Scenario: Registered references load normally

- **WHEN** every `raw_rust` reference a card uses is registered
- **THEN** the pack loads successfully

### Requirement: Lowest/highest-metric deletion exposes the tie choice

An effect that deletes (or otherwise targets) "the <Digimon> with the lowest/highest <metric>" SHALL surface the eligible minimum/maximum set through `pending_selection` when more than one candidate ties at the extreme value. The engine SHALL NOT auto-pick a tied candidate (e.g. by battle-area index).

#### Scenario: Multiple candidates tie at the lowest metric

- **WHEN** an effect deletes "1 of your opponent's Digimon with the lowest play cost" and two or more opponent Digimon share the lowest play cost
- **THEN** a `pending_selection` offers exactly those tied candidates and the controlling player chooses which one is deleted

#### Scenario: A single candidate is unambiguous

- **WHEN** exactly one Digimon has the lowest play cost
- **THEN** that Digimon is the only legal target (a single-option selection is acceptable, but no other candidate is auto-substituted)

### Requirement: Card-header gap citations cannot reference resolved gaps

A CI check SHALL fail when a card YAML header cites a gap identifier that is listed as resolved in `qa/resolved-gaps.md`, so that stale "BLOCKED / pending <gap>" header annotations cannot persist after the underlying gap closes.

#### Scenario: Header cites a resolved gap

- **WHEN** a card YAML header references a gap ID that appears in `qa/resolved-gaps.md`
- **THEN** the CI check fails and names the card and the resolved gap ID

#### Scenario: Header cites only open gaps

- **WHEN** a card header references only gap IDs that are still open (or none)
- **THEN** the CI check passes
