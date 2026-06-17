## ADDED Requirements

### Requirement: Non-PvP hosted-API surfaces import no engine_py_legacy

The hosted API's deck-legality, replay, recording, and state-redaction code paths — every server surface EXCEPT the live PvP/WebSocket runtime — SHALL execute on the Rust engine (directly or via PyO3) and SHALL NOT import any `engine_py_legacy.*` module. The PvP/WebSocket runtime (`ws_games.py`, `ws_manager.py`, and `lobby.py`'s `InteractiveGame` construction) is explicitly out of scope and tracked by `excise-legacy-engine-from-hosted-api`.

#### Scenario: Non-PvP routers import with engine_py_legacy blocked

- **WHEN** `engine_py_legacy` is made unimportable (`sys.modules["engine_py_legacy"] = None`)
- **THEN** importing `server.routers.simulations`, `server.routers.replays`, `server.routers.recordings`, `server.routers.state`, `server.db.routers.training`, and `server.db.routers.decks` succeeds without raising

### Requirement: State redaction is served from a production module over Rust state

Per-recipient state redaction SHALL live in a production package (not `engine_py_legacy`) and SHALL redact a hidden player's `handIds`, `handCards`, and `securityIds` from `RustHeadlessGame.to_ui_json()` output, preserving the rules 9 & 14 contract (an opponent never receives another player's hand metadata).

#### Scenario: Opponent view omits hidden hand data over Rust state

- **WHEN** a per-player view is produced from a Rust `to_ui_json` dict for a given player
- **THEN** the opposing player's `handIds` and `handCards` are empty/absent and the public zones (battle area, trash, breeding, memory) are unchanged

### Requirement: Deck legality served from the Rust deck tools for non-PvP consumers

Deck parsing, validation, summary, and restricted-list enforcement for the non-PvP server consumers SHALL be served from the Rust deck tools, producing verdicts equivalent to the prior Python deck-loader contract, including the `no_restriction`, `pauper`, `eden`, `titan`, and `edh_commander` modes.

#### Scenario: Deck validation parity

- **WHEN** a deck is validated through a non-PvP server consumer
- **THEN** the legality verdict and the set of restricted-list violations match the documented deck-legality contract

### Requirement: Replay and recordings run on the Rust replay core

Server replay and recording SHALL run on the Rust replay core via PyO3, and recordings persisted before this change SHALL remain replayable.

#### Scenario: Existing recording replays on the Rust path

- **WHEN** a recording persisted before this change is replayed through the hosted API
- **THEN** the replay completes and reaches the same terminal outcome (winner / result) it did under the Python path

### Requirement: The Python script-promotion lane is retired

The admin AI `script_promotion` flow (frozen/generated Python card-script promotion) SHALL be removed from the hosted API, and the server SHALL NOT import `engine_py_legacy.engine.data.script_promotion`.

#### Scenario: Server admin router imports without the script-promotion lane

- **WHEN** `engine_py_legacy` is made unimportable
- **THEN** importing `server.db.routers.admin_ai` succeeds without raising

### Requirement: The code/tools CLI imports no engine_py_legacy

Retained scripts under `code/tools/` SHALL import zero `engine_py_legacy.*` symbols; obsolete Python-script-lane tools SHALL be deleted.

#### Scenario: Retained tools import with engine_py_legacy blocked

- **WHEN** `engine_py_legacy` is made unimportable
- **THEN** importing each retained `code/tools/` module (e.g. `tools.resolve_deck`, `tools.meta_loader`, `tools.ingest_cards`) succeeds without raising
