## ADDED Requirements

### Requirement: Declarative scenario fixture schema

The project SHALL define a declarative JSON scenario fixture format that fully describes a staged game and its expected outcomes. A fixture MUST contain: a schema-version field; per-player decks; per-player zone staging (hand, deck order, field stacks with suspended/turn-played, breeding, security, trash); initial scalar state (memory, phase, turn, first player); an optional ordered action script of action ids to apply after staging; and a list of expected assertions to evaluate. The schema MUST be documented so that fixtures are hand-authorable and reviewable.

A fixture MAY reach its starting state by either entry point: direct board injection (the `/debug` staging surface) or decision replay (a `seed` plus an `action_script` of human action ids applied through the live `/games` create path). When an `action_script` is used, each scripted action MUST be validated as legal before it is applied, so an incorrect script fails loudly rather than silently producing a divergent board.

#### Scenario: A fixture round-trips into a staged game
- **WHEN** a valid scenario fixture is loaded and applied to a debug game
- **THEN** the resulting game state matches the fixture's declared zones and scalar state exactly before any assertions run

#### Scenario: Invalid fixture is rejected with a diagnostic
- **WHEN** a fixture references a card id that does not exist, or declares a field stack that violates engine rules
- **THEN** loading fails with an error identifying the offending field, rather than producing an undefined game state

### Requirement: Assertion vocabulary

The fixture format SHALL support a defined vocabulary of expected assertions sufficient to encode rules-quiz outcomes: exact memory value; the top card id of a named permanent's stack; the effective DP of a named permanent; the contents (or count) of a named zone; whether a named effect/event triggered during resolution; whether a given action id is currently legal; and the set of legal options for an outstanding selection. Each assertion MUST produce a pass/fail result with a human-readable message on failure.

#### Scenario: Memory-value assertion
- **WHEN** a fixture asserts memory equals a value and the staged-and-stepped game's memory matches
- **THEN** the assertion passes; if it does not match, the failure message reports expected vs actual

#### Scenario: Effect-triggered assertion
- **WHEN** a fixture asserts a named effect did NOT trigger (e.g. Partition on self-deletion) and no corresponding event was emitted during resolution
- **THEN** the assertion passes

#### Scenario: Legal-action assertion
- **WHEN** a fixture asserts a DNA digivolve action is legal for a hand card and the action mask has that action id set
- **THEN** the assertion passes

### Requirement: Single fixture consumable by both test layers

A scenario fixture SHALL be consumable by both a Rust headless runner and the Playwright UI fixture from the same file, so that engine-correctness and UI-wiring are verified against one source of truth. The fixture format MUST NOT embed assumptions specific to either runner.

#### Scenario: Same fixture drives both runners
- **WHEN** a fixture for Q16 (Paildramon over ExVeemon + Stingmon) is run by the Rust headless runner and by the Playwright UI fixture
- **THEN** both stage the identical board, and each evaluates the fixture's assertions through its own layer (engine state vs rendered UI affordances)

### Requirement: Fixtures tagged by implementation readiness

Each scenario fixture in the seed corpus SHALL carry a tag indicating whether it is expected to pass against the current engine or is blocked on unimplemented card behavior. The test harness MUST treat blocked fixtures as known-pending (not hard failures) while still reporting them, so the corpus can grow ahead of card implementation without breaking CI.

#### Scenario: Blocked fixture does not fail the suite
- **WHEN** a fixture tagged blocked-on-card-impl is executed and its assertions do not yet pass
- **THEN** the run records it as pending rather than failing, and surfaces it in the summary as outstanding work
