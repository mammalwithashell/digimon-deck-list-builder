## ADDED Requirements

### Requirement: Machine-generated vocabulary reference covers every DSL enum member

The DSL authoring guide SHALL contain a vocabulary reference, generated from the `digimon-dsl` enums, that includes one entry for every member of `StepSpec`, `PredicateSpec`, `Timing`, and `DeclarativeKind`. Each entry SHALL record the YAML key, its argument shape, its source doc-comment (when present), its usage count in the card corpus, and a fixture card path (when any card uses it). The generated content SHALL be delimited by stable markers so that hand-written guide content is preserved across regeneration.

#### Scenario: Every enum member has a reference row

- **WHEN** the vocabulary-reference exporter runs against the current `digimon-dsl` enums
- **THEN** the generated reference contains exactly one row per `StepSpec` / `PredicateSpec` / `Timing` / `DeclarativeKind` member
- **AND** each row carries the YAML key and argument shape for that member

#### Scenario: Regeneration preserves curated prose

- **WHEN** the exporter rewrites the guide's generated block
- **THEN** only content between the generated-block markers changes
- **AND** the curated narrative sections (workflow, patterns, red flags) are byte-for-byte unchanged

#### Scenario: Reference is deterministic and idempotent

- **WHEN** the exporter is run twice in succession with no source change
- **THEN** the second run produces no diff

### Requirement: A drift gate keeps the reference in sync with the enums

CI SHALL re-run the exporter and fail the build when the committed vocabulary reference's structural content (the set of keys, families, argument shapes, and doc-comments) differs from what the current enums produce. Advisory metadata that changes with ordinary card authoring (usage counts, fixture paths) SHALL NOT by itself fail the gate.

#### Scenario: New enum variant without a doc row fails CI

- **WHEN** a new `StepSpec` (or predicate / timing / kind) variant is added but the generated reference is not regenerated
- **THEN** the drift gate detects a structural difference and fails the build

#### Scenario: Adding a card that uses an existing verb does not fail CI

- **WHEN** a new card YAML increases a verb's usage count but introduces no new enum member
- **THEN** the drift gate does not fail solely because of the count change

### Requirement: The curated narrative is usage-aware

The vocabulary reference SHALL expose each member's card-corpus usage, and members with zero corpus usage SHALL be tagged as unused. The curated narrative sections SHALL illustrate idioms using vocabulary that is actually used by cards, and SHALL NOT foreground zero-usage vocabulary as a recommended path.

#### Scenario: Zero-usage verbs are flagged

- **WHEN** a documented verb has no uses in the card corpus
- **THEN** its reference row is tagged unused

#### Scenario: Live vocabulary is reachable from idioms

- **WHEN** an authoring agent reads a pattern section for a mechanic that has live vocabulary (e.g. under-tamer source movement, Link/AppFuse, DigiXros)
- **THEN** the section names that live vocabulary or points to its reference rows, rather than implying the mechanic is unsupported
