## ADDED Requirements

### Requirement: Royal Knights stubs are reconciled against current substrate

Each Royal Knights card stub, raw-rust escape, and ignored behavioral test SHALL be reviewed against current Rust engine and DSL capabilities before implementation work begins.

#### Scenario: Stale gap marker is found

- **WHEN** a Royal Knights YAML comment or ignored test cites a gap that current code already supports
- **THEN** the card SHALL be reclassified as card-authoring backlog
- **AND** the stale gap marker SHALL be removed or rewritten during the card migration

#### Scenario: Genuine blocker remains

- **WHEN** current code cannot express a printed Royal Knights clause without approximation
- **THEN** the card SHALL remain partial or blocked
- **AND** the blocker SHALL be recorded in the relevant gap tracker with capability-centric language

### Requirement: BT13-112 Omnimon is faithfully authored

BT13-112 Omnimon SHALL implement its On Play and When Digivolving modal effect faithfully, including the delete branch and the breeding-source play branch.

#### Scenario: Player chooses delete branch

- **WHEN** BT13-112's On Play or When Digivolving effect resolves and an opponent Digimon is legal
- **THEN** the player SHALL be able to choose the delete branch
- **AND** the selected opponent Digimon SHALL be deleted

#### Scenario: Player chooses breeding-source play branch

- **WHEN** BT13-112's effect resolves and the controller has Royal Knight source cards under a breeding-area Digimon
- **THEN** the player SHALL be able to choose the source-play branch
- **AND** selected Royal Knight source cards with different names SHALL be played without paying costs
- **AND** On Play effects of Digimon played by this effect SHALL NOT activate

#### Scenario: Rush is granted only after source play succeeds

- **WHEN** BT13-112 plays at least one Digimon from breeding sources
- **THEN** the controller's Digimon SHALL gain Rush for the turn
- **AND** the controller's breeding-area Digimon SHALL be trashed as printed

### Requirement: King Drasil option and source flows are faithfully authored

Royal Knights cards that place cards under or play cards from a breeding-area King Drasil SHALL use native breeding selection and material-play primitives.

#### Scenario: BT13-110 main effect can decline hand-to-source placement

- **WHEN** BT13-110 resolves its Main effect and a breeding-area King Drasil plus eligible hand Digimon exist
- **THEN** the player SHALL be able to decline the hand-to-source placement
- **AND** the option SHALL still be placed in the battle area as printed

#### Scenario: BT13-110 delay plays a Royal Knight source

- **WHEN** BT13-110's Delay effect resolves with a Royal Knight source under a breeding-area Digimon
- **THEN** the player SHALL select a Royal Knight source to play without paying the cost
- **AND** that Digimon's On Play effects SHALL NOT activate
- **AND** that Digimon SHALL gain Rush for the turn

#### Scenario: BT20-083 on-deletion can decline source placement

- **WHEN** BT20-083 is deleted while the controller has a breeding-area King Drasil
- **THEN** the player SHALL be able to decline placing BT20-083 as a bottom digivolution card

### Requirement: Jesmon Royal Knights pressure cards are faithfully authored

Royal Knights Jesmon cards whose substrate is closed SHALL have production YAML and behavioral tests for token play, other-Digimon-play observers, deletion, and immediate attack choices.

#### Scenario: BT20-017 plays Atho, Rene, and Por token

- **WHEN** BT20-017 resolves its On Play or When Digivolving effect and there is battle-area space
- **THEN** the player SHALL be able to play one Atho, Rene, and Por token
- **AND** the token SHALL have its printed stats and keywords

#### Scenario: BT20-017 reacts to another own Digimon played

- **WHEN** another Digimon controlled by the BT20-017 controller is played during their turn
- **THEN** BT20-017's once-per-turn observer SHALL be able to delete one opponent Digimon with 8000 DP or less
- **AND** one of the controller's Digimon SHALL be able to attack through a player-visible choice

### Requirement: BT17-018 Gallantmon Crimson Mode uses native budgeted selection

BT17-018 SHALL use native DSL and engine budgeted selection for its On Play and When Digivolving delete effect, not a single-target raw-rust approximation.

#### Scenario: Multiple targets within DP budget are deleted

- **WHEN** BT17-018 resolves its budgeted delete effect against opponent Digimon with total selectable DP of 15000 or less
- **THEN** the player SHALL be able to select multiple opponent Digimon whose total DP does not exceed 15000
- **AND** all selected Digimon SHALL be deleted

#### Scenario: Over-budget combination is impossible

- **WHEN** selecting an additional opponent Digimon would make the selected total DP exceed 15000
- **THEN** that additional Digimon SHALL NOT be legal for the current selection

### Requirement: Royal Knights behavioral coverage is active

Royal Knights card migrations SHALL include active behavioral tests that exercise the printed choices, decline paths, and negative cases.

#### Scenario: No test remains ignored for a closed gap

- **WHEN** a Royal Knights ignored test cites a gap that this change closes or verifies as already closed
- **THEN** the test SHALL be re-enabled or replaced by active equivalent coverage

#### Scenario: Remaining ignores cite current blockers

- **WHEN** a Royal Knights behavioral test remains ignored after this change
- **THEN** the ignore reason SHALL cite a code-verified open primitive
- **AND** the same primitive SHALL appear in the current gap trackers
