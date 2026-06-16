## Why

Desktop bot games currently start from a fixed Rust-engine seed, so deck order repeats every app session. That was useful while chasing deterministic failures, but normal play should randomize by default while still making the exact seed easy to capture and reuse.

## What Changes

- Add explicit game-seed controls for bot games and room matches, with blank/default seed input generating a fresh random seed.
- Surface the effective seed in-game and on the result overlay with a copy action so a player can replay a useful failure or bug report.
- Replace the desktop local-game hardcoded seed with a generated seed unless the user supplies one.
- Return seed metadata from game creation and room start paths so frontend state can display the same seed that initialized the engine.
- For room matches, make an explicit seed reproduce the whole setup, including the initial player; when a seed is supplied, the first-player selector is treated as seed-derived instead of independently overriding seed parity.
- After a game reaches a win/loss result, make the result action return to the launcher route instead of only resetting back to the legacy in-game home state.

## Capabilities

### New Capabilities

- `game-seed-controls`: Covers seed entry, random default generation, seed propagation, and in-game/result seed display for bot and room games.
- `game-result-launcher-return`: Covers terminal-game navigation back to the launcher route after the user leaves the result screen.

### Modified Capabilities

- `live-game-surface`: Extend the live game contract so created games expose their effective seed as replay/debug metadata.

## Impact

- Tauri desktop command/API surface: `rust_create_game`, create-game DTOs, seed parsing/generation, and tests around deterministic local starts.
- Browser/server game APIs: `/games` create/state/action responses, create request seed handling, and API schema tests.
- Room-match lobby APIs and UI: host seed controls, room state metadata, first-player interaction when explicit seeds are used, and active-room start tests.
- Frontend gameplay surfaces: bot launch controls, game store/play-flow state, in-game seed chip/copy action, result overlay copy action, and result navigation.
- No Rust engine action-space, observation tensor, card-effect, or legality-mask contract changes are expected.
