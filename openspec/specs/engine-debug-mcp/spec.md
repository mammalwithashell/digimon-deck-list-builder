### Requirement: digimon-engine-mcp Stdio Server

The system SHALL provide a `digimon-engine-mcp` binary that implements the Model Context Protocol over stdio. The binary SHALL be a Rust crate in the workspace that links directly against `digimon-engine`.

The server SHALL be registerable via `.mcp.json` with `"type": "stdio"` and an absolute or `PATH`-resolved command. Server startup SHALL accept the same `--pool` flag as the CLI, defaulting to `implemented`.

The server SHALL maintain an in-memory `HashMap<GameId, LiveGame>` so that game state persists across tool calls within one session. `GameId` SHALL be a server-generated opaque string (UUID or short token).

The server SHALL respond to MCP `initialize`, `tools/list`, and `tools/call` requests per the MCP specification. Tool schemas SHALL be valid JSONSchema.

#### Scenario: Server starts and lists tools

- **WHEN** an MCP client launches `digimon-engine-mcp` and issues `tools/list`
- **THEN** the server returns the documented tool set (lifecycle, state, action, replay) and exits cleanly on stdin close

#### Scenario: GameId persists across calls

- **WHEN** a client calls `new_game_from_decks` and receives `game_id == "abc"`, then calls `state` with `game_id == "abc"`
- **THEN** the second call operates on the same game and returns the post-construction state

#### Scenario: Unknown game_id returns error

- **WHEN** a client calls any tool with a `game_id` not previously returned by a lifecycle tool
- **THEN** the tool response is a structured error identifying the unknown game

---

### Requirement: Lifecycle Tools

The MCP server SHALL expose the following lifecycle tools:

- `new_game_from_decks(deck1: [card_id], deck2: [card_id], seed: number | null) -> { game_id }`
- `new_game_debug(hands: {p0, p1}, decks: {p0, p1}, first_player: 0 | 1) -> { game_id }`
- `load_recording(path_or_json: string) -> { game_id, total_steps }`
- `seek(game_id, step_n: number) -> { current_step }`
- `list_games() -> [{ game_id, source, current_step, turn_count, game_over }]`
- `close_game(game_id) -> { ok }`

The server SHALL impose an upper bound (configurable, default 32) on the number of concurrent games. Exceeding the bound SHALL return a structured error suggesting `close_game`.

#### Scenario: Lifecycle round-trip

- **WHEN** a client opens a game with `new_game_from_decks`, queries `list_games`, then `close_game`s it
- **THEN** the game appears in the list after open, is gone from the list after close, and subsequent calls with the freed `game_id` return unknown-game errors

#### Scenario: Concurrent limit enforced

- **WHEN** the configured limit is 32 and a client attempts to open a 33rd game
- **THEN** `new_game_from_decks` returns a structured error and no game is created

---

### Requirement: State Inspection Tools

The MCP server SHALL expose state-inspection tools that return view JSON (per the `live-game-surface` spec) and SHALL accept a `view` parameter selecting `player0`, `player1`, or `god` perspective. Default perspective SHALL be `god`.

Required tools:

- `state(game_id, view)` → `StateView`
- `hand(game_id, player, view)` → `HandView`
- `field(game_id, player, view)` → `FieldView`
- `security(game_id, player, view)` → `SecurityView`
- `pending_selection(game_id, view)` → `PendingSelectionView | null`
- `effect_queue(game_id)` → `EffectQueueView`
- `events(game_id, since_seq: number | null)` → `EventLogView`
- `modifiers(game_id, handle)` → `ModifierView`
- `inspect_card(card_id)` → `{ metadata, effect_text, script_path, csharp_path? }`
- `legal_actions(game_id, player)` → `[DecodedAction]`

These tools SHALL be read-only (idempotent). They SHALL NOT mutate game state.

#### Scenario: Pending selection inspection

- **WHEN** an active `LiveGame` has a `PendingSelection` and a client calls `pending_selection`
- **THEN** the response is a `PendingSelectionView` describing kind, min, max, and enumerated options

#### Scenario: No pending selection returns null

- **WHEN** no selection is active
- **THEN** `pending_selection` returns `null`

#### Scenario: Card metadata lookup

- **WHEN** a client calls `inspect_card("BT24-102")`
- **THEN** the response includes effect text and metadata for the card; if the card is implemented in Rust, `script_path` points at its `CardEffect` source file

---

### Requirement: Action Tools

The MCP server SHALL expose action-submission tools mirroring `LiveGame`'s action surface:

- `play(game_id, player, hand_idx)` → `ActionResult`
- `digivolve(game_id, host_handle, source_hand_idx, paid_costs?)` → `ActionResult`
- `attack(game_id, attacker_handle, target)` → `ActionResult`
- `resolve_selection(game_id, choice_indices)` → `ActionResult`
- `end_turn(game_id)` → `ActionResult`
- `pass_turn(game_id)` → `ActionResult`
- `step(game_id, action_id)` → `ActionResult`

Every action tool's response SHALL be the JSON-serialized `ActionResult`. Illegal actions SHALL NOT throw MCP errors; instead they SHALL return `ActionResult { ok: false, error: "..." }` so agents can read the rejection reason without breaking the call.

#### Scenario: Illegal action returns structured failure

- **WHEN** a client calls `play(game_id, 0, 99)` with an out-of-bounds hand index
- **THEN** the tool returns `ok: false` with an `error` string, and the underlying `LiveGame` state is unchanged

#### Scenario: Action emits events

- **WHEN** a client submits a legal `play` whose card has an OnPlay effect
- **THEN** `ActionResult.events_emitted` is non-empty and includes the OnPlay-related events in resolution order

---

### Requirement: Tool Surface Stability

Tool names and parameter schemas SHALL be considered stable contracts. Renaming a tool, removing a parameter, or narrowing a parameter type SHALL require a spec delta. Adding optional parameters and adding new tools SHALL NOT require a delta.

Schema responses SHALL be wire-stable: every field documented in the JSON shapes (per `live-game-surface` views) is part of the contract.

#### Scenario: Adding an optional parameter is non-breaking

- **WHEN** a future change adds an optional `limit` parameter to `events`
- **THEN** existing callers omitting `limit` continue to receive their prior behavior, with no spec delta required

---

### Requirement: Branching Tools Deferred

The MCP server SHALL NOT expose `snapshot`, `restore`, or `list_snapshots` tools in v1. These are explicitly deferred to v1.5 pending the engine refactor that makes `Game::Clone` cheap (Arc-wrapping `card_data` and registries).

When v1.5 ships, the tools SHALL be added under `## ADDED Requirements` in a new spec delta.

#### Scenario: Branching tools not advertised

- **WHEN** a client issues `tools/list` against v1 of the server
- **THEN** the returned tool list does not include `snapshot`, `restore`, or `list_snapshots`
