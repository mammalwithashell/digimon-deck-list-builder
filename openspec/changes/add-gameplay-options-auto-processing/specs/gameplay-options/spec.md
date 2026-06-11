# gameplay-options

## ADDED Requirements

### Requirement: Persisted gameplay options panel
The UI SHALL provide a Gameplay Options panel, reachable from the settings page and during a game, exposing individually persisted toggles: auto-resolve trivial choices, auto-order deck placements, auto minimum digivolve cost, auto-hatch, confirm before ending selection, show animations, and rotate suspended cards.

#### Scenario: Options persist across sessions
- **WHEN** the player changes any gameplay toggle and relaunches the app
- **THEN** the changed value is in effect for the next game

#### Scenario: Mid-game changes apply immediately
- **WHEN** the player toggles an option during a game
- **THEN** the new value governs the next applicable prompt or render without restarting the game

### Requirement: Confirm before ending selection
When the confirm-before-ending-selection toggle is enabled, submitting a multi-card selection SHALL require a confirmation step showing the chosen cards before the action is sent.

#### Scenario: Confirmation intercepts submit
- **WHEN** the toggle is on and the player confirms a 2-card selection in the selection panel
- **THEN** a confirmation listing the 2 chosen cards is shown, and the action is sent only after the player confirms

### Requirement: Animation visibility toggle
When the show-animations toggle is disabled, transient gameplay animations (phase banners, digivolve banners, battle effects, security-reveal dwell times) SHALL be skipped or reduced to instant/manual advance, without losing any information the animations convey (revealed cards, results).

#### Scenario: Security reveal without animation
- **WHEN** animations are off and a security check resolves
- **THEN** the revealed card and battle result are still shown (compactly or instantly) and the game proceeds without timed dwell

### Requirement: Suspend rotation toggle
When the rotate-suspended-cards toggle is disabled, suspended permanents SHALL be rendered upright with an explicit suspended indicator instead of the 90° rotation.

#### Scenario: Upright suspended rendering
- **WHEN** the toggle is off and a Digimon is suspended
- **THEN** its card renders upright with a visible suspended tag, and suspend state remains distinguishable at a glance
