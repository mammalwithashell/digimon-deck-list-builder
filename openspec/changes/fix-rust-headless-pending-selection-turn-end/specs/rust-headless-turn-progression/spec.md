## ADDED Requirements

### Requirement: Pending selections remain the active headless decision
The Rust headless engine SHALL preserve an unresolved `pending_selection` as the active decision even when the action that created it caused memory to cross to the opponent's side.

#### Scenario: Memory crosses while On Play creates a mandatory selection
- **WHEN** a Rust headless Main phase action plays a card, pays enough memory to cross turn ownership, and installs a mandatory pending selection
- **THEN** the engine SHALL keep the pending selection resolvable by the selecting player instead of exposing a pass-only `EndTurn` state

#### Scenario: Action mask exposes pending selection choices
- **WHEN** a pending selection is active after memory has crossed
- **THEN** the action mask SHALL expose the selection's `valid_action_ids` and optional pass only when the selection is optional

### Requirement: Turn-end rotation waits for pending choices
The Rust headless engine SHALL defer turn-end rotation until required pending selections and their immediate follow-up effect chain have resolved.

#### Scenario: Selection resolves with memory still crossed
- **WHEN** a mandatory pending selection resolves and memory remains on the opponent's side
- **THEN** the engine SHALL continue turn-end processing and rotate to the next player through the normal turn-start sequence

#### Scenario: Selection resolves and memory swings back
- **WHEN** a pending selection or follow-up effect resolves and restores memory to the active player's side before turn rotation
- **THEN** the engine SHALL keep the current player in Main phase according to the existing memory swing-back rule

### Requirement: Greedy baseline does not loop on engine-induced EndTurn pass
The Rust-backed greedy baseline SHALL be able to resolve pending selections produced by its own legal actions and continue the game without looping on no-op pass actions.

#### Scenario: Greedy plays a setup Digimon with a mandatory On Play choice
- **WHEN** the greedy baseline plays a setup Digimon that installs a mandatory On Play selection after paying memory
- **THEN** greedy SHALL choose a legal selection action from the exposed mask and the game SHALL advance beyond the previously stuck `EndTurn` state

#### Scenario: Generalist smoke evaluation avoids pending-selection timeout draw
- **WHEN** a short Rust-backed generalist pilot evaluation reaches a greedy opponent turn that creates a pending selection
- **THEN** the episode SHALL not draw solely because the engine repeated pass in `EndTurn` while `winner_id` remained unset and `game_over` remained false
