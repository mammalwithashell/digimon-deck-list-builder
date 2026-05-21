## ADDED Requirements

### Requirement: Every DNA Omnimon card has faithful DSL implementation

Every unique card in the DNA Omnimon decklist pool (as resolved from `data/deck_library.json`) SHALL have a production DSL YAML file under `code/digimon-engine/cards/<set>/` whose effects faithfully implement the full printed card text from `data/cards.json` — every clause, timing, and player choice. No clause may be omitted, stubbed, or auto-resolved. Where the printed text exposes a choice, that choice SHALL surface through `pending_selection` so the RL action space sees it (CLAUDE.md §17).

#### Scenario: Card pool fully authored

- **WHEN** the DNA Omnimon card pool is resolved from `data/deck_library.json`
- **THEN** each unique card ID has a corresponding `code/digimon-engine/cards/<set>/<CARD-ID>.yaml` file
- **AND** the previously unauthored cards BT22-084, BT17-007, ST2-13, BT5-093, and AD1-019 each have production YAML

#### Scenario: Card clauses are complete, not approximated

- **WHEN** a DNA Omnimon card's YAML is reviewed against its printed text in `data/cards.json`
- **THEN** every printed clause (main, inherited, security, when-digivolving, alt-path) is represented in the YAML
- **AND** no clause is replaced by a no-op, a hidden auto-selection, or a coarser proxy

### Requirement: Every DNA Omnimon card has behavioral test coverage

Every DNA Omnimon card SHALL have a behavioral test file under `code/digimon-engine/tests/cards_behavioral/<set>/` that exercises its card text via `DebugRunner`. Tests SHALL be written before or alongside the YAML they cover (TDD, CLAUDE.md §18).

#### Scenario: Behavioral test exists per card

- **WHEN** the DNA Omnimon card pool is enumerated
- **THEN** each card has a behavioral test file covering its printed clauses
- **AND** the test suites `cards_behavioral`, `dsl`, `dna_digivolve`, and `digivolve` pass with no regressions

### Requirement: No behavioral test is ignored for an already-closed gap

No DNA Omnimon behavioral test SHALL carry an `#[ignore]` marker that cites a substrate gap which is already closed in the current engine/DSL. Each `#[ignore]` marker that remains SHALL cite a substrate gap that is verifiably still open, confirmed by inspecting the current engine code — not by trusting a tracker document.

#### Scenario: Stale ignore markers are re-enabled

- **WHEN** a DNA Omnimon behavioral test is ignored citing `pending: G-XYZ`
- **AND** the engine/DSL primitive `G-XYZ` is confirmed present in `code/digimon-engine/src/` or `code/digimon-dsl/src/`
- **THEN** the test is re-enabled, its card clause is authored, and the test passes

#### Scenario: Genuinely-blocked tests carry accurate references

- **WHEN** a DNA Omnimon behavioral test remains ignored after the reconciliation sweep
- **THEN** its `#[ignore]` reason cites a substrate gap verified as still open against current code
- **AND** that gap has a corresponding open entry in `qa/dsl-vocab-gaps.md` or `docs/RUST_ENGINE_GAPS.md`

### Requirement: An accurate per-card verdict ledger exists

A `validated_cards_dsl.json` verdict ledger SHALL contain an entry for every DNA Omnimon card, with a verdict of `IMPLEMENTED`, `PARTIAL`, or `BLOCKED` that reflects the card's verified state — not a tracker-derived guess. `PARTIAL` and `BLOCKED` entries SHALL name the specific open gap.

#### Scenario: Ledger covers the full pool

- **WHEN** the reconciliation sweep completes
- **THEN** `validated_cards_dsl.json` has one entry per DNA Omnimon card
- **AND** every `PARTIAL` or `BLOCKED` entry names the open gap blocking it

### Requirement: raw_rust escapes are minimized and documented

`raw_rust` escapes in DNA Omnimon card YAML SHALL be reduced to those that cannot be expressed in the current DSL. Each remaining `raw_rust` escape SHALL be documented with the reason the DSL cannot express it.

#### Scenario: Now-expressible escapes are migrated

- **WHEN** a DNA Omnimon card YAML contains a `raw_rust` escape
- **AND** the behavior is expressible with current DSL vocabulary
- **THEN** the escape is rewritten as pure DSL and the card's behavioral test still passes

#### Scenario: Retained escapes are justified

- **WHEN** a `raw_rust` escape remains in a DNA Omnimon card after review
- **THEN** the card YAML or an associated tracker entry records why the DSL cannot express it

### Requirement: DNA Omnimon trackers reflect verified state

After the change, `qa/dsl-vocab-gaps.md`, `qa/resolved-gaps.md`, and `qa/archetype-qa/dsl/2026-05-03-dna-omnimon-dsl-engine-gaps.md` SHALL reflect the verified end state: closed gaps moved to `resolved-gaps.md`, still-open gaps left open with accurate card attributions.

#### Scenario: Closed gaps relocated

- **WHEN** a DNA Omnimon gap is verified closed during the change
- **THEN** its entry is moved to `qa/resolved-gaps.md` with a closure note
- **AND** the per-archetype gap doc annotates the closed item

#### Scenario: No closed gap left marked open

- **WHEN** the change completes
- **THEN** no DNA Omnimon gap that is verified closed remains listed as open in `qa/dsl-vocab-gaps.md`
