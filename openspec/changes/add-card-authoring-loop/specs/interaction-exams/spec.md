## ADDED Requirements

### Requirement: The gating interaction denominator is deterministic
The system SHALL derive each card's gating interactions solely from committed data — every official Q&A ruling that lists the card (`qa:<Q-number>`) and the probes a versioned, rule-based generator emits from the card's extracted clause text (`probe:<clause-id>:<family>[:neg]`) — so the same inputs always produce the same denominator, and a `--check` mode MUST fail on drift.

#### Scenario: Reproducible denominator
- **WHEN** the denominator is generated twice from the same `card_qa.json` and clause extraction
- **THEN** the outputs are identical

#### Scenario: Drift caught
- **WHEN** a ruling is added to `card_qa.json` without regenerating the committed denominator
- **THEN** `--check` fails naming the new interaction

#### Scenario: Model-authored combos do not gate
- **WHEN** an interaction comes from an archetype model document
- **THEN** it is examined and reported but is not part of any card's gating denominator

### Requirement: Risk probes come from versioned families including negative probes
The probe generator SHALL emit probes for the families optional-decline, scope, once-per-turn-with-multiple-copies, would-replacement, granted-keyword, leave-play, immunity and timing-gate, MUST include negative probes asserting that a clause does not fire where it should not, and SHALL make a new family gate only after explicit promotion.

#### Scenario: Scope negative probe
- **WHEN** a clause targets "this Digimon"
- **THEN** a negative probe is generated asserting another of the player's Digimon is unaffected

#### Scenario: Optional decline probe
- **WHEN** a clause says "you may"
- **THEN** a probe is generated exercising the decline path

#### Scenario: Unpromoted family
- **WHEN** a new family version is added but not promoted
- **THEN** its probes are generated and examinable but do not affect readiness

### Requirement: Q&A rulings are classified before examination
Each ruling SHALL be classified `behavioral`, `textual` or `not_examinable` before any oracle time is spent, and the `textual` and `not_examinable` classes MUST follow the two-family termination rule.

#### Scenario: Printed-data clarification
- **WHEN** a ruling only clarifies a card's name or traits
- **THEN** with both families agreeing it is classified `textual` and adjudicated against card data without an oracle run

### Requirement: Q&A interactions compare three ways
A scenario examining a ruling SHALL carry an `expect_ruling:` assertion encoding the publisher's answer with its Q-number, agreed by both model families, and its outcome MUST be decided by comparing our engine, DCGO and the ruling: all agree → `confirmed`; ours matches DCGO but not the ruling → `ours_wrong`; ours matches the ruling but not DCGO → `dcgo_quirk` citing the Q-number; ours matches neither → `ours_wrong`.

#### Scenario: Both engines contradict the ruling
- **WHEN** our engine and DCGO agree with each other and contradict the ruling's assertion
- **THEN** the item is `ours_wrong` and the DCGO disagreement is logged as a DCGO fork candidate

#### Scenario: Our engine matches the ruling and DCGO does not
- **WHEN** our engine matches the ruling and DCGO diverges
- **THEN** the item may end `dcgo_quirk` citing `qa:<Q-number>`, subject to the two-family rule, and is listed as a DCGO fork candidate

### Requirement: Scenarios can cover several clauses and name their interaction
The exam scenario format SHALL accept `covers: [clause ids]` (defaulting to the single `clause`) and an `interaction: {id, source, kind: positive|negative}` block, existing single-clause scenarios MUST remain valid unchanged, and validation MUST reject an interaction id that is not in the denominator.

#### Scenario: Legacy scenario
- **WHEN** an existing scenario with only `card` and `clause` is validated
- **THEN** it passes and covers exactly its clause

#### Scenario: Orphan interaction
- **WHEN** a scenario names an interaction id absent from the denominator
- **THEN** validation fails naming the orphan id

### Requirement: Interaction exams are authored adversarially
An interaction exam SHALL be authored by a different model family than the one that implemented the card it targets, its task packet MUST frame the goal as finding a divergence, and where possible its line SHALL extend an oracle-confirmed clause line from the scenario library.

#### Scenario: Implementer excluded
- **WHEN** a card's YAML was produced by a Claude attempt
- **THEN** its interaction exams are routed to Codex, outside the randomized exploration share

### Requirement: Readiness requires adjudicated gating interactions
A card SHALL be `ready` for training only when every printed clause and every gating interaction of the card is adjudicated, where `escalated` does not count; a card with no rulings and no generated probes MUST remain `ready` on its clauses alone.

#### Scenario: Unexamined ruling blocks readiness
- **WHEN** all of a card's clauses are confirmed but one of its Q&A rulings is unmeasured
- **THEN** the card is `not_ready` and the ruling is listed among its blockers

#### Scenario: Shared ruling adjudicated once
- **WHEN** a ruling listing two cards is confirmed
- **THEN** it counts as adjudicated for both cards
