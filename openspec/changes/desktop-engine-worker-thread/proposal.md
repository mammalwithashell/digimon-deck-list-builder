## Why

The desktop app's engine commands are synchronous `#[tauri::command]` functions
(`rust_create_game`, `rust_step_game`, `rust_submit_action`, …). In Tauri v2,
sync commands execute on the **UI/event-loop main thread**, which has a **1 MB
stack**. The compiled card pack (`cards.pack`) is bincode-deserialized into a
recursive `digimon_dsl::compiled::CompiledStep` tree during game creation; in a
**debug build** the monomorphized serde/bincode frames are large enough that the
(bounded) recursion overflows 1 MB → `STATUS_STACK_OVERFLOW`, crashing the dev
build intermittently on launch / game start. (The 5 200+ behavioral tests
deserialize the same pack in debug without crashing because Rust spawns test
threads with a 2 MB stack — proving the recursion is bounded and that the *only*
constraint is the 1 MB main thread.) The release build's smaller frames fit, so
shipped users are unaffected, but the documented `cargo tauri dev` workflow is
broken.

Running the engine on the UI thread is also the cause of UI freezing during bot
turns (a long synchronous `run_agent_steps` blocks the event loop).

## What Changes

- Introduce a single long-lived **engine worker thread**, spawned at startup
  with a large stack (e.g. 64 MB), that **solely owns** the game world
  (`Game`, the per-game `GameSession`, and the ONNX `InferenceState`). No more
  `Arc<Mutex<Option<Game>>>` shared across threads.
- All engine/model `#[tauri::command]`s become **thin dispatchers**: they send a
  unit of work to the engine thread and return its reply. Commands become
  `async` so awaiting the reply does **not** block the UI thread.
- Engine code therefore always runs on the large-stack worker, off the UI
  thread — durably eliminating the stack-overflow crash (debug *and* release,
  for any realistically-bounded card complexity) **and** the UI-freeze-during-
  bot-turns jank.
- Defense-in-depth: also raise the binary's linked main-thread stack reserve, so
  any incidental main-thread engine call (tests, future code) has headroom.
- No engine-logic, card, or rules changes; the pure helper functions
  (`run_agent_steps`, `build_action_mask`, DTO builders, …) are unchanged and
  continue to be called — now from inside the worker.

## Capabilities

### New Capabilities
- `desktop-engine-execution`: How the desktop shell executes engine work —
  ownership of game/session/inference state, the thread it runs on, the stack
  budget guarantee, and the command-dispatch contract.

### Modified Capabilities
<!-- None. No existing capability defines the desktop engine-execution model. -->

## Impact

- **`code/src-tauri/src/engine_commands.rs`** — the 18 game commands + 3 model
  commands become async dispatchers; `RustEngineState` (the shared `Mutex`s) is
  replaced by an `EngineHandle` (job sender) managed by Tauri; introduce
  `EngineWorld` (owned by the worker) and the worker spawn/loop.
- **`code/src-tauri/src/main.rs`** — spawn the engine worker in `setup()` and
  `.manage(EngineHandle)` instead of `RustEngineState`; the debug-bridge wiring
  (`debug_bridge::maybe_spawn`) currently shares the game/session Arcs and must
  be re-pointed at the worker (see design).
- **`code/src-tauri/src/inference_state.rs`** — `InferenceState` moves into
  `EngineWorld` (created/owned on the worker; ONNX sessions never cross threads).
- **Build config** — `.cargo/config.toml` (or `build.rs`) adds a generous
  `/STACK` reserve for the Windows MSVC target (and equivalent for Linux).
- **Tests** — existing direct-helper tests in `engine_commands.rs` keep working
  (helpers unchanged); add coverage for the worker dispatch + the stack
  guarantee.
- **No change** to the frontend wire contract (same command names, args,
  response DTOs) or to the hosted-API / PyO3 paths.
