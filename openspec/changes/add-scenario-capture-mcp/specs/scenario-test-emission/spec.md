## ADDED Requirements

### Requirement: Playwright spec generation from a fixture

The project SHALL provide a generator that, given a scenario fixture, writes the fixture under `qa/scenarios/` (the source of truth) and scaffolds a matching Playwright `.spec.ts` under `code/frontend/e2e/`. The generated spec MUST stage the fixture via the `/debug` surface, drive the UI through the existing e2e page objects/helpers, and assert the fixture's expectations through the existing evaluation surface. The generator MUST be idempotent — regenerating from the same fixture produces the same spec — and the generated file MUST carry a banner marking it generated so hand-edits are directed to the fixture instead.

#### Scenario: Generated spec stages and asserts

- **WHEN** a fixture with at least one assertion is passed to the generator
- **THEN** a `.spec.ts` is written that loads that fixture, stages it via `/debug`, drives the UI, and asserts the fixture's expectations, and the spec runs within the existing e2e suite

#### Scenario: Fixture is canonical, spec is regenerable

- **WHEN** the same fixture is passed to the generator twice
- **THEN** the generated spec is identical both times, and the fixture (not the generated TS) is the artifact a reviewer edits to change the scenario

### Requirement: End-to-end capture-to-test loop

It SHALL be possible to go from a staged or captured board to a durable, runnable test without hand-playing a game: stage or capture → optionally drive to a decision point → add assertions → save fixture + emit spec → run. This loop MUST work end-to-end using the MCP tools and the generator.

#### Scenario: Capture an existing board as the proof case

- **WHEN** an existing scenario board (e.g. the Q16 Paildramon staging) is reached, captured via the capture primitive, assertions are added, and a spec is emitted
- **THEN** the emitted fixture re-stages to the identical board and the generated spec passes against the same expectations as the hand-authored equivalent
