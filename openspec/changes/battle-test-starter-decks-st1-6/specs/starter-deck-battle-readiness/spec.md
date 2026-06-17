## ADDED Requirements

### Requirement: Starter-deck card behavior is re-verified against authoritative sources

Every unique card in ST-1 … ST-6 (`ST1-01`…`ST6-16`, 96 cards) SHALL have its Rust DSL behavior re-verified faithful to its printed text, using the canonical source priority (card image → DCGO C# → `general_rule.pdf` → fandom; `cards.json` lowest trust). The prior `audit-starter-decks-st1-6` (2026-05-29) verdicts SHALL be treated as untrusted and re-derived per card. Each card's re-derived verdict SHALL be recorded in `qa/qa-reports/validated_cards_dsl.json` with a card-specific note (not a shared templated string) under a new report id.

#### Scenario: Every starter card has a re-derived verdict
- **WHEN** the battle-test audit completes
- **THEN** each of the 96 ST-1…ST-6 card IDs has a `validated_cards_dsl.json` entry whose report id is the new battle-test report
- **AND** its `audit_note` is specific to that card's behavior, not the shared string "Faithful to printed text + DCGO."

#### Scenario: A faithfulness discrepancy is recorded, not hidden
- **WHEN** a card's DSL behavior diverges from its printed text or DCGO behavior
- **THEN** the divergence is recorded with the authoritative source that establishes the correct behavior
- **AND** the card is not marked OK until the divergence is fixed or, if blocked by a substrate gap, marked BLOCKED with the gap logged

### Requirement: Every non-vanilla starter effect has behavioral test coverage

Every non-vanilla printed effect across ST-1 … ST-6 SHALL be exercised by a Rust DebugRunner behavioral test under `code/digimon-engine/tests/cards_behavioral/st{1..6}/`, covering runtime state changes, pending selections, timing gates, and at least one negative case where applicable. No implemented effect SHALL rely solely on MCP/manual verification for its regression guarantee.

#### Scenario: ST-4 gains the per-card coverage its spec already requires
- **WHEN** the ST-4 behavioral test subset is run
- **THEN** every non-vanilla ST-4 effect has at least one passing behavioral test
- **AND** there are no ignored tests standing in for implemented behavior

#### Scenario: Focused starter card tests pass without regression
- **WHEN** the `st1`–`st6` `cards_behavioral` subsets and the broader `dsl` / `cards_behavioral` suites are run
- **THEN** all starter card tests pass
- **AND** no pre-existing engine tests regress

### Requirement: Each starter deck carries static and interaction tests

Each ST-1 … ST-6 deck SHALL have the four static archetype tests (deck-legality, coverage gate, smoke games, combo-presence) via the `archetype-static-tests` crate, plus multi-card interaction tests in `code/digimon-engine/tests/archetypes/st{N}.rs` covering its principal digivolution lines and combos.

#### Scenario: Static archetype tests pass for every starter deck
- **WHEN** the `archetype-static-tests` runner is executed for each of ST-1 … ST-6
- **THEN** deck-legality, coverage-gate, smoke-game, and combo-presence checks pass for all six decks
- **AND** the verdicts are recorded in `qa/qa-reports/archetype_interactions.json`

#### Scenario: Interaction tests cover each deck's key combos
- **WHEN** the `archetypes` test target is run for `st1`–`st6`
- **THEN** each deck's principal multi-card interaction (its main digivolution line and signature combo) is exercised by a passing test

### Requirement: Starter decks expose every player choice during play

When ST-1 … ST-6 cards resolve, every player decision required by printed text SHALL be surfaced through `pending_selection` and reflected in the legal-action mask. No choice SHALL be auto-resolved, and no forced-but-illegal action SHALL be exposed.

#### Scenario: A staged effect exposes its choice
- **WHEN** an effect with a player choice is staged via `digimon-scenario-mcp` and stepped to its decision point
- **THEN** `get_pending_selection` reports the choice with all printed-legal options
- **AND** the legal-action mask permits exactly those options

#### Scenario: No auto-selection on a mandatory single-target effect
- **WHEN** a mandatory effect with exactly one legal target resolves in a staged scenario
- **THEN** the engine still surfaces the selection rather than silently auto-picking, consistent with the no-approximations policy

### Requirement: Each starter deck plays full games without engine faults

Each ST-1 … ST-6 deck SHALL be playable to game end in both mirror and cross-matchup configurations without panics, soft-locks (a reachable state with no legal action that is not a terminal state), or illegal-mask states. The set of decks played and the number of games SHALL be reported with no silent caps.

#### Scenario: Mirror and cross-matchup games complete
- **WHEN** full games are played for each starter deck in a mirror match and against each of the other five decks
- **THEN** every game reaches a terminal state (win/loss/draw) without a panic or soft-lock
- **AND** the battle-test report records the deck pairings and game counts actually played

#### Scenario: A discovered crash is localized and fixed
- **WHEN** a full game panics or soft-locks
- **THEN** the failing game is localized via recording forensics, the root cause fixed, and a regression test added before the deck is declared battle-ready

### Requirement: Starter lists are training-ready

The six starter lists (`starter_st{1..6}_*` in `data/deck_library.json`) SHALL resolve through the deck-pool / archetype wiring and initialize a Rust-backed `DigimonEnv` that resets and steps without missing-card, deck-size, or implemented-card-filtering errors.

#### Scenario: Deck-pool wiring resolves all six lists
- **WHEN** the deck pool / archetype resolution is exercised for the six starter list names
- **THEN** each list resolves to a legal 54-card deck of implemented cards

#### Scenario: DigimonEnv resets and steps on each starter list
- **WHEN** a Rust-backed `DigimonEnv` is initialized with each starter list
- **THEN** `reset()` returns an observation and action mask of the expected shapes
- **AND** at least one `step()` with a legal action succeeds without error

### Requirement: Battle-test outcome is reported with a go/no-go

The change SHALL produce a human-readable battle-test report under `qa/` recording, per deck: cards audited, bugs found and fixed, tests added, scenarios staged, full-game pairings/counts/results, any cards marked BLOCKED with the substrate gap logged, and an explicit go/no-go for training.

#### Scenario: Report enumerates results and a go/no-go
- **WHEN** the battle-test work completes
- **THEN** the report lists per-deck audit, fix, test, and game-play results
- **AND** it states go or no-go for training, with any BLOCKED cards and their logged gaps called out
