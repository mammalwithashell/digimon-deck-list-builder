# Spec: rules-conformance-suites

## ADDED Requirements

### Requirement: Keyword semantics matrix
A data-driven test suite SHALL encode the verified keyword-semantics table (derived from `general_rule.pdf` §16 via `docs/digimon-rules/keyword-semantics.md`) — each keyword's kind (persistent / mandatory / optional / optional-cost-then-mandatory), trigger timing, and once-per semantics — instantiating every keyword on synthetic cards. Every keyword the engine models MUST have a matrix row; adding a `Keyword` variant without a row MUST fail the suite.

#### Scenario: Keyword machinery change trips the matrix in seconds
- **WHEN** a change to shared keyword machinery alters a keyword's optionality or timing
- **THEN** the matrix suite (tier 1) fails on the affected row within its fast budget, before any full-suite run

#### Scenario: New keywords arrive with rows
- **WHEN** ＜Guard＞ and ＜Engage＞ land for EX12
- **THEN** the matrix gains their rows (Guard: optional-cost-then-mandatory protect-others replacement; Engage: optional end-of-turn attack) in the same change

### Requirement: FAQ conformance suite
The repository SHALL provide a curated, generated conformance suite derived from official card Q&A/rulings: each approved Q&A entry becomes a DebugRunner scenario citing the ruling text verbatim. Curation is manual-approve — entries become tests only when marked scenario-izable — and the suite runs in tier 1.

#### Scenario: Ruling regression is caught fast
- **WHEN** an engine change contradicts a scenario-ized official ruling
- **THEN** the FAQ suite fails with the ruling text in the assertion message

### Requirement: Judge quiz is an always-run canary
The existing judge-quiz suite SHALL run in tier 1 on every engine-affecting change (locally and in CI), not only on demand.

#### Scenario: Quiz gate on engine changes
- **WHEN** any engine or DSL lowering change is verified at tier 1
- **THEN** the judge-quiz binary runs and gates the result
