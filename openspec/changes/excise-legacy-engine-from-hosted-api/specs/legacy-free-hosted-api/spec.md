## ADDED Requirements

### Requirement: Hosted API runs all gameplay logic on the Rust engine

The hosted API (`code/server/`) SHALL execute all live gameplay, replay, recording, and deck-legality logic on the Rust engine (directly or via PyO3 bindings) and SHALL NOT import any `engine_py_legacy.*` module.

#### Scenario: Server package imports with engine_py_legacy blocked

- **WHEN** `engine_py_legacy` is made unimportable (`sys.modules["engine_py_legacy"] = None`)
- **THEN** importing the hosted API app (`server.api`) and its routers succeeds without raising

### Requirement: Network state redaction is preserved on the Rust path

State broadcast to network clients SHALL be redacted such that an opponent never receives another player's `handIds` or `handCards`, matching the existing `state_filter` contract, when computed over Rust game state.

#### Scenario: Opponent view omits hidden hand data

- **WHEN** a per-player state view is produced for a client over Rust game state
- **THEN** the opposing player's `handIds` and `handCards` are absent or redacted

### Requirement: Deck legality and restricted list are served from the Rust deck tools

Deck parsing, validation, and restricted-list enforcement SHALL be served from the Rust deck tools, producing results equivalent to the prior Python deck-loader contract.

#### Scenario: Deck validation parity

- **WHEN** a deck is validated through the hosted API
- **THEN** the legality verdict and any restricted-list violations match the documented deck-legality contract

### Requirement: Recording and replay compatibility is preserved

Existing persisted recordings SHALL remain replayable after the server's replay path moves to the Rust replay core.

#### Scenario: Existing recording replays on the Rust path

- **WHEN** a recording produced before this migration is replayed through the hosted API
- **THEN** the replay completes and reaches the same terminal outcome as before the migration
