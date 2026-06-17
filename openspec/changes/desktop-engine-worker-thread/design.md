## Context

Desktop engine commands are synchronous `#[tauri::command]`s that lock
`RustEngineState { game: Arc<Mutex<Option<Game>>>, session: Arc<Mutex<GameSession>> }`
and run engine work inline. In Tauri v2 sync commands run on the UI main thread
(1 MB stack). A confirmed `STATUS_STACK_OVERFLOW` backtrace shows recursive
`bincode` deserialization of `digimon_dsl::compiled::CompiledStep` (a step
variant holds `Vec<CompiledStep>`) exhausting that 1 MB during game creation in
debug builds. The recursion is bounded (the 5 200+ behavioral tests deserialize
the same pack in debug on 2 MB test-thread stacks without overflow); the 1 MB
main thread is the sole constraint. Sync-on-main also freezes the UI during long
`run_agent_steps` bot turns.

The pure engine helpers (`run_agent_steps`, `build_action_mask`, `build_tensor`,
the DTO builders, `game_state_dto`, etc.) are correct and stay as-is — they
operate on `&mut Game` / `&GameSession`. Only *where* they run changes.

## Goals / Non-Goals

**Goals:**
- Engine execution always has a large, explicit stack — crash-proof for bounded
  recursion in debug *and* release.
- Engine work runs off the UI thread (no event-loop blocking during bot turns).
- Single-owner game/session/inference state (no `Mutex<Game>` shared across
  threads).
- Unchanged external command contract (names, args, DTOs, errors); unchanged
  frontend, hosted-API, and PyO3 paths.

**Non-Goals:**
- No engine-logic, rules, card, DTO-shape, or action-space changes.
- Not rewriting `CompiledStep` to a non-recursive/flat representation — the
  recursion is bounded; that would be disproportionate.
- Not changing the hosted-API (`RustHeadlessGame`) or training paths.

## Decisions

### D1 — One engine worker thread owns an `EngineWorld`

Spawn a single long-lived thread at startup with an explicit large stack:

```rust
thread::Builder::new()
    .name("digimon-engine".into())
    .stack_size(64 * 1024 * 1024)   // 64 MB — far beyond the bounded recursion
    .spawn(move || engine_worker_loop(job_rx))?;
```

It owns `EngineWorld { game: Option<Game>, session: GameSession, inference: InferenceState }`.
This is the single owner of all engine state — the `Arc<Mutex<…>>`s in
`RustEngineState` are removed. ONNX sessions are *created on the worker* (model
load runs there) and never cross threads.

### D2 — Closure executor, not a giant request enum

Rather than a ~21-variant `Request`/`Reply` enum (and matching reply-type
plumbing), the worker runs boxed closures over the world:

```rust
type Job = Box<dyn FnOnce(&mut EngineWorld) + Send + 'static>;
struct EngineHandle { tx: std::sync::mpsc::Sender<Job> }  // managed Tauri state, Clone
```

Each command keeps its existing body verbatim inside a closure that captures its
(owned) args and a reply sender. This preserves the "thin dispatcher → single
owner thread → reply" architecture chosen, with minimal boilerplate and no
per-command enum/reply churn. (Equivalent in behavior to an explicit `Create{..}`
enum; the closure form is just a lower-friction encoding of the same contract.)

### D3 — Async commands, non-blocking await

Commands become `async fn` and await the reply via a `tokio::sync::oneshot`, so
the UI thread is not blocked while the worker runs:

```rust
#[tauri::command]
pub async fn rust_create_game(engine: State<'_, EngineHandle>, /* args */)
    -> Result<CreateGameResponseDto, String>
{
    engine.run(move |world| { /* existing create logic over `world` */ }).await
}
```

`EngineHandle::run<R: Send + 'static>(f) -> impl Future<Output = Result<R,String>>`
sends `Box::new(move |world| { let _ = reply_tx.send(f(world)); })` and awaits
`reply_rx`. `send` failure → `Err("engine worker stopped")`; dropped reply →
`Err("engine worker did not reply")`.

### D4 — Inference moves into `EngineWorld`

`InferenceState` is owned by the worker (game-stepping needs it for `Trained`
seats, and the model commands `rust_load_model` / `_unload_model` /
`_list_loaded_models` become `engine.run` closures too). Benefit: ONNX sessions
are confined to one thread, sidestepping `Send`/`Sync` concerns entirely.

