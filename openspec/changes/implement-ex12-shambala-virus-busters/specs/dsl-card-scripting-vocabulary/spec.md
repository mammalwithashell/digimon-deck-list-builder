# Delta: dsl-card-scripting-vocabulary

## ADDED Requirements

### Requirement: EX12 keywords in the DSL keyword surface
The DSL validator's keyword allowlist (`KNOWN_KEYWORD_KEYS`) SHALL accept `Guard` and `Engage`, and `grant_keyword` clauses naming them SHALL lower consistently with the established native-printed-keyword pattern (as with `Training`/`Ascension`: the runtime behavior rides the printed-keyword parse and keyword machinery; a `grant_keyword` clause is the visible compiled-DSL declaration and any aura-granted instance MUST activate the same behavior as a printed instance).

#### Scenario: dsl-lint accepts the new keywords
- **WHEN** a YAML card declares `grant_keyword: Guard` or `grant_keyword: Engage`
- **THEN** `dsl-lint` reports no unknown-keyword error and the card compiles

#### Scenario: Aura-granted Guard behaves like printed Guard
- **WHEN** an aura grants ＜Guard＞ to a Digimon for a duration (e.g. EX12-072's [Security] effect granting all [ME] Digimon Guard)
- **THEN** the granted Digimon offers the same protect-others leave replacement as a printed Guard carrier while the grant is active

### Requirement: Assessment-surfaced vocabulary additions are spec'd before implementation
Any DSL verb, predicate, or timing the EX12 gap assessment surfaces beyond the two keywords SHALL be added to this delta (with its own requirement and scenarios) before the closure round that implements it, so the vocabulary contract is reviewable ahead of code.

#### Scenario: New vocabulary lands with spec coverage
- **WHEN** a closure round adds a new DSL step/predicate for an EX12 card
- **THEN** this delta contains a requirement + scenario for it, and the vocab-doc drift gate (`docs/RUST_DSL_AGENT_GUIDE.md` regen) passes
