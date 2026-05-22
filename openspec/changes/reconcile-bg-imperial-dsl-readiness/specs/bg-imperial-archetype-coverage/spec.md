## ADDED Requirements

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
