## ADDED Requirements

### Requirement: Desktop debug staging commands

The Tauri desktop shell SHALL provide debug staging commands that operate on the live `RustEngineState` game by wrapping the same engine staging primitives used by the browser `/debug` path: stage zones (hand, deck order, field stacks, breeding, security, trash), inject a single card, place a digivolution stack on field, bulk-replace multiple zones, set the memory gauge, set phase/turn/first-player, step by a legal action, read full-information internal state, and export the current game to a scenario fixture. These commands MUST NOT duplicate staging logic — they delegate to the shared `Game::stage_*` / `Game::to_scenario()` engine API. Because the Tauri process holds a single game, the commands operate on that implicit game with no game id.

#### Scenario: Staging replaces the desktop game's zones

- **WHEN** a desktop debug stage command sets player 1's hand to a list of card ids
- **THEN** the live desktop game's hand becomes exactly those cards, and a subsequent internal-state read reflects them

#### Scenario: Illegal staged board is rejected

- **WHEN** a desktop staging command would produce a rule-illegal board
- **THEN** the command returns an error identifying the offending field rather than leaving the game in an undefined state

### Requirement: Gated localhost bridge server

The desktop shell SHALL be able to expose the debug staging commands over a localhost-only HTTP server so an external process can drive the running desktop game. The server MUST be gated by BOTH a compile-time `debug-bridge` cargo feature AND a runtime environment variable, so it is absent from release builds and inert in debug builds unless explicitly enabled. It MUST bind only to a loopback address. Its endpoints mirror the staging command verbs.

#### Scenario: Bridge absent from production builds

- **WHEN** the desktop app is built without the `debug-bridge` feature (the release/prod configuration)
- **THEN** no bridge server is compiled in and the binary exposes no network listener for staging

#### Scenario: Bridge inert unless opted in

- **WHEN** a `debug-bridge`-enabled build runs without the enabling environment variable set
- **THEN** no bridge server starts

#### Scenario: Bridge drives the live game

- **WHEN** the bridge is enabled and an external client posts a stage request to the loopback endpoint
- **THEN** the live desktop game is staged accordingly and a subsequent read returns the staged state

### Requirement: Webview refresh after external mutation

When the game is mutated through the bridge (outside the normal `invoke` flow), the shell SHALL notify the webview so the rendered board does not go stale. The bridge MUST emit a state-changed window event after each external mutation, and the frontend MUST refresh its game state on that event.

#### Scenario: Board refreshes after external staging

- **WHEN** an external client stages a new board via the bridge while the desktop window is open
- **THEN** the window emits the state-changed event and the rendered board updates to the staged state without a manual reload
