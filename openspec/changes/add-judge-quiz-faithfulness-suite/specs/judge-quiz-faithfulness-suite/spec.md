## ADDED Requirements

### Requirement: All 30 judge-quiz scenarios are encoded as behavioral tests

Every one of the 30 TCG-Judges'-quiz questions SHALL be reproduced as a behavioral test under `code/digimon-engine/tests/judge_quiz/`, built with `DebugRunner` from the question's board state and asserting the official judge-correct outcome. Each test's docstring SHALL quote the question, the judge answer, the `docs/RULES_CONTEXT.md` citation, and the `DCGO/` reference for the processing-order detail.

#### Scenario: Every question has a test

- **WHEN** the judge-quiz suite is enumerated
- **THEN** there is exactly one behavioral test per question Q1–Q30 (a question spanning two clusters, e.g. Q30, has one test)
- **AND** each test's docstring cites the question text, the judge answer, the rules citation, and the DCGO reference

#### Scenario: Tests assert the judge-correct outcome

- **WHEN** a judge-quiz test runs
- **THEN** it asserts the official correct answer from the quiz, not the engine's current output
- **AND** it does not weaken its assertion to match incorrect engine behavior

### Requirement: Discover-then-pin discipline

A judge-quiz test that fails because the engine produces the wrong outcome SHALL be treated as a discovered faithfulness gap: the failure SHALL be logged to `qa/archetype-qa/engine-gaps.md` with the question, expected-vs-actual, the rule cluster, and the DCGO citation. The test SHALL NOT be `#[ignore]`-d to hide the failure. An `#[ignore]` marker is permitted ONLY when a scenario is blocked on an unimplemented card or a missing engine primitive, and SHALL cite that specific blocker.

#### Scenario: Engine disagreement is logged, not hidden

- **WHEN** a judge-quiz test asserts the judge answer and the engine produces a different outcome
- **THEN** the test fails (or is implemented to fail) and an `engine-gaps.md` entry records the discrepancy with its DCGO citation
- **AND** the assertion is not changed to match the engine, and the test is not silently ignored

#### Scenario: Ignore markers cite a real blocker

- **WHEN** a judge-quiz test carries an `#[ignore]` marker
- **THEN** the marker cites a specific unimplemented card or missing engine primitive
- **AND** no `#[ignore]` marker cites a gap that is already closed in the current engine

### Requirement: Every referenced card is faithfully implemented

Every card referenced by a judge-quiz scenario SHALL have a production DSL YAML (or hand-written Rust effect) faithfully implementing its full printed text from `data/cards.json` — every clause, timing, and player choice, with each choice surfaced through `pending_selection` (CLAUDE.md §17) — together with its own per-card behavioral test. No card is admitted to a scenario test as a quiz-scoped subset of its text.

#### Scenario: Referenced card fully authored before its scenario is pinned

- **WHEN** a judge-quiz scenario test references a card
- **THEN** that card has a faithful full-text implementation and a per-card behavioral test
- **AND** the scenario test composes the real card, not a stand-in or partial implementation

#### Scenario: Card unavailable in data is marked BLOCKED-DATA

- **WHEN** a referenced card cannot be resolved to an entry in `data/cards.json` / `card_overrides.json` (e.g. a printing that does not exist in the pool)
- **THEN** its scenario is recorded as `BLOCKED-DATA` in the verdict ledger with the missing card named
- **AND** its test carries an `#[ignore]` citing the `BLOCKED-DATA` blocker

### Requirement: Tests organized by rule cluster

The judge-quiz tests SHALL be organized into modules by the rule cluster each scenario exercises: A immunity scope, B deferred rules-check, C declare-then-pay cost, D trigger activation site, E `<Partition>`/DigiXros departure & de-digivolve, F token lifecycle & memory arithmetic, G zone/keyword scoping. A test name SHALL identify its question.

#### Scenario: Cluster modules exist and tests are named by question

- **WHEN** the `tests/judge_quiz/` tree is inspected
- **THEN** there is one module per cluster A–G
- **AND** each test name identifies its question number (e.g. `q1_...`, `q17_...`)

### Requirement: Per-question verdict ledger reconciled to test reality

`qa/qa-reports/judge-quiz.md` SHALL record, for every question Q1–Q30, the resolved card(s), the rule cluster, the verdict (PASS / BLOCKED-DATA), the test path, and the DCGO reference. The ledger SHALL match `cargo test --test judge_quiz` results.

#### Scenario: Ledger matches the suite

- **WHEN** the judge-quiz ledger is compared against `cargo test --test judge_quiz`
- **THEN** every question marked PASS has a passing test
- **AND** every question marked BLOCKED-DATA has an `#[ignore]`-d test citing the named missing card
- **AND** no question is unaccounted for

### Requirement: Gaps surfaced by the suite are reconciled

Every rules-engine gap surfaced by the judge-quiz suite SHALL be either fixed (failing test → minimal primitive → green test) or recorded as genuinely open in `qa/archetype-qa/engine-gaps.md`, confirmed against current engine source. Gaps closed by this change SHALL be moved to `qa/resolved-gaps.md` with a resolution note and test command.

#### Scenario: Closed gaps are archived with evidence

- **WHEN** a gap surfaced by a judge-quiz scenario is closed
- **THEN** its entry is moved to `qa/resolved-gaps.md` with a resolution note and the test command that proves it
- **AND** the corresponding judge-quiz test passes

#### Scenario: Open gaps are confirmed against source

- **WHEN** a judge-quiz scenario remains failing at change completion
- **THEN** its `engine-gaps.md` entry is confirmed open by inspecting current engine code (not by trusting a tracker)
- **AND** the change does not claim the scenario as pinned
