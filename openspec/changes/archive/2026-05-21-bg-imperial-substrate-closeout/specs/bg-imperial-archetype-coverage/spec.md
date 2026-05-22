## ADDED Requirements

### Requirement: BG Imperial gap re-audit

The system SHALL produce a verified, per-clause classification of every `#[ignore]`'d behavioral test across the 24 BG Imperial cards before any substrate or card-authoring code is written. Each blocked clause SHALL be classified as `stale-ignore` (the required primitive already exists and the card needs re-authoring), `genuine-gap` (cites a primitive verified missing against current source), or `out-of-scope` (BT3-103 Clause 0). The BG Imperial gap trackers SHALL be updated to reflect the verified state.

#### Scenario: Every ignored test classified

- **WHEN** Phase 0 completes
- **THEN** each `#[ignore]`'d test in the 24 BG Imperial card test files has a recorded classification of `stale-ignore`, `genuine-gap`, or `out-of-scope`
- **AND** every `genuine-gap` classification names a primitive whose absence was verified by source inspection of `code/digimon-dsl/src/` and `code/digimon-engine/src/`

#### Scenario: Trackers no longer cite resolved primitives as open

- **WHEN** Phase 0 completes
- **THEN** `qa/archetype-qa/dsl/bg-imperial-cross-archetype-gaps-2026-05-03.md` and the BG Imperial entries of `qa/dsl-vocab-gaps.md` no longer list as open any primitive that exists in current source
- **AND** primitives confirmed resolved are moved to `qa/resolved-gaps.md` with a passing `cargo test` command

### Requirement: Source-stack-relative size predicate

The DSL SHALL provide a `stack_size_lte_source` boolean predicate that evaluates true when a candidate permanent's digivolution-card count is less than or equal to the effect source permanent's digivolution-card count. It SHALL be usable inside selection filters.

#### Scenario: Candidate with fewer or equal sources is selectable

- **WHEN** an effect from a source permanent with N digivolution cards selects an opponent permanent with a filter of `stack_size_lte_source: true`
- **THEN** opponent permanents with N or fewer digivolution cards are selectable
- **AND** opponent permanents with more than N digivolution cards are excluded

### Requirement: Carrier-keyword predicate for inherited clauses

The DSL SHALL provide a `carrier_has_keyword` predicate that, for an inherited (digivolution-source) effect clause, evaluates whether the top card of the permanent carrying the source has a given keyword, counting both printed and modifier-granted keywords.

#### Scenario: Inherited clause gated on carrier keyword

- **WHEN** an inherited clause with `condition: { carrier_has_keyword: <K> }` is evaluated
- **THEN** the predicate resolves against the carrier permanent's keyword set, not the digivolution-source slot
- **AND** the clause does not fire when the carrier lacks keyword `<K>`

### Requirement: Self-aura target restricted to the source carrier

The DSL SHALL allow a `kind: aura` clause to restrict its target set to the permanent carrying the effect source, so an inherited aura grants only to its own carrier rather than to all of the controller's Digimon.

#### Scenario: Inherited aura grants only to its carrier

- **WHEN** an inherited `kind: aura` clause restricts its target to the source carrier
- **THEN** only the carrier permanent receives the granted keyword or modifier
- **AND** other Digimon controlled by the same player are unaffected

### Requirement: Self digivolution-stack trait predicate

The DSL SHALL provide a `self_digivolution_contains_trait` predicate that evaluates whether the source permanent's digivolution stack (including its top card) contains a card with a given trait.

#### Scenario: Trait present in own stack

- **WHEN** `condition: { self_digivolution_contains_trait: <T> }` is evaluated and the source permanent's stack contains a card with trait `<T>`
- **THEN** the predicate resolves true
- **AND** it resolves false when no card in the stack carries trait `<T>`

### Requirement: Effect-suspended result predicate for opponent and any scope

The DSL SHALL provide a predicate that branches on whether a prior step of the current effect suspended an opponent's Digimon (and/or any Digimon), complementing the existing own-scoped `effect_suspended_any_own_digimon`.

#### Scenario: Conditional reward when the suspend did not occur

- **WHEN** an optional suspend step targeting an opponent's Digimon is declined or has no legal target
- **THEN** a subsequent clause gated on "this effect did not suspend an opponent's Digimon" resolves true and its body runs

### Requirement: Select opponent digivolution sources

The DSL SHALL provide a `select_opponent_sources` step that surfaces a player-visible selection of digivolution-source cards across the opponent's battle-area stacks, supporting exact-N and up-to-N counts, PASS exposed only after the minimum count, optional `filter:`, and stable source references for downstream steps.

#### Scenario: Pick opponent sources for a downstream effect

- **WHEN** a `select_opponent_sources` step with count N resolves
- **THEN** the selecting player is shown N source picks across the opponent's battle-area stacks
- **AND** the chosen source references are usable by the following step (e.g. trash)
- **AND** the choice is surfaced through `PendingSelection` / action masks

### Requirement: Selected-trash-card to deck-top movement

The DSL SHALL provide a movement verb that places a selected trash card on top of its owner's deck, distinct from the existing trash-to-deck-bottom verbs.

#### Scenario: Chosen trash card returns to deck top

- **WHEN** a step moves a player-selected trash card to the deck top
- **THEN** that card becomes the topmost card of its owner's deck
- **AND** other trash cards are unaffected

### Requirement: Returned-card result-set predicate

The DSL SHALL provide an `any_returned_card` predicate that evaluates whether the cards moved by an immediately preceding zone-movement step include at least one card matching a given filter, and SHALL support a player choice of whose trash a bulk return operates on.

#### Scenario: Memory rider conditioned on a returned card

- **WHEN** a bulk trash-return step has completed and at least one returned card matches `{ color_is: white, level_eq: 7 }`
- **THEN** an `if` gated on `any_returned_card` with that filter resolves true and its body runs

#### Scenario: Player chooses whose trash is returned

- **WHEN** an effect text returns all cards from "your or your opponent's trash"
- **THEN** the controlling player is offered a choice of which player's trash is affected
- **AND** the chosen player's trash is the one returned

### Requirement: BG Imperial card faithfulness coverage

Each BG Imperial card whose blocking primitive is closed by this change SHALL have its YAML re-authored to implement the previously omitted clauses, with `DebugRunner` behavioral tests covering the positive and negative cases of each clause, and SHALL have its `validated_cards_dsl.json` verdict updated. No clause SHALL be implemented with a stub, auto-selection, or approximation.

#### Scenario: Blocked card reaches IMPLEMENTED

- **WHEN** every clause of a BG Imperial card has a faithful implementation and passing behavioral tests
- **THEN** its `validated_cards_dsl.json` verdict is updated to IMPLEMENTED
- **AND** no behavioral test for that card remains `#[ignore]`'d except clauses explicitly scoped out (BT3-103 Clause 0)

#### Scenario: Out-of-scope clause stays explicitly omitted

- **WHEN** a card has a clause depending on `G-COST-REDUCE-ALLY-DIGIVOLVE` (BT3-103 Clause 0)
- **THEN** that clause remains omitted from the card YAML
- **AND** its tests remain `#[ignore]`'d with a reason citing the out-of-scope engine gap
