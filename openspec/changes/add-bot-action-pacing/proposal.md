# Proposal: add-bot-action-pacing

## Why

Playing against the bot is currently imperceptible: both game wires run the agent's *entire* turn inside a single request (`_autoplay_agent_turns` in `code/server/routers/games.py`, `run_agent_steps` in `code/src-tauri/src/engine_commands.rs`), so the frontend receives one final state snapshot and renders every bot action — plays, digivolves, attacks, security checks — all at once. A human cannot follow what the opponent did, which breaks the core play-vs-AI experience the UI exists for (and makes watching RL-model games for QA equally useless). RL training must keep running the engine at full speed; only the human-facing UI path needs pacing.

## What Changes

- Add a **paced agent-step mode** to both game wires: instead of "run agents until a human is up", the backend can execute **one agent action per request**, returning that action's trace + events + post-action state, plus a flag that more agent steps remain.
  - Desktop: `rust_step_game` (and game-creation prelude) honors a `paced` option that caps the internal agent loop at one action.
  - Browser: the `/games` step/state routes honor the same option for greedy-agent seats.
- Add a **frontend pacing driver** in `GamePage`: when a response says the agent has more steps, render the action (with its events/animations), wait a configurable inter-action delay, then request the next agent step. The human's input stays locked while the agent sequence is draining.
- Show **what the bot just did** while pacing: surface the action trace label for each paced step (the existing `ActionTraceTicker` / log machinery), so each action is both seen and named.
- Add a **bot speed setting** (e.g. Slow / Normal / Fast / Instant) persisted in `uiStore`; Instant reproduces today's run-to-completion behavior and remains the default for spectator-less programmatic use.
- **No engine or RL-path changes**: `HeadlessRunner`, the gym env, and training/eval flows never enter the paced path; pacing lives entirely in the human-facing request loop. The unpaced mode stays the wire default so existing clients/tests are unaffected.

## Capabilities

### New Capabilities
- `bot-action-pacing`: Human-facing bot games reveal agent actions one at a time at a human-perceivable, user-configurable pace, on both the desktop (Tauri) and browser (HTTP) wires, without slowing any RL/training path.

### Modified Capabilities
<!-- None. The existing step contract keeps its default behavior; pacing is an opt-in request option. -->

## Impact

- **Desktop wire**: `code/src-tauri/src/engine_commands.rs` (`run_agent_steps` loop gains a step cap + "agent pending" flag in the step response DTO).
- **Browser wire**: `code/server/routers/games.py` (`_autoplay_agent_turns` gains the same cap; response builder exposes the pending flag).
- **Frontend**: `code/frontend/src/pages/GamePage.tsx` (pacing driver), `stores/uiStore.ts` (persisted bot-speed setting), `stores/gameStore.ts` (agent-pending state), `components/game/ActionTraceTicker.tsx` / log (per-action display); both API clients (`api/gameApi.ts`, Tauri invoke wrapper) thread the option.
- **Not affected**: `digimon-engine` core, `digimon_gym`, training/eval entrypoints, WebSocket PvP (no agent seat), scenario MCP.
- **Tests**: Tauri-layer unit tests for the capped loop; server route test for single-step mode; frontend behavior is covered by the Playwright scenario substrate where practical.
