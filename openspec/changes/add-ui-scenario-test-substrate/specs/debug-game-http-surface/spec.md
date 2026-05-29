## ADDED Requirements

### Requirement: Rust-backed debug game HTTP router

The hosted API SHALL expose a `/debug` router backed by `RustDebugGame` that lets browser-mode clients create and manipulate staged games over HTTP. The router MUST NOT import from `engine_py_legacy`. It MUST be registered before the database-dependent routers so its paths are not consumed by a catch-all `/{id}` route, mirroring the `desktop_decks` registration ordering.

#### Scenario: Create a staged debug game over HTTP
- **WHEN** a client POSTs a staging payload (decks, per-player zones, memory, phase, first player, skip-shuffle) to the debug create endpoint
- **THEN** the server constructs a `RustDebugGame`, registers it in the active-games store, and returns a game id plus the initial state and action mask

#### Scenario: Debug router does not touch the legacy engine
- **WHEN** the hosted API starts
- **THEN** the debug router is importable and functional without `engine_py_legacy` being present

### Requirement: Debug mutation and inspection endpoints

The `/debug` router SHALL provide endpoints for the operations the e2e fixture relies on: set-memory, inject-card, internal-state, place-on-field, and bulk zone setup. A staged game created through the debug router MUST be steppable and inspectable through the existing live `/games/{id}/...` action, step, state, and action-mask routes, so a test can stage via `/debug` and then play via `/games`.

#### Scenario: Stage then play
- **WHEN** a game is created via the debug create endpoint and then an action is submitted to `POST /games/{id}/actions`
- **THEN** the action resolves against the staged game and the response carries the updated state, mask, logs, and events

#### Scenario: Inject and inspect over HTTP
- **WHEN** a client injects a card into player 1's hand via the debug inject endpoint and then GETs the debug internal-state endpoint
- **THEN** the internal-state response shows the injected card in player 1's hand

### Requirement: Action-listing endpoint for tests

The API SHALL provide the action information the e2e fixture needs to choose actions. Tests MUST be able to obtain the set of currently legal action ids for an active game. The action mask exposed at `GET /games/{id}/action-mask` is the canonical source; any debug-only convenience that returns decoded action labels MUST derive from the same mask.

#### Scenario: Fixture resolves a digivolve action from the mask
- **WHEN** the e2e fixture requests the action mask for a staged game
- **THEN** it receives the array of legal action ids and can select the digivolve/DNA action range without relying on the removed `GET /games/{id}/actions` route

### Requirement: Legacy debug router removed

The legacy `engine_py_legacy`-based `/debug` router SHALL be removed. Its behavior is replaced one-for-one by the Rust-backed router for the endpoints the e2e suite uses.

**Reason**: The legacy router imports the sunset Python engine (forbidden for production code by CLAUDE.md rule #22) and depends on bit-rotted Python card scripts that break game creation.

**Migration**: Use the Rust-backed `/debug` endpoints, which preserve the create-with-staging, set-memory, inject-card, and internal-state surface the existing `debug-game.ts` fixture calls.

#### Scenario: Legacy debug import is gone
- **WHEN** the codebase is searched for `engine_py_legacy` imports in the debug router
- **THEN** none are found, and the `/debug` routes are served by the Rust-backed implementation