### D5 — Re-point the debug bridge at the worker

`debug_bridge::maybe_spawn` currently takes `state.game.clone()` /
`state.session.clone()` (the shared Arcs) and reads/stages state directly. It is
dev-only (`feature = "debug-bridge"`, env-gated). It will instead receive an
`EngineHandle` and perform its reads/mutations via `engine.run(...)` closures.
The `debug:state-changed` window event still fires after external mutations.

### D6 — Stack guarantee per platform + defense-in-depth

- The 64 MB worker `stack_size` is the primary guarantee on **all** platforms
  (Windows + Linux desktop builds) because engine code runs there.
- Defense-in-depth for any residual main-thread engine path (tests, future
  code): raise the binary's linked stack reserve via `code/src-tauri/build.rs`
  using **bin-scoped** link args (not a workspace-wide `rustflags`, to avoid
  cache churn and cross-crate effects):
  ```rust
  // Windows MSVC: 64 MB reserve
  println!("cargo::rustc-link-arg-bins=/STACK:67108864");
  ```
  (Linux main-thread stack is governed by `RLIMIT_STACK`, not the linker, and is
  not the constraint here since engine work is on the 64 MB worker; no Linux
  link-arg needed. Document this asymmetry.)

### D7 — Panic containment

The worker loop wraps each job in `catch_unwind(AssertUnwindSafe(..))` so a panic
in one unit of work doesn't kill the thread (subsequent commands still work). A
panicked job won't have sent its reply → the awaiting command observes a dropped
reply and returns `Err`. (A panic mid-mutation could leave `world` inconsistent;
engine ops are effectively transactional per action, so this is acceptable —
noted as a risk.)

## Risks / Trade-offs

- **Deadlock if a command's closure itself dispatches to the worker** → the
  worker is single-threaded and would block on itself. Mitigation: closures run
  the existing inline helpers directly on `world`; they never call back through
  `EngineHandle`. Enforce by construction (helpers take `&mut Game`, not the
  handle).
- **Debug-bridge refactor (D5) is the riskiest piece** (it mutates state out of
  band). Mitigation: it's dev-only and feature-gated; cover with the existing
  scenario-bridge flow; if time-boxed, the bridge can dispatch coarse-grained
  read/stage closures.
- **Async command + `State<'_, EngineHandle>` lifetimes** — async Tauri commands
  with borrowed `State` are supported; the closure captures only owned data +
  the (owned) reply sender, never the `State` borrow.
- **Worker panic leaves stale world** (D7) — bounded by per-action transaction
  granularity; a hard-corrupt world still only affects the current game (user can
  start a new game).
- **Build-arg cache churn** — using bin-scoped `build.rs` link args (not
  workspace `rustflags`) limits rebuilds to the `digimon-tcg` binary.
- **Two runtimes/threads to reason about** (tokio command tasks + the engine
  worker) — acceptable; the worker is a simple serial loop.

## Migration Plan

Pure desktop-shell change; no data/schema/wire migration.
1. Add `EngineWorld`, `EngineHandle`, the worker loop + spawn; `.manage` the
   handle; keep helpers untouched.
2. Convert the 18 game commands + 3 model commands to async `engine.run(...)`
   dispatchers (bodies moved verbatim into closures over `world`).
3. Re-point `debug_bridge::maybe_spawn` (D5).
4. Add the bin-scoped `/STACK` link arg (D6).
5. Verify: `cargo test --manifest-path code/src-tauri/Cargo.toml --lib`, then a
   live `cargo tauri dev` bot game (no crash, UI responsive during bot turns).
Rollback = revert the desktop-shell diff (no persisted state changes).

## Open Questions

- Worker stack size: 64 MB reserve is generous (virtual reserve, not committed);
  confirm it's comfortable on the Linux desktop target too. Default chosen: 64 MB.
- Should model file I/O (`rust_load_model`) run on the engine worker (serialized
  with gameplay) or its own thread? Default: on the worker (simplest, model loads
  are infrequent and not latency-critical).
- Do any commands need to run *concurrently* with a long bot turn (e.g. a
  surrender mid-turn)? Today everything is serialized by the `Mutex`; the single
  worker preserves that semantics. If concurrent surrender is desired later, it'd
  need a control channel — out of scope here.
