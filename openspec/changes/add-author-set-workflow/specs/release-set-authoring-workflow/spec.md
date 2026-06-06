## ADDED Requirements

### Requirement: Release set is resolvable and refreshed from source before authoring

The workflow SHALL resolve a release set by its set prefix (e.g. `BT17`, `EX12`) into its complete card-ID list, and SHALL refresh that data from `digimoncard.io` before any authoring begins. The workflow SHALL diff the freshly pulled set against `data/cards.json` and surface the diff to the user. The workflow SHALL NOT author cards against a stale or partial local snapshot without first ingesting the differences.

#### Scenario: Set prefix resolves to its full card list

- **WHEN** the workflow is invoked with a release-set prefix
- **THEN** it produces the set's complete distinct card-ID list (alternate-art printings collapsed to one entry per ID)

#### Scenario: Live pull diffed against local snapshot

- **WHEN** the set is pulled from `digimoncard.io` via the `?card=<PREFIX>` endpoint
- **THEN** the workflow diffs pulled card IDs and fields against `data/cards.json`
- **AND** reports added, removed, and changed cards to the user
- **AND** merges missing or changed cards into `data/cards.json` via the established ingest path before proceeding

#### Scenario: Stale snapshot is not silently authored

- **WHEN** the pulled set contains cards absent from `data/cards.json` (e.g. a partially-ingested newest set)
- **THEN** those cards are ingested first
- **AND** authoring proceeds only against the refreshed snapshot

#### Scenario: Source unreachable falls back loudly

- **WHEN** `digimoncard.io` is unreachable during the ingest-diff phase
- **THEN** the workflow emits an explicit warning that it is using the local snapshot unverified
- **AND** continues rather than failing, since settled sets match the source exactly

### Requirement: Set is decomposed into archetype/evolution slices plus a labeled orphan bucket

The workflow SHALL cluster the set's cards into archetype/evolution slices using multiple signals — card traits for membership, the in-text named-card reference graph for connectivity, and color plus level for intra-slice ordering. Cards belonging to no slice SHALL be placed in an explicitly labeled orphan-staples bucket rather than silently dropped. The slice partition SHALL be presented for user approval before mass-implementation.

#### Scenario: Cards group into slices

- **WHEN** the clusterer runs over the resolved set
- **THEN** it emits one or more slices, each a set of card IDs ordered by level from lowest digivolution stage to highest
- **AND** slices that intersect an existing `deck_library.json` archetype inherit that archetype's canonical name

#### Scenario: Orphan staples are labeled, not dropped

- **WHEN** a card matches no archetype/evolution slice
- **THEN** it is placed in the orphan-staples bucket
- **AND** is still scheduled for authoring in Phase 4

#### Scenario: Slice partition is approved before dispatch

- **WHEN** the slice partition is computed
- **THEN** it is presented to the user for confirmation before any implementation agents are dispatched

### Requirement: Slices are mass-implemented via the existing card-authoring skill in dependency order

The workflow SHALL author each slice's cards by invoking the existing `batch-implement-cards-rust-dsl` skill, processing cards within a slice in digivolution-stage order so prerequisites precede dependents. The workflow SHALL NOT reimplement card authoring.

#### Scenario: Slice authoring delegates to the existing skill

- **WHEN** a slice is dispatched for implementation
- **THEN** its cards are authored through `batch-implement-cards-rust-dsl` with the slice's card IDs
- **AND** lower-level cards in the slice are authored before higher-level cards that digivolve from them

#### Scenario: Orphan staples are authored, combo-tested case by case

- **WHEN** the orphan-staples bucket is processed
- **THEN** its cards are authored through the same skill
- **AND** each orphan staple is evaluated case by case for whether it warrants Phase 5 interaction coverage (e.g. a generic removal/tempo option that defines a line), rather than being categorically excluded

### Requirement: Non-orphan slices receive combo testing with lazy cross-set dependency pull

The workflow SHALL run combo/interaction testing per non-orphan slice via the existing `archetype-interaction-test-author` skill. Cross-set card implementations SHALL be pulled only when a slice's interaction test exercises that card's behavior and the card is not already implemented. The workflow SHALL NOT eagerly compute or author the transitive closure of cross-set references.

#### Scenario: Combo testing runs per slice

- **WHEN** a non-orphan slice has finished authoring
- **THEN** its interaction tests are authored through `archetype-interaction-test-author` scoped to that slice

#### Scenario: Evolution prerequisites use synthesized fixtures

- **WHEN** a slice's test needs a lower digivolution-stage card that lives in another set
- **THEN** the test synthesizes that prerequisite as a DebugRunner fixture rather than pulling the other set's card implementation

#### Scenario: Cross-set implementation pulled only on behavioral need

- **WHEN** a slice's interaction test fires the actual printed effect of a named card from another set
- **AND** that card is not already implemented
- **THEN** only that single card's implementation is pulled into scope
- **AND** no eager transitive closure of references is authored

### Requirement: Set authoring is gated on complete, tested coverage

The workflow SHALL conclude with a set-level coverage gate asserting that every card in the resolved set has reached an implemented verdict with passing behavioral tests, and SHALL record a set-level verdict and report. A set is not complete while any card is blocked, partial, or untested.

#### Scenario: Coverage gate verifies full set

- **WHEN** the set gate runs after all slices and orphans are processed
- **THEN** it asserts every resolved set card ID has an `IMPLEMENTED` verdict
- **AND** the engine's full behavioral test suite passes

#### Scenario: Set verdict tracked

- **WHEN** the set gate completes
- **THEN** a set-level entry recording per-card verdicts and the overall set status is written to the verdict tracker

#### Scenario: Blocked card prevents set completion

- **WHEN** any set card is left blocked, partial, or untested
- **THEN** the set gate reports the set as incomplete and enumerates the outstanding cards
