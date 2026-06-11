# Design: add-bot-action-pacing

## Context

Both human-facing wires drive agent seats to completion inside a single request:

- **Desktop**: `rust_step_game` (`code/src-tauri/src/engine_commands.rs`) calls an internal `run_agent_steps` loop that repeatedly picks the greedy/trained action until a human is up or the game resolves. Per-action traces (`actor: "agent_greedy"`) and events are accumulated and returned in one response.
- **Browser**: `_autoplay_agent_turns` (`code/server/routers/games.py`) does the same against `RustHeadlessGame`, draining per-step logs/events into buffers, returned in one response.

The frontend (`GamePage` + `gameStore`) applies the final state snapshot and appends the batched events, so animation components fire effectively simultaneously and the board "teleports" to the end of the bot's turn. RL paths (`HeadlessRunner`, gym env) do not go through these wires.

## Goals / Non-Goals

**Goals:**
- A human watching a bot seat (greedy now, trained later) can perceive each action: see it named, see its state delta, then see the next.
- User-tunable speed, persisted; "Instant" preserves today's behavior.
- Identical mechanism on desktop (Tauri invoke) and browser (HTTP) wires.
- Zero impact on training/eval throughput.

**Non-Goals:**
- Replaying events into reconstructed intermediate states from a single snapshot (a presentation-layer event sequencer). Rejected for now — see Decisions.
- Pacing WebSocket PvP (no agent seat) or spectator streams (server-paced already by the players' real cadence).
- A full replay scrubber. This change's request-driven pacing is a stepping stone, but timeline UI is separate work.

## Decisions

### D1: Frontend-driven single-step pacing, not backend sleeps, not event-replay

Three options considered:

1. **Backend sleeps** between agent actions inside the existing loop. Rejected: blocks the Tauri invoke / HTTP request for the whole bot turn, holds locks, makes speed a server concern, and the frontend still receives one batched response.
2. **Frontend event sequencer**: keep run-to-completion, but replay the returned event batch over time, animating intermediate states. Rejected for this change: requires deriving intermediate board states from events (a second state-application engine in the frontend) — high effort, high divergence risk. Worth revisiting when a replay scrubber is built.
3. **Frontend-driven single-step (chosen)**: backend gains a `paced` mode that executes **at most one agent action per request** and reports `agentPending: true` while more remain. The frontend renders each response normally (existing state-snapshot path, existing animations fire per-action because events now arrive per-action), waits `delayMs`, then requests the next step.

Option 3 reuses the entire existing render path — pacing falls out of request cadence, no new state machinery. Latency per step (~ms locally) is negligible against human-scale delays (300–1500ms).

### D2: Wire contract — additive option + additive flag

- Desktop: `rust_step_game` and `rust_submit_action` gain optional `paced: bool` (default `false`); response DTOs gain `agent_pending: bool`. **(As implemented:** no `rust_advance_agent` command — desktop `rust_create_game` runs no agent prelude and `rust_step_game` already *is* the no-action "advance agents" call, so `rust_step_game { paced: true }` is the advance-one-beat request. Field name is snake_case `agent_pending`, matching the browser wire's existing key convention.**)**
- Browser: `POST /games/{id}/actions` gains `paced` in the body; a `POST /games/{id}/agent-step` route advances one agent action. Response builder exposes `agentPending`.
- Game creation (`rust_create_game` / `POST /games`) also honors `paced` for the opening prelude (today it auto-plays an agent first turn before the first response).
- Defaults preserve current behavior for every existing caller and test.

### D3: Pacing policy lives in `uiStore`; the driver in `GamePage`

- `uiStore` gains `botSpeed: 'slow' | 'normal' | 'fast' | 'instant'` (persisted like resolution presets), mapped to inter-step delays (e.g. 1500 / 900 / 400 / 0ms-unpaced).
- `GamePage` effect: when last response has `agentPending` and `botSpeed !== 'instant'`, schedule the next agent-step request after the delay; cancel on unmount/new game/speed change. Human input is locked while `agentPending` (mask is empty anyway, but the UI should show "Opponent is acting…" via `PromptBar`).
- `instant` mode sends `paced: false` and keeps today's single-response flow — not "paced with 0 delay" — so the legacy path stays exercised and programmatic consumers are untouched.

### D4: Per-action visibility

Each paced response carries exactly one agent trace + its events. The existing `ActionTraceTicker` and `GameLog` render it; `DigivolveBanner`/`BattleEffect`/`SecurityRevealOverlay` now fire per action because events arrive per action (this alone fixes the "all animations at once" symptom). Slow/normal delays should exceed the longest transient animation (~1.4s DigivolveBanner) at 'slow'.

### D5: Interrupt timing windows

The agent loop today also auto-answers agent-side interrupts (block/counter timing) inside the same loop; paced mode steps through these identically — one decision per request — so the human sees "opponent declined to block" as a beat rather than it vanishing. No special-casing: anything the loop did in bulk, paced mode does one-at-a-time.

## Risks / Trade-offs

- [Stuck `agentPending` if a paced request fails] → driver retries once, then surfaces an error banner with a manual "continue" affordance; `agentPending` recomputed from every response, never accumulated client-side.
- [Desktop DTO drift (known failure mode: desktop DTOs lag browser)] → add `agentPending`/`paced` to BOTH wires in the same change with a parity test; see memory `project_desktop_dto_lags_browser`.
- [Trained-policy seats on browser wire don't exist yet (greedy only)] → pacing applies to whatever agent kinds the wire supports; no new coupling.
- [Long bot turns at 'slow' (10+ actions) feel sluggish] → 'fast' preset; also the speed setting is changeable mid-game (driver reads it per tick).
- [Scenario MCP / e2e fixtures assume run-to-completion] → defaults unchanged; paced mode is opt-in per request.

## Open Questions

- Should 'normal' be the shipped default rather than 'instant'? Proposal says pacing should be on by default for human games — recommend `normal` default for games with a human seat, `instant` for spectate-free programmatic creation; confirm during implementation.
