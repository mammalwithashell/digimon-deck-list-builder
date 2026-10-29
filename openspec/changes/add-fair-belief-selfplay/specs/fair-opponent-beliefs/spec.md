## ADDED Requirements

### Requirement: Beliefs depend only on entitled information
Closed-decklist beliefs and search outputs SHALL depend only on the viewer's own information, public facts, entitled remembered knowledge, frozen prior configuration, and seeds. The provider MUST NOT read true opponent composition or hidden assignments.

#### Scenario: Different hidden compositions are indistinguishable
- **WHEN** two legal source games have identical viewer knowledge and root action masks but different concealed opponent compositions
- **THEN** closed-mode beliefs, sampled worlds, and search outputs are identical under fixed seeds and evaluator, including after resume

### Requirement: Legal hypotheses account for public copies
Hypotheses SHALL obey the pinned format's deck construction, restrictions, combination and card-specific copy rules and current public accounting across all zones. Historical sightings MUST NOT double-count recycled instances.

#### Scenario: All copies are accounted for
- **WHEN** a hypothesis contains four copies of an ID and two are in trash and two are public elsewhere
- **THEN** it assigns zero copies of that ID to hidden zones until a legal zone move changes accounting

#### Scenario: Restriction and recovery
- **WHEN** a format limits a card to one copy and its public instance is returned from trash to a hidden hand
- **THEN** every hypothesis contains at most that one copy and viewer-entitled identity knowledge follows the move without inventing another copy

### Requirement: Candidate uncertainty includes unfamiliar legal lists
The provider SHALL maintain a normalized candidate mixture and configurable positive unseen-list mass, condition on evidence, and use a constrained fallback from the frozen legal population. Exhaustion SHALL be explicit and SHALL NOT consult true hidden information or relax legality.

#### Scenario: Evidence eliminates every candidate
- **WHEN** public facts contradict every known candidate list
- **THEN** the provider samples consistent legal fallback completions or reports bounded exhaustion with diagnostics

#### Scenario: Compatible candidates remain
- **WHEN** one candidate remains but the prior config reserves unseen-list mass
- **THEN** the provider retains that mass rather than asserting that the opponent must use the remaining list

### Requirement: Entitled memory has lifecycle semantics
Viewer knowledge SHALL retain legitimate reveals and peeks, distinguish private and public facts, invalidate knowledge when warranted by hidden randomization, and survive snapshot/resume without hidden-truth reconstruction.

#### Scenario: Peek followed by shuffle
- **WHEN** a viewer knows a deck's top card and that deck is shuffled
- **THEN** the prior no longer pins that top position and the opponent does not acquire the private peek

### Requirement: Sampled worlds represent the hypothesis
Closed-mode worlds SHALL be constructed from sampled compositions and entitled facts, with coherent unique instances, counts, metadata, action masks, and continuations. Invariants SHALL compare to the hypothesis, not the actual hidden deck. Unsupported continuation states SHALL fail explicitly.

#### Scenario: Hypothetical card differs from the true deck
- **WHEN** a consistent sampled list includes a legal card absent from the actual opponent list
- **THEN** the materialized world can contain and resolve that card without borrowing hidden instances or enforcing the true list's composition
