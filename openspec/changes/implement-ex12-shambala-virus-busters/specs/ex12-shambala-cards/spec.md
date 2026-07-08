# Spec: ex12-shambala-cards

## ADDED Requirements

### Requirement: Shambala slice fully implemented
All 33 Shambala-slice EX12 cards (EX12-002, -004, -006, -009, -011, -012, -015, -019, -020, -022, -025, -026, -029, -031, -034, -036, -039, -043, -045, -046, -047, -048, -056, -057, -061, -062, -063, -065, -070, -071, -074, -075, -076) SHALL be implemented as YAML DSL specs in `code/digimon-engine/cards/ex12/` with every printed clause faithfully modeled per the no-approximations policy: no stubs, no auto-selections, every player choice exposed through the pending-selection surface.

#### Scenario: Card pool coverage
- **WHEN** the slice is complete
- **THEN** each of the 33 card IDs has a YAML spec that compiles into the embedded registry and a verdict entry in `qa/qa-reports/validated_cards_dsl.json` (IMPLEMENTED, or PARTIAL citing a real tracker entry)

#### Scenario: Printed-scan authority
- **WHEN** the per-card JSON text diverges from the card scan
- **THEN** the implementation follows the scan, and the divergence is reconciled into `data/card_overrides.json`

### Requirement: Shambala per-card behavioral coverage
Each implemented Shambala card SHALL have a DebugRunner behavioral test suite in `code/digimon-engine/tests/cards_behavioral/ex12/` covering: structural clause shape, a positive and negative path per condition, decline paths for every optional choice, once-per-turn lockouts where printed, and event-log assertions for costs.

#### Scenario: Suite green at wave merge
- **WHEN** a wave containing the card merges
- **THEN** `cargo test --test cards_behavioral -- <card_id_lower>` passes with zero failures and zero unexplained ignores

### Requirement: New token species registered
Token species summoned by Shambala cards (at minimum [Paishu] — Yellow, 6000 DP, ＜Blocker＞ ＜Guard＞ — and [Kotenken] — Black, 9000 DP, ＜Blocker＞ per DCGO EX12_034, confirmed against the EX12-034 scan) SHALL be registered in the token registry with their printed stats and keywords carried through the same keyword parse as printed cards.

#### Scenario: Paishu token carries its keywords
- **WHEN** EX12-057 plays a [Paishu] token
- **THEN** the token permanent has Blocker and Guard active, 6000 DP, and Yellow color

### Requirement: Shambala interaction capstone
After per-card suites are green, the slice SHALL receive multi-card interaction tests in `code/digimon-engine/tests/archetypes/` plus the four static archetype tests (deck-legality, coverage gate, smoke games, combo-presence), covering at least the SW/TB sub-engine cross-references and one Tentei Hachibushu line to Lv.7 Susanoomon.

#### Scenario: Interaction suite green
- **WHEN** the capstone lands
- **THEN** the Shambala interaction suite passes and a verdict is recorded in `qa/qa-reports/archetype_interactions.json`
