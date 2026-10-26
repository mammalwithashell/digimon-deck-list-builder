## ADDED Requirements

### Requirement: Recent global EX13 selection preserves exact provenance
The preparer SHALL select exact sourced EX13 lists using explicit recency,
competitive result and diversity criteria. English language or region SHALL NOT
be required. Every selected list MUST pass current admission and legality checks.

#### Scenario: Competitive recent list
- **WHEN** a distinct admitted list has a recent EX13 event result
- **THEN** its original counts, source URL, date, event/finish metadata, fingerprint
  and selection rationale are recorded in the versioned pool

#### Scenario: Unsupported or duplicate list
- **WHEN** a candidate has unsupported cards, is illegal, or repeats card counts
- **THEN** it is rejected with reasons rather than repaired or counted as distinct

#### Scenario: Native sampling uses explicit weighting slots
- **WHEN** some archetypes have one distinct training list and others have two
- **THEN** the native flat-sampling export can repeat explicitly named weighting
  slots to preserve uniform archetype probability, while distinct-list and
  held-out evidence counts remain deduplicated

### Requirement: Evaluation exposes real variant generalization
The preparer SHALL target two distinct training and two distinct held-out lists
for each of the existing 13 archetypes. Held-out lists MUST NOT share a card-count
fingerprint with any training list. Source shortfalls MUST be explicit.

#### Scenario: Insufficient qualified pet lists
- **WHEN** an archetype lacks sufficient sourced admitted variants
- **THEN** a retained pet fallback is labeled as such and no duplicate or training
  list is claimed as unseen-list evaluation

#### Scenario: Preserved original benchmark
- **WHEN** new assets are prepared
- **THEN** old frozen snapshots remain intact and original pet lists are retained
  as a separate regression panel

### Requirement: Preparation is isolated from live training
Pool preparation SHALL emit new versioned snapshots and validation receipts
without changing the active diagnostic run, checkpoint, replay or search budget.

#### Scenario: Current cloud run continues
- **WHEN** the new pool is prepared locally
- **THEN** the live run continues with its original image and input hashes

### Requirement: Multiple list variants receive matchup coverage
The balanced evaluator SHALL traverse every candidate/opponent list combination
within an archetype matchup when given enough seed pairs, while retaining exact
archetype quotas and paired opening order. Revised schedules MUST identify their
cohort scheme separately from previous schedules.

#### Scenario: Two lists in each of thirteen archetypes
- **WHEN** four seed pairs per archetype matchup are scheduled
- **THEN** all four list combinations appear, each with both opening orders,
  rather than two combinations repeating in lockstep
