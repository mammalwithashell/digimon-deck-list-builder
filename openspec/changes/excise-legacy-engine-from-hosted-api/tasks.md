> **STATUS: ACTIVE — PvP cutover landed (2026-06-15).** The hosted API now runs
> all gameplay on the Rust engine; `code/server/` imports zero `engine_py_legacy`
> (guardrail-enforced). Phases 1–3 + 5 were delivered by
> `shrink-legacy-engine-surface`; phase 4 (live PvP) was done as a full cutover.
> Remaining: GameEvent wiring (animation parity — also benefits desktop) and the
> final `code/engine_py_legacy/` deletion. Note: the original "differential-test
> against the Python path" gate was dropped — the Python engine is bit-rotted and
> cannot serve as an oracle (see design findings 2026-06-14); the Rust faithfulness
> suite (DCGO replay + behavioral + judge-quiz) + new interactive-wire tests are the gate.

## 0. Prioritization gate

- [x] 0.1 Explicit decision to schedule this migration. **— prioritized 2026-06-15 ("use the Rust engine for all gameplay now" → full cutover).**
- [x] 0.2 Confirm `make-training-build-legacy-free` has landed (training already legacy-free). **— training is Rust-only/code-complete; the server was the remaining surface and is now migrated.**

## 1. Deck rules → Rust deck tools (lowest risk)

- [x] 1.1 Expose parse/validate/summarize + restricted-list over PyO3. **— already exposed in `digimon-engine-py`.**
- [x] 1.2 Differential-test against the Python deck-loader over a corpus. **— verified deck modes (standard/no_restriction/eden/titan/edh) on the Rust binding; restricted/EDEN lists byte-identical (shrink-legacy-engine-surface §2).**
- [x] 1.3 Migrate `simulations.py`, `lobby.py`, `db/routers/decks.py`, `db/routers/training.py`; re-home `PlayerType`. **— migrated; `PlayerType` had no consumer once `InteractiveGame` was removed (deleted, not re-homed).**

## 2. Replay + recordings → Rust replay core

- [x] 2.1 Expose the replay core over PyO3. **— `RustReplayRunner` added (shrink §3).**
- [x] 2.2 Recording-format compatibility gate. **— round-trip gate (record → replay → same winner) over `RustHeadlessGame` recordings.**
- [x] 2.3 Migrate `state.py`, `replays.py`, `recordings.py` off `ReplayRunner`/`HeadlessGame`. **— done; save gate → `RustHeadlessGame`, `record_actions=True`.**

## 3. State redaction over Rust state

- [x] 3.1 Implement redaction satisfying the `state_filter` contract. **— `state_filter.py` relocated to `code/server/`; engine-agnostic dict filter over Rust `to_ui_json` (shrink §1).**
- [x] 3.2 Differential-test redacted output (player + spectator). **— contract tests over real Rust `to_ui_json` output (Python differential dropped — bit-rotted, not an oracle).**
- [x] 3.3 Migrate `ws_manager.py`, `ws_games.py`. **— done (phase 4).**

## 4. Live PvP runtime → Rust interactive runner (the cutover)

- [x] 4.1 Interactive runner over the Rust engine, preserving the Python 1/2 player-id convention. **— NO new pyclass needed (finding): `RustHeadlessGame` already exposes the interactive surface. Built `code/server/rust_interactive_game.py` (`RustInteractiveGame`) — a thin Python adapter (step / get_action_mask / `.game.*` shim / surrender→concede / event drain). PvP is human-vs-human, so no bot/policy loop (that lives in `games.py`).**
- [x] 4.2 ~~Shadow-run against the Python engine~~. **— NOT POSSIBLE (Python bit-rotted). Validated instead via `code/tests/api/test_rust_interactive_game.py` (full game to a winner, surrender, redaction, events) + the engine's existing faithfulness suite.**
- [x] 4.3 Migrate `lobby.py`, `ws_*` to the Rust interactive runner. **— FULL cutover (Rust-only, no legacy fallback — the bit-rotted Python path is not a viable rollback). `matchmaking` rides on `lobby`. The whole `code/server/` is now legacy-free.**
- [x] 4.4 **Wire the unwired Rust `GameEvent`s.** **— DONE; smaller than feared: `Attack`/`Digivolve`/`Trash`/`SecurityReveal` (+ Play/MemoryChange/GameOver/Concede) were ALREADY wired (the `events.rs` docstring was stale). Only `TurnStart`/`PhaseChange`/`Mill` remained → wired via a `set_turn_phase` helper on the turn machine (`game_phases.rs`, main phases only — not the 60+ selection sub-phase sites) + `TurnStart` in `begin_turn` + `Mill` in `trash_from_top`. Engine `--lib` (226) + `cards_behavioral` (4636) green; Python event tests assert TurnStart/PhaseChange fire + PhaseChange names a main phase. NOTE (separate, pre-existing): engine events are PascalCase but the frontend animation components match snake_case (`'digivolve'`, `'battle_result'`, `'effect_activate'`) — some with NO Rust equivalent — so engine-event wiring does NOT drive frontend animations; that is a distinct frontend-adaptation task, not part of this change.**

## 5. Admin AI script_promotion

- [x] 5.1/5.2 Retire vs migrate → **RETIRED** (Python card-script lane; Rust DSL-first). **— done (shrink §4): import + 2 promote endpoints → 410; `GET /admin/promotions` keeps historical audits.**

## 6. Close-out

- [x] 6.1 Assert `code/server/` imports zero `engine_py_legacy`. **— `code/tests/api/test_legacy_free_support_surfaces.py` now asserts the WHOLE server (incl. ws_*/lobby/matchmaking + `server.api`) imports with `engine_py_legacy` blocked.**
- [ ] 6.2 Retire `docs/RUST_PYTHON_PARITY.md`; update `docs/ARCHITECTURE.md` / `docs/DEPLOYMENT.md`. **— REMAINING; gated on the final `code/engine_py_legacy/` deletion (the directory still ships the sunset engine + legacy tests + the relocated `run_scenario.py`). Parity doc stays as a live tracker until then.**
- [ ] 6.3 **Delete `code/engine_py_legacy/`** (the final gate) — relocate any remaining server/tool-imported modules first (none remain in production), migrate `test_rust_backend_parity.py` into `code/tests`, drop the two `pyproject.toml` exclusion lines. **— REMAINING; the actual directory removal.**
