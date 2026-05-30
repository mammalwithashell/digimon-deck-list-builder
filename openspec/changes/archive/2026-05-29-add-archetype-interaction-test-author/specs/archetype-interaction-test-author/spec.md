## ADDED Requirements

### Requirement: Archetype Comprehension Skill

The system SHALL provide an `/archetype-interaction-test-author` skill that takes an archetype name (resolved via `code/tools/resolve_deck.py::resolve_archetype` over `data/deck_library.json` + `data/archetype_aliases.json`) or an explicit card list, researches the archetype, and produces interaction tests and static archetype tests. The skill SHALL be a capstone: it runs after the archetype's cards are implemented and per-card behavioral tests are green, and it SHALL NOT re-implement cards or author per-card tests.

#### Scenario: Skill resolves an archetype to a card pool

- **WHEN** the skill is invoked with an archetype name that exists in the deck library (or under a known alias)
- **THEN** it resolves the archetype to its unique-card pool with per-card text and play frequency, and proceeds to the research phase

#### Scenario: Skill accepts an explicit card list

- **WHEN** the skill is invoked with an explicit list of card IDs
- **THEN** it treats that list as the archetype pool without consulting the deck library

### Requirement: Archetype-Model Artifact

The skill SHALL emit a durable archetype-model document at `qa/archetype-qa/<archetype>-model.md` capturing its system-level understanding before any test is authored. The document SHALL include: the card pool with per-card roles (payoff / enabler / engine / tech), digivolution lines, named combos (each listing the cards involved, the expected mechanical outcome, and the rules/keyword basis), playstyle (archetype class, tempo, memory curve), win conditions, and a ranked list of interactions to test. Sources consulted SHALL be cited inline (DCGO C# location and/or `general_rule.pdf` rule number).

#### Scenario: Model documents named combos with expected outcomes

- **WHEN** the skill completes the research phase for an archetype with at least one multi-card combo
- **THEN** the model document names each combo, lists the cards involved, states the expected mechanical outcome, and cites the rules/keyword basis

#### Scenario: Model precedes test authoring

- **WHEN** the skill is run
- **THEN** the archetype-model document is produced and (optionally reviewed) before any interaction test file is written

### Requirement: Interaction Test Authoring

For each named combo in the model (up to a ranked cap), the skill SHALL author a DebugRunner interaction test that exercises the multiple cards together and asserts the combo's claimed mechanical outcome. Interaction tests SHALL live in a dedicated tree (`code/digimon-engine/tests/archetypes/<archetype_slug>.rs`), separate from per-card tests, and SHALL use shared multi-card fixture helpers. Each test SHALL be traceable to a specific combo in the archetype-model. When the ranked set of interactions is capped, the skill SHALL log what was dropped rather than silently truncating.

#### Scenario: Interaction test maps to a model combo

- **WHEN** the skill authors an interaction test
- **THEN** the test exercises the cards named in a model combo and asserts that combo's expected mechanical outcome, and the test is identifiable as covering that combo

#### Scenario: Capped interaction set is reported

- **WHEN** the model lists more candidate interactions than the authoring cap
- **THEN** the skill authors the top-ranked ones and logs the interactions it did not cover

### Requirement: Execution, Triage, and Findings Routing

The skill SHALL run the authored interaction tests and the static archetype tests, and SHALL treat a failure as a candidate engine bug. Before filing, it SHALL confirm the failure against the card's printed text, `general_rule.pdf`, and DCGO C#. Confirmed engine-primitive gaps SHALL be routed to `docs/RUST_ENGINE_GAPS.md` and confirmed card-effect faithfulness gaps to `qa/archetype-qa/engine-gaps.md`. Per-archetype outcomes (combos tested, pass/fail, static-gate results, findings filed) SHALL be recorded in `qa/qa-reports/archetype_interactions.json`. The skill SHALL NOT modify engine code as part of a run.

#### Scenario: Failing interaction test becomes a confirmed finding

- **WHEN** an authored interaction test fails and the skill confirms the discrepancy against card text + rules + DCGO C#
- **THEN** the skill files the finding in the appropriate tracker (engine-primitive vs card-effect) citing the combo, the test, and the source consulted, without editing engine code

#### Scenario: Run is recorded in the verdict tracker

- **WHEN** the skill completes a run for an archetype
- **THEN** `qa/qa-reports/archetype_interactions.json` records the archetype, the combos tested, their pass/fail status, the static-gate results, and any findings filed

### Requirement: Capstone Precondition Gating

Before authoring interaction tests, the skill SHALL run the coverage-gate and combo-presence static checks. If required cards are unimplemented, the skill SHALL report the missing pieces (routing them to the implementation backlog / gap trackers) and SHALL NOT author interaction tests that cannot pass for lack of those cards.

#### Scenario: Missing combo piece halts authoring for that combo

- **WHEN** a model combo names a card that is not yet implemented
- **THEN** the skill reports that combo as blocked on the missing card and does not author its interaction test

#### Scenario: Sufficiently-implemented archetype proceeds

- **WHEN** the coverage-gate and combo-presence checks pass for the targeted combos
- **THEN** the skill proceeds to author and run the interaction tests
