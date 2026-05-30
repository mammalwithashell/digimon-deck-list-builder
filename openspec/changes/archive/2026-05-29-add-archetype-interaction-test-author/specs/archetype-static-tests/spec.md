## ADDED Requirements

### Requirement: Deck Legality and Construction Test

The system SHALL provide a static test that, for a given archetype, takes its best/meta decklist(s) from the resolved manifest and asserts the deck builds into a rules-legal deck and that the engine constructs a game from it without error. A decklist that fails rules validation (size, copy limits, egg-deck constraints) or engine construction SHALL be reported with the specific violation.

#### Scenario: Legal archetype deck constructs

- **WHEN** the deck-legality test runs on an archetype whose meta decklist is rules-legal and uses implemented cards
- **THEN** the test asserts a legal deck and a successful game construction

#### Scenario: Illegal or unconstructable deck is reported

- **WHEN** the decklist violates a deck-construction rule or the engine cannot construct a game from it
- **THEN** the test fails and names the specific violation (size / copy limit / egg constraint / construction error)

### Requirement: Coverage Gate Test

The system SHALL provide a static test that cross-references an archetype's unique-card pool against `qa/qa-reports/validated_cards_dsl.json` and asserts that all (or a configurable threshold of) the archetype's cards are implemented and have passing per-card behavioral tests. Cards whose status is absent from the tracker SHALL be reported as "unknown" rather than counted as passing. An archetype below the threshold SHALL be reported, not silently passed.

#### Scenario: Fully-implemented archetype passes the gate

- **WHEN** every unique card in the archetype is marked implemented with passing per-card tests in the verdict tracker
- **THEN** the coverage gate passes

#### Scenario: Sub-threshold archetype is reported

- **WHEN** some of the archetype's cards are unimplemented or lack passing per-card tests
- **THEN** the gate reports the specific missing/failing cards and the computed coverage ratio, and does not silently pass

#### Scenario: Unknown-status card is not counted as passing

- **WHEN** a card in the pool has no entry in `validated_cards_dsl.json`
- **THEN** it is reported as "unknown" and excluded from the passing count

### Requirement: Per-Archetype Smoke Games Test

The system SHALL provide a static test that plays N self-play games using the archetype's deck and asserts each runs to completion without a panic or illegal-state error. The smoke test SHALL be a liveness gate only and SHALL NOT be treated as a correctness check.

#### Scenario: Smoke games complete cleanly

- **WHEN** N self-play games are run on the archetype's deck and none panic or reach an illegal state
- **THEN** the smoke test passes

#### Scenario: Panic during smoke is surfaced

- **WHEN** any smoke game panics or hits an illegal state
- **THEN** the test fails and reports the game seed / step at which the failure occurred

### Requirement: Combo-Presence Test

The system SHALL provide a static test that asserts every card named in the archetype-model's combos is implemented, so that interaction tests can be authored. A missing combo piece SHALL be reported as a blocker on the specific combo.

#### Scenario: All combo pieces present

- **WHEN** every card referenced by the model's combos is implemented
- **THEN** the combo-presence test passes for all combos

#### Scenario: Missing combo piece blocks the combo

- **WHEN** a card named in a combo is not implemented
- **THEN** the test reports that combo as blocked and names the missing card

### Requirement: Static-Test Harness and Verdict Tracking

The four static tests SHALL be runnable independently of the authoring skill (CI-able) and SHALL record per-archetype results in `qa/qa-reports/archetype_interactions.json`. The harness SHALL accept an archetype name and produce a structured result per invariant (pass / fail / details) without requiring a full interactive game step-through.

#### Scenario: Harness runs standalone for an archetype

- **WHEN** the static-test harness is invoked with an archetype name
- **THEN** it runs all four invariant checks and emits a structured per-invariant result recorded in the verdict tracker
