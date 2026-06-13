## ADDED Requirements

### Requirement: Terminal result exits to launcher

After a bot or room game reaches a win/loss terminal result, the result overlay SHALL keep the result visible until the player activates the return action. Activating that action SHALL reset transient game state and navigate to the launcher route instead of the legacy in-game home state.

#### Scenario: Player returns after terminal result

- **WHEN** a player activates the result overlay return action after a win or loss
- **THEN** the current game state is reset
- **AND** the app navigates to `/`
- **AND** a desktop build displays the launcher page

#### Scenario: Seed remains available before return

- **WHEN** a game has ended and the result overlay is visible
- **THEN** the app remains on the game result surface until the player activates the return action
- **AND** the effective seed remains visible and copyable
