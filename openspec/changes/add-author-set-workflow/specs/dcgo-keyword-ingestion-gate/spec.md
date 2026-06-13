## ADDED Requirements

### Requirement: Candidate new keywords are detected by lexicon set-subtraction

The keyword gate SHALL detect candidate new keywords/mechanics in a set by scanning bracketed tokens in printed effect, inherited, and security text, and subtracting known card names, known traits, known timings, grammar tokens, and existing Rust `Keyword` variants. The gate SHALL denoise trait false-positives using a complete trait lexicon and the positional rule that a trait reference in effect text is followed by the literal word "trait".

#### Scenario: Bracketed tokens scanned and reduced

- **WHEN** the gate scans a set's printed text for `[…]` and `<…>` tokens
- **THEN** tokens matching a known card name, known trait, known timing, grammar token, or existing Rust keyword are removed
- **AND** the remaining tokens are the candidate new keywords

#### Scenario: Trait references denoised positionally

- **WHEN** a bracketed token is immediately followed by the word "trait" in the printed text
- **THEN** it is classified as a trait reference, not a candidate keyword
- **AND** if it is absent from the trait lexicon it is recorded as a lexicon-miss

#### Scenario: Numeric-parameter keywords are normalized

- **WHEN** a candidate token carries a numeric parameter (e.g. `App Fusion -4`)
- **THEN** the gate normalizes it to its base keyword form for matching against the Rust enum and the DCGO manifest

### Requirement: Each candidate is triaged against the DCGO keyword manifest

The keyword gate SHALL classify each candidate into exactly one of: already-covered by a Rust `Keyword` variant, a lexicon-miss (trait or card name), auto-ingestable (present in the DCGO keyword manifest), or flag-for-human (absent from DCGO). The DCGO manifest SHALL be a checked-in artifact, since DCGO has no central keyword enum and models keyword behavior across `I…Effect` interfaces, `CardEffectFactory` methods, and core mechanism files.

#### Scenario: Already-covered keyword is skipped

- **WHEN** a candidate normalizes to an existing Rust `Keyword` variant
- **THEN** it is marked covered and requires no action

#### Scenario: DCGO-implemented keyword is marked auto-ingest

- **WHEN** a candidate is absent from the Rust enum but present in the DCGO keyword manifest
- **THEN** it is marked for auto-ingestion

#### Scenario: DCGO-absent keyword is flagged

- **WHEN** a candidate is absent from both the Rust enum and the DCGO keyword manifest
- **THEN** it is flagged for human input

#### Scenario: Lexicon-miss patches the lexicon

- **WHEN** a candidate is determined to be a trait or card name missing from the lexicon
- **THEN** the gate patches the appropriate lexicon and continues without halting

### Requirement: DCGO keyword manifest and lexicons are maintained artifacts

The repository SHALL hold a checked-in DCGO keyword manifest enumerating DCGO's keyword surface (its `I…Effect` keyword interfaces, `CardEffectFactory/Add*.cs` factory methods, and curated core-mechanism files), plus trait and card-name lexicons used for denoising. These artifacts SHALL be refreshable from the base-repo DCGO checkout and SHALL be refreshed when the DCGO submodule is rebased.

#### Scenario: Manifest extracted from base-repo DCGO

- **WHEN** the manifest extractor runs against the base-repo DCGO checkout
- **THEN** it enumerates the union of both `KeyWordEffects/` directories (`CardEffectFactory/KeyWordEffects/*.cs` and `CardEffectCommons/KeyWordEffects/*.cs`) as the primary keyword registry
- **AND** supplements it with `I…Effect` interface names from `CardEffectInterfaces.cs`
- **AND** includes a curated allowlist of core-modeled keywords that appear in neither `KeyWordEffects/` directory (e.g. security-attack modifiers, draw, de-digivolve, digi-burst)
- **AND** normalizes DCGO keyword spellings to the Rust enum's spellings

#### Scenario: Neither keyword directory is treated as complete on its own

- **WHEN** a keyword appears in only one of the two `KeyWordEffects/` directories (e.g. `MindLink` in Commons only, `Link` in Factory only)
- **THEN** the extractor still includes it via the union
- **AND** does not drop it for being absent from the other directory

#### Scenario: Proactive gap candidates seeded from the registry diff

- **WHEN** the extractor diffs the DCGO keyword registry against the Rust `Keyword` enum
- **THEN** keywords DCGO implements but the engine lacks are recorded as standing auto-ingest candidates independent of any single set scan

#### Scenario: Lexicon completeness for denoising

- **WHEN** the trait lexicon is built
- **THEN** it includes every distinct trait across the full card database, not a per-set sample

#### Scenario: Manifest refreshed on rebase

- **WHEN** the DCGO submodule is rebased onto a newer upstream commit
- **THEN** the manifest is regenerated as part of the rebase checklist

### Requirement: Auto-ingestion ports DCGO behavior under TDD and acts as a barrier

When a candidate is auto-ingestable, the gate SHALL port its behavior from the DCGO C# reference into a Rust `Keyword` variant with DSL lowering, engine wiring, and a behavioral test that passes against the DCGO-faithful behavior, before any set card depending on that keyword is mass-implemented. Auto-ingestion SHALL apply only to keyword primitives, not to card-specific effects.

#### Scenario: Keyword ported with a green behavioral test

- **WHEN** a keyword is auto-ingested
- **THEN** a Rust `Keyword` variant, its DSL lowering, and its engine wiring are added
- **AND** a DebugRunner behavioral test asserting DCGO-faithful behavior passes before the barrier lifts

#### Scenario: Mass-implementation blocked until ingestion lands

- **WHEN** a set requires a not-yet-implemented but auto-ingestable keyword
- **THEN** mass-implementation does not begin until that keyword's ingestion is complete and tested

#### Scenario: Card-specific behavior is not ingested as a primitive

- **WHEN** a candidate flagged as a keyword turns out to be card-specific behavior
- **THEN** it is reclassified as a normal card clause for the per-card pipeline
- **AND** is not added as a `Keyword` primitive

#### Scenario: New player choice triggers action-space codegen

- **WHEN** an auto-ingested keyword exposes a new player choice
- **THEN** the action-space codegen and drift check are triggered rather than silently adding an action

### Requirement: Flagged keywords halt the run and request human direction

When a candidate is flagged for human input, the workflow SHALL halt set authoring, record the gap in the engine gap tracker with a planning stub, and request the context and direction needed to implement the keyword. The run SHALL NOT proceed to author cards that depend on a flagged keyword.

#### Scenario: Flagged keyword halts and reports

- **WHEN** the gate flags a keyword as absent from DCGO
- **THEN** set authoring halts
- **AND** the gap is recorded in `docs/RUST_ENGINE_GAPS.md` with a `.claude/plans/` stub
- **AND** the user is asked to supply context and direction

#### Scenario: Dependent cards are not authored on a flagged keyword

- **WHEN** a card's printed text requires a flagged keyword
- **THEN** that card is not mass-implemented until the keyword is resolved
