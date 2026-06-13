# pvp-game-runtime

## ADDED Requirements

### Requirement: Online PvP runs on the Rust engine
Games created by the lobby and matchmaking paths SHALL be executed by the Rust engine (`digimon_engine.RustHeadlessGame`); the PvP path (`lobby`, `matchmaking`, `ws_games`, `ws_manager`) SHALL import zero `engine_py_legacy` symbols, including deck parsing.

#### Scenario: Lobby start constructs a Rust game
- **WHEN** a room starts
- **THEN** the runner placed in `active_games` is a `RustHeadlessGame` and the WebSocket endpoint accepts player connections to it

#### Scenario: No legacy imports remain in the PvP path
- **WHEN** the PvP modules are inspected (e.g. by a guardrail test)
- **THEN** none of `lobby.py`, `matchmaking.py`, `ws_games.py`, `ws_manager.py` import from `engine_py_legacy`

### Requirement: First-player choice maps to the created game
The room's first-player choice SHALL be realized via the game seed (`Game::new` first-player parity), with the parity→seat mapping pinned by a test: host choice `1` makes seat 1 act first, `2` makes seat 2 act first, `random` leaves the seed unconstrained.

#### Scenario: Deterministic first player
- **WHEN** a room starts with first player set to `1`
- **THEN** the created game's first decision belongs to player 1

### Requirement: Decision-player action routing over WebSocket
The WebSocket handler SHALL route actions by the runner's decision player: only the player whose decision is pending may submit, the action mask is sent only to that player, and submissions are validated against the mask. This SHALL hold for mid-turn decisions owned by the non-turn player (e.g. defender selections) and for mulligan keep/redraw, which flow through the normal action path.

#### Scenario: Defender selection routes to the defender
- **WHEN** a pending selection is owned by the non-turn player
- **THEN** that player receives the action mask and may submit, while the turn player's submission is rejected as not their decision

#### Scenario: Mulligan over WebSocket
- **WHEN** the game is in the mulligan phase
- **THEN** each player resolves keep/redraw through the standard action message using the action mask, with no special message type

#### Scenario: Illegal action is rejected
- **WHEN** a player submits an action id whose mask entry is 0
- **THEN** the server responds with an error message and the game state does not advance

### Requirement: Per-player and spectator state redaction
Every state broadcast to a network client SHALL pass through the server's state filter: a player never receives the opponent's `handIds` or `handCards`, neither player receives `securityIds`, and spectators receive the spectator-mode-appropriate view. Raw `to_ui_json()` output SHALL never be sent to a player or spectator.

#### Scenario: Opponent hand is redacted
- **WHEN** a state update is sent to player 1 from a live Rust game
- **THEN** player 2's `handIds` and `handCards` are empty in the payload while hand count remains visible

#### Scenario: Security stacks stay face-down
- **WHEN** a state update is sent to either player
- **THEN** both players' `securityIds` are empty in the payload

### Requirement: Concede and game-over reporting
A player SHALL be able to concede over the WebSocket, ending the game with the opponent as winner; on any game end the server SHALL broadcast a game-over message carrying the winner (and the conceding player when applicable) and release the game's connection tracking.

#### Scenario: Concede ends the game
- **WHEN** a player sends the surrender/concede message during a live game
- **THEN** all connected clients receive a game-over message naming the opponent as winner and identifying who conceded

#### Scenario: Game over by play
- **WHEN** a game reaches a terminal state through normal play
- **THEN** all connected clients receive a game-over message with the winner id and the game's connections are cleaned up
