## Why

The Rust engine's per-card faithfulness campaign is bottlenecked by how thoroughly we can stress-test it. Hand-written behavioral tests cover specific edge cases but cannot generate the long-tail card interactions that real games surface. DCGO — the community Unity client — already implements every card, hosts random matchmaking against humans, and ships a built-in bot for unattended play. By modding DCGO to record every action both players take (encoded directly into our 2192-action space) and building a Rust harness that replays those recordings through `digimon-engine`, we get two deliverables off one mod: (1) a high-volume faithfulness fuzzer that surfaces engine gaps automatically, and (2) a corpus of human-decision games usable as behavioral-cloning seed data for RL training.

## What Changes

- **DCGO modification** (`DCGO/Assets/Scripts/Script/Recording/`): new `GameRecorder` MonoBehaviour that intercepts `TurnStateMachine.QueueMainPhaseAction` and `UserSelectionManager.Set{Int,Bool}ForPlayer` for both players, encodes each decision into a 2192-space action ID, and writes one JSONL row per decision. Bot-vs-bot, bot-vs-human, and PvP all flow through the same chokepoints — one recorder serves all three modes.
- **Action-space codegen** (`code/tools/action-space-export/`): new Rust workspace member that imports `digimon_engine::action::space::*` and emits a JSON descriptor of all 2192 IDs, their ranges, formulas, and phase semantics. A tiny Python emitter consumes the JSON and generates `DCGO/Assets/Scripts/Script/Recording/ActionSpace.cs` so DCGO's encoder stays in lockstep with the Rust source of truth. CI gate: regenerate and diff.
- **Opaque-opponent-deck engine mode** (`code/digimon-engine/src/game.rs`, `lib.rs`): new constructor `Game::new_with_opaque_opponent(my_deck, opp_decklist_unordered)` plus `supply_reveal(player, card_id, source)` that lets the engine consume opponent-deck reveals from an external stream rather than its own RNG. Required for PvP replay (we never observe opponent deck order from one side) and also valuable for live RL inference against unknown opponents.
- **Rust replay harness** (`code/tools/dcgo-replay/`): new binary that consumes a DCGO JSONL recording, feeds the action stream through `HeadlessRunner`, asserts mask legality at each step, and reports first divergence (illegal action, mismatched winner, mismatched terminal state). Aggregates over a corpus into a parity report keyed by card identity — every divergence points at a specific Rust card script needing repair.
- **Behavioral-cloning dataset emitter** (`code/tools/dcgo-bc-emitter/`): consumes the same JSONL, replays through the engine, snapshots `(observation_tensor, action_mask, action_id)` per agent decision into numpy shards consumable by SB3.
- **Three-phase rollout**: Phase 1 ships the recorder + codegen + replay harness against bot-vs-bot games only (no opaque-deck dependency). Phase 2 adds opaque-opponent-deck mode. Phase 3 turns on PvP recording and the BC emitter.
- No change to the existing native engine recording path (`recording-replay` spec) — DCGO recordings are a separate format with their own consumer.

## Capabilities

### New Capabilities

- `dcgo-parity-harness`: the DCGO recording mod + Rust replay binary + parity report pipeline. Covers the JSONL schema, the encoder's phase-disambiguation contract, the replay binary's failure semantics, and the bot-vs-bot fuzzer loop.
- `opaque-opponent-deck`: engine mode where one player's deck composition is known but order is hidden; draws and security pops are supplied externally rather than drawn from a pre-shuffled list. Required by PvP recording; also reusable for RL inference and any future "play against a deck you've only partially scouted" workflow.

### Modified Capabilities

None. The DCGO recording format is intentionally distinct from the native `recording-replay` format — different producer, different schema — and the action-space-export codegen is a build tool with no behavioral spec surface.

## Impact

- **DCGO submodule**: requires building from source (Unity 2021.3.45f2) with the asset bundle pulled separately per the DCGO README. Mod adds new C# files under `Assets/Scripts/Script/Recording/`; touches `TurnStateMachine.cs` and `UserSelectionManager.cs` with one-line `Recorder.Log(...)` calls at the chokepoints. Patch is small enough to track as a single diff against an upstream-pinned commit.
- **Rust engine**: net-new `Game::new_with_opaque_opponent` + `supply_reveal` surface in `code/digimon-engine/src/game.rs`. Does not modify existing constructors; existing tests untouched. Likely interacts with `card_source.rs` (cards drawn from opaque deck must be instantiable from card-ID alone, same as `add_card_to_hand` in tests). RL gym wrapper unaffected for now (Phase 2 opt-in).
- **`code/tools/`**: two new Cargo workspace members (`action-space-export`, `dcgo-replay`) and one new Python tool (`dcgo-bc-emitter`). Mirrors the existing `tools/dsl-schema-export/` + `tools/dsl-lint/` pattern.
- **CI**: new check that `cargo run -p action-space-export | diff -` against the committed `ActionSpace.cs` is clean. Drift gate.
- **Action space**: NOT bumped. We're adding a consumer (DCGO), not changing the 2192-ID layout. Existing trained models are unaffected.
- **Data products**: new artifact paths under `recordings/dcgo/<timestamp>/game_NNN.jsonl` and `recordings/dcgo/parity_reports/`. Disk impact ~few KB per game compressed.
- **Legal / ethical**: the recorder is a local-write-only listener — it does not modify Photon traffic or interfere with other players' games. Phase 3 (PvP) recording captures opponent moves from public RPC events; the BC training pipeline keeps recordings local. Out-of-scope for this proposal: publishing recordings publicly.
