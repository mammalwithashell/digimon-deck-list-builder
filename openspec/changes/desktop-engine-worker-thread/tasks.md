## 1. Worker infrastructure

- [x] 1.1 Define `EngineWorld { game: Option<Game>, session: GameSession, inference: InferenceState }` (the single owner of engine state) in `engine_commands.rs` (or a new `engine_worker.rs`).
- [x] 1.2 Define `EngineHandle { tx: Sender<Job> }` (Clone) with `async fn run<R: Send + 'static>(&self, f: impl FnOnce(&mut EngineWorld) -> R + Send + 'static) -> Result<R, String>` using a `tokio::sync::oneshot` reply; map send-failure → `Err("engine worker stopped")`, dropped-reply → `Err("engine worker did not reply")`.
- [x] 1.3 Implement `engine_worker_loop(rx)` that owns the `EngineWorld` and runs each `Job` inside `catch_unwind(AssertUnwindSafe(..))` so a panicked job doesn't kill the thread.
- [x] 1.4 Spawn the worker via `thread::Builder::new().name("digimon-engine").stack_size(64<<20).spawn(...)`; return the `EngineHandle`.

## 2. Wire into the Tauri app

- [x] 2.1 In `main.rs`, spawn the worker in `setup()` and `.manage(EngineHandle)` (remove `.manage(RustEngineState)` and `.manage(InferenceState)` — inference now lives in `EngineWorld`).
- [x] 2.2 Re-point `debug_bridge::maybe_spawn` to take an `EngineHandle` (clone) and perform its reads/stages via `engine.run(...)`; preserve the `debug:state-changed` emit. (Dev-only / `feature = "debug-bridge"`.)

## 3. Convert game commands to async dispatchers

- [x] 3.1 Convert the 18 game commands (`create_test_game`, `get_rust_game_state`, `rust_play_card`, `rust_attack_digimon`, `rust_attack_player`, `rust_end_turn`, `rust_pass_turn`, `rust_mulligan_decide`, `rust_hatch`, `rust_move_from_breeding`, `rust_create_game`, `rust_submit_action`, `rust_step_game`, `rust_get_mask`, `rust_get_board_tensor_summary`, `rust_get_log`, `rust_surrender`, `rust_delete_game`) to `async fn` taking `State<'_, EngineHandle>`, moving each existing body verbatim into an `engine.run(move |world| { ... })` closure over `world.game` / `world.session` / `world.inference`. Keep the helper functions (`run_agent_steps`, `build_action_mask`, DTO builders, …) unchanged and called from inside the closures.
- [x] 3.2 Convert the 3 model commands (`rust_load_model`, `rust_unload_model`, `rust_list_loaded_models`) to async `engine.run(...)` over `world.inference`. (Also re-pointed `models::models_delete` / `models::models_load_cached`, which consumed the now-removed managed `InferenceState`, to dispatch through the worker's `world.inference`.)
- [x] 3.3 Ensure no closure calls back through `EngineHandle` (would self-deadlock the single worker); helpers operate directly on `&mut Game` / `world`.

## 4. Defense-in-depth stack reserve

- [x] 4.1 In `code/src-tauri/build.rs`, emit a bin-scoped Windows MSVC stack reserve: `println!("cargo::rustc-link-arg-bins=/STACK:67108864");` (guard to the msvc target). Document that Linux relies on the worker's explicit 64 MB stack (main-thread stack is `RLIMIT_STACK`-governed).

## 5. Verify

- [x] 5.1 `cargo test --manifest-path code/src-tauri/Cargo.toml --lib` — existing helper tests pass; add a test that `EngineHandle::run` dispatches to the worker and returns a value, and that a panicking job yields an `Err` without killing the worker (a subsequent `run` still succeeds). (59 passed with `--features debug-bridge`, 56 without; new tests `engine_handle_run_dispatches_to_worker_and_returns_value` + `engine_handle_panicking_job_errors_without_killing_worker`.)
- [~] 5.2 `cargo tauri dev` (per the run-desktop recipe): PARTIALLY VERIFIED. The full Tauri build compiled + launched with NO startup crash, the debug bridge bound on :5199, and a `POST /stage` (which runs `Game::new` + `full_card_data` + the recursive `CompiledStep` card-pack deserialization — the exact cdb-confirmed crash path) returned HTTP 200 with a valid game state while the process stayed alive (RSS grew 42→119 MB as the pack deserialized on the 64 MB worker). The crash is fixed. NOT yet done: the full UI click-through (bot game several turns) + UI-responsiveness observation — blocked because the machine's lock screen came up mid-session (environmental, not code).
- [ ] 5.3 Confirm the frontend works unchanged (same command names/DTOs): launcher → bot game → board renders, actions work; and the debug bridge (if enabled) can still read/stage state.
- [ ] 5.4 Regression: `cargo test --manifest-path code/digimon-engine/Cargo.toml --test combat` (and the broader suites if touched) remain green — engine crate is unchanged, so this is a sanity check only.
