## ADDED Requirements

### Requirement: raw_rust effects must be clone-safe
A hand-written `raw_rust` `CardEffect` SHALL be clone-safe: either atomic (it installs no mid-effect player selection) or it provides an explicit `Clone`-able resume-state implementing the interpreter frame contract. No `raw_rust` effect may leave a non-`Clone` boxed continuation parked on `Game`.

#### Scenario: Atomic raw_rust effect is allowed
- **WHEN** a `raw_rust` effect resolves fully without parking a player selection
- **THEN** it is clone-safe and permitted

#### Scenario: Selection-driving raw_rust effect must provide resume-state
- **WHEN** a `raw_rust` effect needs a mid-effect player selection
- **THEN** it SHALL encode its continuation as a `Clone`-able resume-state frame rather than a boxed closure, or be expressed in the DSL instead

### Requirement: Clone-safety is enforced and documented
The clone-safety constraint SHALL be enforced by a guard test or lint, and CLAUDE.md rule 28 SHALL document it so authors know the escape hatch is constrained.

#### Scenario: Guard rejects an unsafe raw_rust effect
- **WHEN** a `raw_rust` effect parks a non-`Clone` continuation on `Game`
- **THEN** the guard test or lint fails, naming the offending effect

#### Scenario: Rule 28 documents the constraint
- **WHEN** an author consults CLAUDE.md rule 28
- **THEN** it states that raw_rust effects must be clone-safe (atomic or resume-state-providing)
