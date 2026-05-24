# Engine Debug MCP + CLI

Tools for interactively driving the Rust source-of-truth engine — for card debugging, smoke-test forensics, and training-run forensics.

Shipped by the `add-engine-debug-mcp` change. Two binaries plus engine additions.

```
┌──────────────────────────────────────────────────────────────────┐
│  digimon-engine  (lib — view layer, LiveGame, ReplayRunner)      │
└───────────────┬──────────────────────────────────────────────────┘
                │
       ┌────────┴────────┐
       ▼                 ▼
┌──────────────┐   ┌──────────────────┐
│ digimon-     │   │ digimon-         │
│ engine-cli   │   │ engine-mcp       │
│ (REPL + replay)  │ (stdio MCP server)│
└──────────────┘   └──────────────────┘
   for humans         for AI agents
```

## Build

```bash
cargo build -p digimon-engine-cli -p digimon-engine-mcp
# binaries land in target/debug/
```

## CLI: `digimon-engine-cli`

Three subcommands. All share global flags `--pool` and `--cards-json`.

### `debug` — interactive REPL

```bash
digimon-engine-cli debug --deck1 deck.json --deck2 deck.json --seed 7
```

REPL accepts one command per line. Type `help` for the full list. Highlights:

| Command | What it does |
|---|---|
| `new decks <d1> <d2> [--seed=N]` | Build a fresh game from deck JSON files |
| `load <recording.json> [--step=N]` | Load a recording paused at step N |
| `state [--view=player0|player1|god]` | Top-level state |
| `hand <player> [--view=...]` | Hand contents (opponent perspective hides cards) |
| `field <player>` | Battle area + breeding area |
| `security <player> [--view=...]` | Security stack (card IDs only in god view) |
| `pending` | Current `PendingSelection` with decoded options |
| `queue` | Effect queue in resolution order |
| `events [--since=N]` | Recent events |
| `actions <player>` | Every legal action_id, decoded |
| `play <player> <hand_idx>` | Play from hand |
| `resolve <player> <action_id>` | Resolve a selection |
| `step <action_id>` | Universal action gate |
| `end-turn`, `pass` | Turn flow |
| `inspect <card_id>` | Card metadata |

### `replay` — single-shot recording viewer

```bash
digimon-engine-cli replay rec.json --step 47 --show field --view god
digimon-engine-cli replay rec.json --verify           # walk + report divergences
```

Flags: `--step` (default 0), `--view` (god|player0|player1, default god), `--show` (state|hand|field|security|pending|queue|events|actions, default state), `--player` (default 0), `--verify` (compare replayed state to recorded values, exit 3 on any divergence).

### `scenario` — stubbed in v1

Returns non-zero with a "not yet implemented" message. The Python `ScenarioRunner` shape will get its own change proposal.

## MCP server: `digimon-engine-mcp`

