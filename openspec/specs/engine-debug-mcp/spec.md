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

The MCP server SHALL expose the following read-only state-inspection tools:

- `state(game_id, view?)` → `StateView`
- `hand(game_id, player, view?)` → `HandView`
- `field(game_id, player, view?)` → `FieldView`
- `security(game_id, player, view?)` → `SecurityView`
- `pending_selection(game_id, view?)` → `PendingSelectionView | null`
- `effect_queue(game_id)` → `EffectQueueView`
- `events(game_id, since_seq?)` → `EventLogView`
- `modifiers(game_id, handle)` → `ModifierView`
- `inspect_card(card_id)` → `{ metadata, effect_text, script_path, csharp_path? }`
- `legal_actions(game_id, player)` → `[DecodedAction]`

These tools SHALL be read-only (idempotent). They SHALL NOT mutate game state.

`legal_actions(game_id, player)` SHALL return an empty array when `player != current_decision_player()`. The returned actions SHALL be executable via `step` at the moment of the call.

The `events` tool SHALL return events as structured JSON objects with stable field names — NOT `Debug`-formatted strings.

#### Scenario: Pending selection inspection

- **WHEN** an active `LiveGame` has a `PendingSelection` and a client calls `pending_selection`
- **THEN** the response is a `PendingSelectionView` describing kind, min, max, and enumerated options

#### Scenario: No pending selection returns null

- **WHEN** no selection is active
- **THEN** `pending_selection` returns `null`

#### Scenario: Card metadata lookup

- **WHEN** a client calls `inspect_card("BT24-102")`
- **THEN** the response includes effect text and metadata for the card; if the card is implemented in Rust, `script_path` points at its `CardEffect` source file

#### Scenario: legal_actions for inactive player returns empty

- **WHEN** a client calls `legal_actions(game_id, 0)` during a moment when `current_decision_player()` is 1
- **THEN** the response is an empty JSON array `[]`, regardless of what actions player 0 would have if it were their turn

#### Scenario: events returns structured event objects

- **WHEN** a client calls `events(game_id, since_seq=0)` after one or more actions have produced events
- **THEN** the response's `events` array contains entries with `kind` and per-variant fields accessible via JSON property access; entries are NOT strings of the form `'MemoryChange { ... }'`

---

### Requirement: Action Tools

The MCP server SHALL expose action-submission tools mirroring `LiveGame`'s action surface:

- `play(game_id, player, hand_idx)` → `ActionResult`
- `digivolve(game_id, host, source_hand_idx, paid_costs?)` → `ActionResult`
- `attack(game_id, attacker, target)` → `ActionResult`
- `resolve_selection(game_id, player, action_id)` → `ActionResult`
- `end_turn(game_id)` → `ActionResult`
- `pass_turn(game_id)` → `ActionResult`
- `move_from_breeding(game_id, player)` → `ActionResult`
- `step(game_id, action_id)` → `ActionResult` — universal action gate; pass an action ID from `legal_actions`.

Every action tool's response SHALL be the JSON-serialized `ActionResult`. Illegal actions SHALL NOT throw MCP errors; instead they SHALL return `ActionResult { ok: false, error: "..." }` so agents can read the rejection reason without breaking the call.

`events_emitted` inside every `ActionResult` SHALL contain structured `GameEvent` JSON objects with stable field names — NOT `Debug`-formatted strings. The same constraint applies to the `events` read tool.

`digivolve` and `attack` SHALL accept typed arguments: `host` and `attacker` are permanent handles serialized as `{"player": <0|1>, "index": <u8>}` matching the `handle` field returned by the `field` view; `target` for `attack` SHALL accept either a permanent handle (battle-attack) or the literal string `"security"` (security-attack); `paid_costs` for `digivolve` is an optional list of cost specifiers consumed by the digivolve action decoder.

#### Scenario: Illegal action returns structured failure — out of bounds

- **WHEN** a client calls `play(game_id, 0, 99)` with an out-of-bounds hand index
- **THEN** the tool returns `ok: false` with an `error` string, and the underlying `LiveGame` state is unchanged

#### Scenario: Illegal action returns structured failure — wrong decision player

- **WHEN** a client calls `play(game_id, 0, 0)` while it is player 1's turn (Main phase) and the active decision player is 1
- **THEN** the tool returns `ok: false` with an `error` describing the decision-player mismatch, and the `LiveGame` state is unchanged

#### Scenario: Illegal action returns structured failure — wrong phase

- **WHEN** a client calls `play(game_id, 0, 0)` during the Mulligan phase
- **THEN** the tool returns `ok: false` with an `error` describing the phase mismatch, and the `LiveGame` state is unchanged

#### Scenario: Illegal step returns structured failure

- **WHEN** a client calls `step(game_id, action_id)` with an `action_id` that is not legal for `current_decision_player()` in the current phase
- **THEN** the tool returns `ok: false` with an `error` naming the action_id and the current decision player / phase, and the `LiveGame` state is unchanged

#### Scenario: Illegal end_turn / pass_turn returns structured failure

- **WHEN** a client calls `end_turn(game_id)` or `pass_turn(game_id)` during a phase where the action is not engine-legal (e.g., Mulligan)
- **THEN** the tool returns `ok: false` with an `error` describing the phase mismatch, and `turn_count` / `current_phase` / `pending_selection` are unchanged

#### Scenario: Action emits structured events

- **WHEN** a client submits a legal `play` whose card has an OnPlay effect that emits a MemoryChange and a Play event
- **THEN** `ActionResult.events_emitted` is a JSON array whose entries are structured event objects, e.g., `[{"type": "MemoryChange", "seq": 0, "player": 0, "delta": -3, "total": -3}, {"type": "Play", "seq": 1, "player": 0, "card_id": "BT24-008", "field_index": 0}]` — each entry has a top-level `type` field matching `GameEvent::type_str()`, variant-specific fields are siblings (no `meta` wrapper), and the client can access fields like `events_emitted[0].delta` directly without parsing strings

#### Scenario: Mandatory selection with unfulfillable option fizzles

- **WHEN** a client encounters a mandatory `pending_selection` whose only option is unfulfillable AND calls `step(game_id, action_id)` for that option
- **THEN** the tool returns `ok: true` with an `events_emitted` entry of the form `{"type": "EffectFizzled", "seq": N, "source_permanent": {…}, "reason": "no executable target"}`; `pending_selection_after` is `null`; the engine continues normal phase / turn progression

#### Scenario: digivolve tool dispatches digivolution

- **WHEN** a client calls `digivolve(game_id, host={"player":0,"index":1}, source_hand_idx=3)` and the move is legal
- **THEN** the tool returns `ok: true` with structured events covering memory change, digivolve event, draw, and any When-Digivolving triggers; the host permanent's stack now includes the source card on top

#### Scenario: attack tool dispatches attack

- **WHEN** a client calls `attack(game_id, attacker={"player":0,"index":0}, target="security")` and the attacker can legally attack security
- **THEN** the tool returns `ok: true` with structured attack/security events; pending selections (blocker timing, security trigger effects) surface in `pending_selection_after`

#### Scenario: digivolve / attack illegal returns error

- **WHEN** a client calls `digivolve` or `attack` with arguments that don't resolve to a legal action ID (e.g., suspended attacker, no matching digivolve source)
- **THEN** the tool returns `ok: false` with an `error` describing why no legal action matched

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
