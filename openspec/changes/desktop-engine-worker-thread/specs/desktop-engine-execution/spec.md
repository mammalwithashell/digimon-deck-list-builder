## ADDED Requirements

### Requirement: Engine work runs on a large-stack worker thread

Engine execution SHALL run on a dedicated worker thread created with a stack
size large enough to absorb the engine's bounded recursion (including bincode
deserialization of the `CompiledStep` card-spec tree) in both debug and release
builds. This covers game creation, action submission, agent/bot stepping,
mask/tensor/log queries, and model load/predict. Engine code MUST NOT execute on
Tauri's UI/event-loop main thread.

#### Scenario: Game launch does not overflow the stack in a debug build

- **WHEN** a game is created (and a bot takes its turn) in a debug desktop build
- **THEN** the card pack deserializes and the game runs to a stable board with
  no `STATUS_STACK_OVERFLOW` crash

#### Scenario: Engine work is off the UI thread

- **WHEN** any engine command executes
- **THEN** the work runs on the engine worker thread, not the main thread

### Requirement: Single-owner game state

The game world SHALL be owned solely by the engine worker thread — namely the
`Game`, its per-game session metadata (card registry, player kinds, model ids),
and the inference/model state. The shared `Mutex`-guarded game state MUST be
removed; no other thread accesses the `Game` directly.

#### Scenario: No shared mutable game state

- **WHEN** the desktop engine layer is built
- **THEN** there is no `Arc<Mutex<…Game…>>` shared between the command threads
  and the worker; commands reach the game only by dispatching work to the worker

### Requirement: Commands dispatch to the worker without blocking the UI thread

Each engine/model `#[tauri::command]` SHALL submit its work to the engine worker
and return the worker's result, awaiting that result in a way that does not
block the UI/event-loop thread (i.e. the command is async and yields while the
worker runs). The command's external contract (name, arguments, response DTO,
error type) MUST be unchanged from the caller's perspective.

#### Scenario: UI stays responsive during a long bot turn

- **WHEN** the bot takes a multi-action turn that the worker processes
- **THEN** the UI event loop is not blocked for the duration of that work

#### Scenario: Command contract is preserved

- **WHEN** the frontend invokes an engine command (e.g. `rust_create_game`)
- **THEN** it receives the same response shape and error semantics as before the
  change

### Requirement: Worker failures surface as command errors, not crashes

The dispatching command SHALL return an error result (rather than panic or hang)
if the engine worker thread cannot service a request (e.g. it has stopped). A
panic inside a single unit of engine work MUST be contained so it returns an
error for that command without tearing down the whole app where feasible.

#### Scenario: Worker unavailable returns an error

- **WHEN** a command dispatches work but the worker channel is closed
- **THEN** the command returns an `Err(...)` describing the failure instead of
  panicking the caller

### Requirement: Defense-in-depth stack reserve

The desktop binary SHALL be linked with a generous main-thread stack reserve so
that any engine code path that still runs on the main thread (e.g. during tests
or future code) has ample headroom beyond the default 1 MB.

#### Scenario: Linked stack reserve is raised

- **WHEN** the desktop binary is built
- **THEN** its configured main-thread stack reserve is substantially larger than
  the 1 MB default
