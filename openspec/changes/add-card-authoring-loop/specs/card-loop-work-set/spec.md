## ADDED Requirements

### Requirement: Cards, sets, decklists and archetypes resolve to one work set
The card loop SHALL accept `--cards` (ids or `@file`), `--set <PREFIX>`, `--decklists <files>` and `--archetype <name>`, alone or combined, and SHALL resolve them to a single work set consisting of a card pool, a core subset and a ranking.

#### Scenario: Explicit card list
- **WHEN** the loop is planned with `--cards BT7-056,EX7-008`
- **THEN** the pool and the core are exactly those two cards, ranked in the given order

#### Scenario: Decklists define core by frequency
- **WHEN** the loop is planned with ten decklists and `core_fraction` 0.7
- **THEN** the pool is the union of their cards and the core is the cards appearing in at least seven of the ten lists

#### Scenario: Decklists in any supported format
- **WHEN** a decklist file is in any format the engine's deck parser accepts
- **THEN** it is parsed by that parser rather than a separate loop-specific parser

#### Scenario: Archetype uses the deck library
- **WHEN** the loop is planned with `--archetype "Rocks"`
- **THEN** the decklists are taken from the deck library entries resolved through the archetype alias map, and an unknown name fails with near-miss suggestions

#### Scenario: Set with and without decklists
- **WHEN** the loop is planned with `--set BT27` alone
- **THEN** the pool is every BT27 card id and the core is the whole pool
- **WHEN** the loop is planned with `--set BT27 --decklists lists/*.txt`
- **THEN** the pool is the BT27 set and the core and ranking come from the lists

### Requirement: Ranking prefers the cards that unblock the most decklists
When decklists are part of the input, the work set MUST rank cards by greedy decklist completion, repeatedly choosing the card whose adjudication completes the most otherwise-ready decklists, tie-broken by the number of decklists containing the card.

#### Scenario: A staple outranks a one-of
- **WHEN** one card appears in 270 input decklists and another in 3, and both are the last missing card of their lists
- **THEN** the 270-list card ranks first

### Requirement: A new-set preflight gates every run
Before planning or running, the loop SHALL check that the official mirror and `cards.json` contain every pool card, that the keyword gate passes, which pool cards have a DCGO script, that the oracle player's action-space hash matches the engine, that both worker CLIs resolve, and that the oracle node reports GO; a failed check MUST name its remedy, and the loop MUST NOT spend model budget on a NO-GO.

#### Scenario: Stale player
- **WHEN** the player manifest's action-space hash differs from the engine's
- **THEN** preflight fails naming the rebuild remedy and no worker is invoked

#### Scenario: Card without a DCGO script
- **WHEN** a pool card has no DCGO script for its set and colour
- **THEN** its clauses and interactions are planned as `unavailable` with that reason, and the rest of the run proceeds

#### Scenario: DCGO later adds the card
- **WHEN** a run is resumed after the DCGO checkout gains the missing script
- **THEN** that card's items re-enter the plan as outstanding

### Requirement: Runs are resumable from committed ledgers
The loop SHALL keep each run in `runs/card-loop/<run-id>/` with a frozen `plan.json` and an append-only `events.jsonl`, and on `resume` SHALL rebuild outstanding work from the committed verdict and attempt ledgers first and the run's events second, so completed work is never repeated.

#### Scenario: Crash mid-run
- **WHEN** a run is interrupted after some items reached terminal states and is resumed
- **THEN** only items without a terminal state are scheduled

#### Scenario: Work adjudicated elsewhere
- **WHEN** an item in the plan was adjudicated by another run or an attended session since planning
- **THEN** resume skips it
