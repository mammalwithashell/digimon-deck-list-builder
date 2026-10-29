## ADDED Requirements

### Requirement: Timing evidence names its scope
The audit SHALL inventory timing declarations for the frozen pool and new pool
additions, recording event/carrier/controller scope and evidence status.

#### Scenario: Reviewed but untested clause
- **WHEN** a clause has only static/source review
- **THEN** the audit labels that evidence distinctly from a native passing
  regression or independent oracle confirmation

### Requirement: New defects require behavioral reproduction
Each additional timing defect repaired by this change MUST first have a failing
native regression and then a passing repair plus relevant negative controls.

#### Scenario: Wrong event kind
- **WHEN** an effect printed to trigger on a Digimon play is tested with a Tamer
  play and a legal own-Digimon play
- **THEN** the negative case does not trigger and the positive case remains
  available with normal choices

#### Scenario: Evidence gaps remain
- **WHEN** a timing class or new card has not received behavioral verification
- **THEN** the report preserves that gap and does not infer correctness from
  card admission, existing test-file presence, or training win rate