Stdio JSON-RPC 2.0 per the [MCP spec](https://modelcontextprotocol.io/specification). Same `--pool` / `--cards-json` flags as the CLI, plus `--max-games N` (default 32) for the concurrent-games cap.

### Registering with a client

Edit `.mcp.json` and uncomment the `_digimon-engine-mcp` entry (drop the leading underscore):

```json
"digimon-engine-mcp": {
  "type": "stdio",
  "command": "target/debug/digimon-engine-mcp",
  "args": ["--pool", "implemented"]
}
```

For Claude Code, restart the client after editing `.mcp.json`.

### Tool surface (26 tools)

**Lifecycle (6):**
- `new_game_from_decks(deck1, deck2, seed?)` → `{ game_id }`
- `new_game_debug(hands, decks, first_player)` → `{ game_id }`
- `load_recording(recording_json | recording_path)` → `{ game_id }`
- `seek(game_id, step_n)` — **v1: stubbed**, use `load_recording` with a different step instead
- `list_games()` → `[{ game_id, turn_count, phase, ... }]`
- `close_game(game_id)` → `{ ok }`

**State inspection (12):**
- `state(game_id, view?)`
- `hand(game_id, player, view?)`
- `field(game_id, player, view?)`
- `security(game_id, player, view?)`
- `pending_selection(game_id)` — current selection with decoded option labels
- `effect_queue(game_id)`
- `events(game_id, since_seq?)`
- `modifiers(game_id, handle)`
- `inspect_card(game_id, card_id)`
- `legal_actions(game_id, player)`
- `deck_cards(game_id)` — full metadata for both decks (Phase 6.5)
- `recorded_actions(game_id, decode_labels?)` — recorded action log with optional human labels (Phase 6.5)

**Actions (8):**
- `play(game_id, player, hand_idx)`
- `digivolve(game_id, host, source_hand_idx)` — `host = {player, index}` permanent handle
- `attack(game_id, attacker, target)` — `attacker = {player, index}`, `target = {player, index} | "security"`
- `resolve_selection(game_id, player, action_id)`
- `end_turn(game_id)`
- `pass_turn(game_id)`
- `move_from_breeding(game_id, player)`
- `step(game_id, action_id)` — universal action gate; submit a raw `action_id` from `legal_actions`

### Result envelope

Every tool call returns `{ content: [{ type: "text", text: "<JSON>" }] }` per the MCP spec. Inside the `text` field:

- **Read tools** return their view JSON directly.
- **Action tools** return `ActionResult { ok, error, events_emitted, new_phase, pending_selection_after }`.
- **Tool-level errors** (illegal action, unknown game_id, at-capacity) appear as `{ ok: false, error: "..." }` — they don't escalate to JSON-RPC errors.
- **Protocol errors** (malformed args) appear as JSON-RPC `error` objects.

### Action validation

Action methods validate before dispatching. `ok: false` with a descriptive `error` is returned (engine state unchanged) for:

- `play()` outside `Main` phase, or by a player who is not `current_decision_player()`
- `step()` with an `action_id` that's not legal for `current_decision_player()` in the current phase
- `end_turn()` outside `Main` or `EndOfTurnAction`
- `pass_turn()` outside `Breeding`, `Main`, or `EndOfTurnAction`
- `play(player, hand_idx=99)` — hand index out of bounds

`step()`'s validation is intentional divergence from `HeadlessRunner::step` (which is fire-and-forget for the RL training loop). Debug callers need to detect failed actions; RL callers don't.

### Event format

`events_emitted` (in every `ActionResult`) and the `events` read tool return events as **structured JSON objects** with a top-level `type` field (matching `GameEvent::type_str()`) and variant-specific siblings:

```json
[
  {"type": "MemoryChange", "seq": 0, "player": 0, "delta": -3, "total": -3},
  {"type": "Play",         "seq": 1, "player": 0, "card_id": "BT24-008", "field_index": 0},
  {"type": "Digivolve",    "seq": 2, "player": 0, "top_card_id": "EX9-021", "field_index": 0, "from_stack_top": "BT22-026"},
  {"type": "GameOver",     "seq": 9, "winner": 0, "reason": "SecurityAttack"},
  {"type": "EffectFizzled","seq":10, "source_permanent": {"player": 0, "index": 1}, "reason": "no valid target"}
]
```

The `EffectFizzled` variant is emitted in two situations:
- **Install-time**: a selection helper's target filter matched zero entities (effect silently does nothing per existing engine convention, now observable via the event).
- **Execute-time**: `LiveGame::step` detected that the only option in a mandatory pending selection produced no state change; the wrapper clears the pending and emits the event so callers aren't soft-locked.

## Recipe cookbook

### Recipe 1 — Debug a card I'm implementing

```
1. Edit my Rust CardEffect impl in code/digimon-engine/src/cards/
2. Start CLI:    digimon-engine-cli debug
3. > new decks deck_with_my_card.json same.json --seed=1
4. > step 0    (mulligan keep)
5. > step 0
6. > actions 0
7. > step 4    (play my card from hand[4])
8. > pending   (was there a selection?)
9. > field 0   (did the card land correctly?)
```

### Recipe 2 — Investigate a training-run crash

```
1. Pull the crashed episode's recording from the training output.
2. Start MCP via Claude Code (after registering in .mcp.json):
3. Agent calls load_recording(recording_path: "runs/episode_42.json")
              → { game_id }
4. Agent calls deck_cards(game_id)
              → both decks' full metadata — now it knows what cards are in play
5. Agent calls recorded_actions(game_id, decode_labels: true)
              → the full action log with labels like "play hand[3]: Agumon"
6. Agent scans for the anomaly, then walks the live game to that step
   via `step` calls. Inspects state at each point.
```

### Recipe 3 — Reproduce a flaky smoke test

```
1. Smoke-test runner emits (decks, seed) on failure.
2. > new decks failing_deck.json same.json --seed=12345
3. Step through the game by hand, watching for the divergence.
```

## v1 limitations (worth knowing)

- **No `snapshot` / `restore` / branching.** Deferred to v1.5 once the engine's
  `card_data` is `Arc`-wrapped. Today, "what if I had picked differently" requires
  re-loading the recording or rebuilding the game.
- **`seek` (MCP) is stubbed.** Use `load_recording` with a different step instead.
- **Verify mode in CLI replay** catches memory/turn/phase/game-over divergences
  but doesn't compare full state.
- **`scenario` subcommand** is stubbed — Python `ScenarioRunner` port is its own
  change.
- **Cross-engine parity (Python recording → Rust replay)** works for the same
  recording schema, but no in-tree fixture pins the contract yet.

## Implementation pointers

- `code/digimon-engine/src/live_game.rs` — `LiveGame` wrapper, four constructors, view accessors, action methods
- `code/digimon-engine/src/runners/replay.rs` — `ReplayRunner` (port of `engine_py_legacy/engine/runners/replay_runner.py`)
- `code/digimon-engine/src/view/mod.rs` — `Perspective`, `StateView`, `HandView`, `FieldView`, etc.
- `code/digimon-engine/src/action/explain.rs` — `ActionExplanation`, `legal_decoded_actions`
- `code/digimon-engine-cli/src/` — REPL, replay viewer, scenario stub
- `code/digimon-engine-mcp/src/` — JSON-RPC framing, game registry, 26 tools
