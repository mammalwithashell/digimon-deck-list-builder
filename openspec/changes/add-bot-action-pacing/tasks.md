# Tasks: add-bot-action-pacing

## 1. Desktop wire (Tauri)

- [x] 1.1 Add a step cap to the internal agent loop in `engine_commands.rs` (`run_agent_steps` gains `max_actions: Option<usize>`), returning whether further agent actions remain (separate `agent_pending(game, session)` helper)
- [x] 1.2 Add `agent_pending: bool` to the step/state response DTOs (`StepResponseDto`, `ActionResponseDto`) populated from the helper (unpaced mode always reports `false` at human-up / game-over, preserving current semantics)
- [x] 1.3 ~~`rust_advance_agent` command~~ **Design adjustment:** desktop `rust_create_game` runs no agent prelude (frontend drives agents via `rust_step_game`), so instead of a new command, `paced: Option<bool>` was added to `rust_step_game` (= the advance-one-beat call) and `rust_submit_action` (human action + at most one agent action). See design.md D2 (updated).
- [x] 1.4 Tauri-layer unit tests: paced advance executes exactly one action with correct `agent_pending`; paced beat-by-beat run reproduces the exact unpaced action sequence to game_over; `agent_pending` false for human decider and finished games (25 lib tests + 3 integration tests green)

## 2. Browser wire (FastAPI)

- [x] 2.1 `_autoplay_agent_turns` paced via its existing `max_steps` param (`=1`); `agent_pending` added unconditionally to `_build_state_payload` (derived: live game AND non-human decider)
- [x] 2.2 `paced` accepted on `CreateGameRequest` (ignored while `action_script` replays — staging is not presentation) and `GameActionRequest`; `POST /games/{id}/agent-step` advances exactly one agent action and is a safe no-op on human turn / game over
- [x] 2.3 Route tests (`code/tests/api/test_games_paced_agent.py`, 6 tests green): paced prelude caps at one beat, paced beats reach the identical terminal state as unpaced (same seed), paced human action, no-op safety, unpaced default, action_script+paced
- [x] 2.4 Cross-wire parity: both wires emit snake_case `agent_pending`; Rust serde test asserts the literal key in StepResponse/ActionResponse JSON, browser tests assert the same key in every payload

## 3. Frontend driver + setting

- [x] 3.1 Persisted `botSpeed` (`slow|normal|fast|instant`, default `normal`, delays 1500/900/400ms) in `uiStore` + `BotSpeedControl` segmented control rendered above the in-game ActionBar for local games (settings-page placement folded into the in-game control — there is no general settings page yet; GraphicsSettingsPage is desktop-only graphics)
- [x] 3.2 `paced` + `agent_pending` threaded through `gameApi.ts` on both runtimes (browser: `/agent-step` route for paced beats; desktop: `rust_step_game`/`rust_submit_action` invoke args); `agentPending` state + setter in `gameStore`
- [x] 3.3 Pacing driver extracted as `hooks/usePacedAgentDriver.ts` (testable): timer per beat, re-arms per applied response, instant-switch drains unpaced, retry-once then stall with `resumePacing`, auto-clears stall when nothing pending; `GamePage` consumes it + paces create/start/sendAction flows
- [x] 3.4 Human input locked while `agentPending` (mask swapped to empty for ActionBar + parsedMask — the in-flight mask belongs to the AGENT during beats — plus a sendAction guard); PromptBar gains `agentActing` "Opponent is acting…" bar; stall banner with Continue; per-beat traces flow through existing ticker/log
- [x] 3.5 `usePacedAgentDriver.test.tsx` (4 tests green, fake timers): beat sequencing until pending clears, instant bypass (unpaced call), retry-once→stall→resume, inert when inactive/idle; full vitest suite otherwise unchanged (2 pre-existing guest.test.ts failures + Playwright e2e collection errors exist on clean tree too)

## 4. Verification

- [x] 4.1 Live verification via the documented browser-dev path (uvicorn `PYTHONPATH=code` + `npm run dev:desktop` + Playwright): played vs greedy — Bot speed control renders, "Opponent is acting…" bar appears when the turn passes, the bot's turn resolves as paced beats (~2.8s at Normal vs instant before), board progresses correctly across turns. Measured timeline: pass at t=3ms → bar+turn-flip at t=632ms → human turn back at t=3398ms. **Desktop caveat:** the Tauri invoke path is covered by 25 unit + 3 integration tests but was not live-run in the app window — a `/run-desktop` spot-check at each speed is a recommended follow-up. (Browser-mode note: per-beat action traces stay empty in the ticker — pre-existing browser-wire gap, beats still visible via state changes/log.)
- [x] 4.2 RL path unaffected: `DigimonEnv` smoke green (`(8410,) (2192,)`); no paced code reachable from `HeadlessRunner`/gym (pacing lives only in the Tauri commands + `/games` routes)
