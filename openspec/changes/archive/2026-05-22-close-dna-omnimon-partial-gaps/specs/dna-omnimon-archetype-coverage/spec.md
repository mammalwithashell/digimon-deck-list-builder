## MODIFIED Requirements

### Requirement: Every DNA Omnimon card has faithful DSL implementation

Every unique card in the DNA Omnimon decklist pool (as resolved from `data/deck_library.json`) SHALL have a production DSL YAML file under `code/digimon-engine/cards/<set>/` whose effects faithfully implement the full printed card text from `data/cards.json` — every clause, timing, and player choice. No clause may be omitted, stubbed, hidden behind `raw_rust`, auto-resolved, or represented by a coarser proxy.

#### Scenario: Card pool fully authored

- **WHEN** the DNA Omnimon card pool is resolved from `data/deck_library.json`
- **THEN** each unique card ID has a corresponding `code/digimon-engine/cards/<set>/<CARD-ID>.yaml` file
- **AND** the previously unauthored cards BT22-084, BT17-007, ST2-13, BT5-093, and AD1-019 each have production YAML

#### Scenario: Card clauses are complete, not approximated

- **WHEN** a DNA Omnimon card's YAML is reviewed against its printed text in `data/cards.json`
- **THEN** every printed clause (main, inherited, security, when-digivolving, alt-path) is represented in the YAML
- **AND** no clause is replaced by a no-op, a hidden auto-selection, a `raw_rust` escape, or a coarser proxy

#### Scenario: BT17-102 dynamic source names are implemented

- **WHEN** BT17-102 Greymon has level 3 or lower cards in its digivolution cards
- **THEN** the engine treats that Digimon as having all names of those source cards for relevant name checks
- **AND** the DSL implementation does not rely on a hardcoded Koromon-source proxy for the all-turns name behavior

#### Scenario: BT23-096 Delay fires from ally CS attack

- **WHEN** BT23-096 Comet Hammer is in the battle area as a delayed option during the player's turn
- **AND** one of that player's `[CS]` trait Digimon attacks
- **THEN** the Delay effect can be declared through normal pending-selection/action-mask flow
- **AND** resolving it trashes BT23-096 and performs the printed de-digivolve effect

### Requirement: Every DNA Omnimon card has behavioral test coverage

Every DNA Omnimon card SHALL have a behavioral test file under `code/digimon-engine/tests/cards_behavioral/<set>/` that exercises its card text via `DebugRunner`. Tests SHALL be written before or alongside the YAML they cover, and the previously partial BT17-102 and BT23-096 clauses SHALL have enabled behavioral coverage.

#### Scenario: Behavioral test exists per card

- **WHEN** the DNA Omnimon card pool is enumerated
- **THEN** each card has a behavioral test file covering its printed clauses
- **AND** the test suites `cards_behavioral`, `dsl`, `dna_digivolve`, and `digivolve` pass with no regressions

#### Scenario: Partial-card tests are enabled

- **WHEN** the change is complete
- **THEN** BT17-102's dynamic source-name alias test is not ignored
- **AND** BT23-096's Delay-on-ally-attack test is not ignored
- **AND** both tests pass against production DSL YAML

### Requirement: An accurate per-card verdict ledger exists

A `validated_cards_dsl.json` verdict ledger SHALL contain an entry for every DNA Omnimon card, and every entry SHALL have a verdict of `IMPLEMENTED` after the change completes. No DNA Omnimon entry may remain `PARTIAL` or `BLOCKED` after BT17-102 and BT23-096 pass their behavioral tests.

#### Scenario: Ledger covers the full pool

- **WHEN** the reconciliation sweep completes
- **THEN** `validated_cards_dsl.json` has one entry per DNA Omnimon card
- **AND** every entry has verdict `IMPLEMENTED`

#### Scenario: Former partial cards are promoted

- **WHEN** BT17-102 and BT23-096 behavioral tests pass with their omitted clauses enabled
- **THEN** their ledger entries are updated from `PARTIAL` to `IMPLEMENTED`
- **AND** their former gap IDs are recorded as closed in the appropriate tracker updates

### Requirement: raw_rust escapes are minimized and documented

DNA Omnimon card YAML SHALL contain zero live `raw_rust` escapes. Historical comments may mention retired raw-Rust migrations, but no DNA Omnimon production YAML may use `kind: raw_rust` or reference a raw Rust function to implement card behavior.

#### Scenario: Now-expressible escapes are migrated

- **WHEN** a DNA Omnimon card YAML contains a live `raw_rust` escape
- **AND** the behavior is expressible with current DSL vocabulary
- **THEN** the escape is rewritten as pure DSL and the card's behavioral test still passes

#### Scenario: No live raw_rust remains

- **WHEN** the DNA Omnimon card pool YAML files are scanned
- **THEN** no non-comment YAML entry contains `kind: raw_rust`
- **AND** no DNA Omnimon card behavior depends on a raw Rust card-function registry entry
