## ADDED Requirements

### Requirement: Separate legal information-limited teacher
The system SHALL expose a versioned development heuristic independently from
unchanged greedy defaults. It MUST select only legal actions for the current
deciding seat and use only information available to that seat.

#### Scenario: Hidden-zone permutations
- **WHEN** hidden opponent hands, security identities, or unrevealed deck order
  change while the deciding player's visible state and legal mask stay equal
- **THEN** the development teacher selects the same action

#### Scenario: Opponent-turn selection
- **WHEN** an effect requests a decision from the non-turn player
- **THEN** that player's assigned teacher and visible information determine the
  legal selected action

### Requirement: Cost-aware evolution and hand development
The teacher SHALL rank useful evolution and hand development using current
action semantics, memory costs, material losses, and net card accounting. It
MUST retain immediate tactical priorities and distinguish unknown benefits.

#### Scenario: Evolution versus excessive hard play
- **WHEN** a supported beneficial low-cost evolution and a costly setup play
  are legal with no overriding tactical requirement
- **THEN** the versioned tactical fixture selects the beneficial evolution,
  including a configured case that passes a small amount of memory

#### Scenario: Replacement is not net card advantage
- **WHEN** an action consumes one hand card and draws one card
- **THEN** its hand feature records replacement and zero net hand gain

#### Scenario: Inactive card text
- **WHEN** a legal play's card mentions drawing only in an unavailable timing
- **THEN** that text alone does not earn an immediate draw or net-hand bonus

#### Scenario: DNA route tradeoff
- **WHEN** a legal DNA route consumes two friendly bodies
- **THEN** ranking accounts for both consumed materials and the resulting board
  and telemetry identifies the route as DNA

### Requirement: Complete pending-choice and paging behavior
The teacher SHALL resolve useful optional effects and mandatory choices through
normal actions, including payment, materials, retrieval, and owned card pages.
It MUST bound navigation and reject no-progress loops without illegal shortcuts.

#### Scenario: Useful optional retrieval
- **WHEN** an affordable optional retrieval has an eligible revealed evolution
  piece and no overriding tactical conflict
- **THEN** the teacher activates and selects it through separate legal decisions

#### Scenario: Paged duplicate card identities
- **WHEN** the best revealed physical candidate is on another page and shares
  a printed ID with another candidate
- **THEN** bounded browsing and current-page action rebinding select the intended
  physical candidate without committing an inspection row or stale action

#### Scenario: No-progress cycle
- **WHEN** legal choices repeatedly revisit the same visible semantic state
- **THEN** the teacher selects a progressing alternative or legal decline, or
  reports a diagnostic failure when neither is available

### Requirement: Strategy state and observability
The teacher SHALL reset state at game boundaries and emit its version, selected
action category, feature contributions, and unknown-metadata coverage. Its
scores MUST NOT modify engine rules, rewards, observations, or legal masks.

#### Scenario: Reproducible clean reset
- **WHEN** a seeded game is repeated after an unrelated game
- **THEN** teacher state resets and the same visible decisions produce the same
  action trace without residual paging/progress history
