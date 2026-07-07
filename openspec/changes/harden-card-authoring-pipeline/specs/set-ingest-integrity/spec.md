# Spec: set-ingest-integrity

## ADDED Requirements

### Requirement: Ingest always ends override-applied
Every set pull/merge path (the author-set `pull_and_diff`/`merge_diff` flow and the legacy full-ingest CLI) SHALL apply `card_overrides.json` to the merged card data before writing `data/cards.json`, so a re-ingest can never leave the live data missing hand-maintained corrections.

#### Scenario: Re-pull preserves corrections
- **WHEN** a set is re-pulled from the API after its cards received override corrections (e.g. reconciled evo circles or Rule-granted traits)
- **THEN** the written `cards.json` reflects the overrides without a separate manual apply step

### Requirement: Override-regression guard
The merge SHALL refuse (with a diff report) any result in which a card present in `card_overrides.json` would end up with fewer `evo_costs` entries or fewer `type_eng` entries than its override specifies.

#### Scenario: Lossy API pull is caught
- **WHEN** a pulled record carries a single digivolve circle for a card whose override records two
- **THEN** the merge aborts for that card with a report naming the field and both values, rather than silently regressing

### Requirement: Gate ordering — lexicons before keyword gate
The new-set keyword gate SHALL run against trait/name lexicons that include the set being gated; the gate helper MUST refresh the lexicons when they predate the set's ingest.

#### Scenario: New trait tokens are not false keyword flags
- **WHEN** the keyword gate runs on a freshly ingested set that introduced new trait tokens
- **THEN** those tokens classify as traits (lexicon hits), not as flag-for-human keyword candidates

### Requirement: DUAL Option-face colors are face-verified
Entries added to the DUAL Option-face color override map SHALL cite verification against the printed card face (scan or official DB), and the ingest MUST fail loudly on a DUAL card with no verified entry rather than guessing.

#### Scenario: Unverified DUAL blocks ingest visibly
- **WHEN** a pulled set contains a DUAL card absent from the override map
- **THEN** the ingest reports the specific card and required verification instead of assigning a default color
