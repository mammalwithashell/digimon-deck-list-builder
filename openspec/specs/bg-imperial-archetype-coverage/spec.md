# bg-imperial-archetype-coverage Specification

## Purpose

Define the end-state coverage guarantees for the BG Imperial archetype in the
Rust DSL engine: every `#[ignore]`'d behavioral test across the 24 BG Imperial
cards is verifiably classified, the DSL substrate gaps that genuinely block
those cards are closed by new predicate and verb leaves, every unblocked card
has a faithful re-authored YAML implementation with behavioral coverage, and the
verdict ledger and gap trackers reflect the verified state.
## Requirements
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

### Requirement: BG Imperial readiness reconciliation

The system SHALL maintain a reconciled BG Imperial readiness record after gap closeout, covering every card in the `BG Imperial` deck-library pool and keeping the DSL ledger, QA trackers, YAML comments, and behavioral-test annotations consistent with current source and verification results.

#### Scenario: Deck-library pool is fully accounted for

- **WHEN** BG Imperial readiness is reconciled
- **THEN** every unique card ID in `data/deck_library.json` for the `BG Imperial` archetype is represented in the reconciliation notes
- **AND** any mismatch between that pool and `qa/qa-reports/validated_cards_dsl.json` is explicitly resolved or documented

#### Scenario: Stale live-blocker language is removed

- **WHEN** a BG Imperial YAML file, test file, or QA tracker references a gap that current source and tests prove resolved
- **THEN** the reference is rewritten or moved to resolved-history language
- **AND** it no longer presents the card as `PARTIAL`, `BLOCKED`, approximated, or raw-rust-dependent

#### Scenario: Ledger status follows verification

- **WHEN** a BG Imperial card's YAML implements all printed clauses in scope and its focused behavioral tests pass
- **THEN** `qa/qa-reports/validated_cards_dsl.json` records that card as `IMPLEMENTED`
- **AND** the card's notes no longer cite the resolved gap as a live blocker

### Requirement: BG Imperial raw-rust audit

The system SHALL verify that the complete BG Imperial deck-library pool has no live YAML `raw_rust` clauses, steps, or formulas before claiming the archetype has no raw-rust escapes.

#### Scenario: No live raw-rust in archetype YAML

- **WHEN** the BG Imperial raw-rust audit is run across all YAML files for the deck-library pool
- **THEN** no non-comment `raw_rust` usage is found
- **AND** the audit result is recorded in the BG Imperial readiness notes

#### Scenario: Adjacent raw-rust remains out of scope

- **WHEN** a raw-rust function belongs to a card outside the BG Imperial deck-library pool
- **THEN** it is not counted as a BG Imperial raw-rust escape
- **AND** any useful follow-up is documented separately from BG Imperial readiness

### Requirement: BG Imperial focused verification

The system SHALL run focused Rust behavioral tests for BG Imperial cards whose readiness status or stale-blocker language changes during reconciliation.

#### Scenario: Disputed partial card is verified

- **WHEN** a card moves from `PARTIAL` to `IMPLEMENTED` in the BG Imperial ledger
- **THEN** its focused `cargo test --manifest-path code/digimon-engine/Cargo.toml --test cards_behavioral <card_test_filter> -- --nocapture` command passes
- **AND** the reconciliation notes record the command and result

#### Scenario: Existing implemented extra pool card is verified

- **WHEN** a card appears in the BG Imperial deck-library pool but was previously tracked under another archetype in `validated_cards_dsl.json`
- **THEN** its focused behavioral tests pass before it is cited as covered for BG Imperial
- **AND** the ledger mismatch is documented without duplicating or corrupting the card's canonical ledger entry

