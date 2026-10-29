## ADDED Requirements

### Requirement: Training declares its information regime
Self-play and evaluation SHALL explicitly select closed- or open-decklist mode. Open mode SHALL retain exact-list behavior; closed mode SHALL use viewer beliefs on both seats. Untagged historical artifacts SHALL remain identifiable as legacy exact-prior data.

#### Scenario: Both seats generate fair targets
- **WHEN** closed-mode self-play reaches a decision for either seat
- **THEN** that seat's search target uses only its entitled knowledge and the shared frozen prior, regardless of driver access to true decks

### Requirement: Artifacts preserve regime provenance
Run, shard, model and resume metadata SHALL identify information regime, prior and legality snapshot hashes, candidate split, knowledge schema, search budgets and seeds. Incompatible resumes or mixed-regime buffers SHALL be rejected unless mixing is explicitly configured and recorded.

#### Scenario: Resume changes the prior
- **WHEN** a run resumes with a different candidate snapshot or information regime
- **THEN** it rejects the mismatch instead of silently continuing under old provenance

### Requirement: Fair targets preserve observation and action contracts
Training SHALL retain active observation/action shapes and legal masked decisions. Belief diagnostics SHALL remain sidecar metadata. Any required tensor expansion SHALL be separately planned with synchronized compatibility changes.

#### Scenario: A policy cannot observe all teacher history
- **WHEN** a counting probe exposes missing observation history or model memory
- **THEN** the limitation is reported and no policy-only readiness claim is inferred from search-assisted success
