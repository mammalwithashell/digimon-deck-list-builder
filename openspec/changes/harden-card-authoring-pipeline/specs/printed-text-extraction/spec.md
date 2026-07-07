# Spec: printed-text-extraction

## ADDED Requirements

### Requirement: Per-card printed-authority files
Each onboarded set SHALL have a committed `code/digimon-engine/cards/<set>/<ID>.printed.md` per card containing the verbatim printed text extracted from the card scan — effect / inherited / security boxes, keywords, digivolve circles (color, level, cost), alt-play boxes, and DUAL faces — plus a rulings section mirrored from the card's official Q&A or wiki rulings where available. Extraction MUST be verbatim; paraphrase is prohibited.

#### Scenario: EX12 corpus exists before authoring waves
- **WHEN** the EX12 Shambala/Virus Busters implementation waves dispatch
- **THEN** every in-scope card has a `.printed.md` and the implementer/reviewer prompts cite it as the printed authority (with the scan as tiebreak)

#### Scenario: Verbatim fidelity
- **WHEN** an extraction file's text is compared against the card scan
- **THEN** clause text matches the printed wording exactly (modulo Unicode bracket normalization), and any illegible region is marked as such rather than guessed

### Requirement: Extraction quality gate
Each set's extraction pass SHALL include an independent spot-check over a sample of cards (comparing `.printed.md` against the scans) before the corpus is treated as authoritative, and any mismatch found later SHALL be fixed in the file through the normal review loop.

#### Scenario: Spot-check gates the corpus
- **WHEN** the extraction wave completes for a set
- **THEN** a sample (at least one card in five) is re-verified against scans by a different agent, and the corpus is only cited as authority after the sample passes

### Requirement: API text demoted to stats
Where a `.printed.md` exists, downstream authoring prompts SHALL treat the digimoncard.io-derived per-card JSON as authoritative for numeric stats only (cost, DP, level, colors); effect text in the JSON is a hint whose divergence from the printed file MUST be flagged for `card_overrides.json` reconciliation.

#### Scenario: Divergence surfaces instead of propagating
- **WHEN** an implementer notices the JSON effect text disagrees with the printed file
- **THEN** the implementation follows the printed file and the divergence is reported in the worker manifest for reconciliation
