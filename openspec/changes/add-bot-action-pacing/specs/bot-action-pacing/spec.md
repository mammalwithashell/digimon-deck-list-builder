# bot-action-pacing

## ADDED Requirements

### Requirement: Paced agent stepping on both wires
Both human-facing game wires (desktop Tauri commands and browser HTTP routes) SHALL support a paced mode in which at most one agent action is executed per request, with the response carrying that action's trace, its events, the post-action state, and an `agentPending` flag indicating whether further agent actions remain before a human decision point.

#### Scenario: Desktop paced step
- **WHEN** a desktop game is created with a greedy seat in paced mode and the agent's turn contains multiple actions
- **THEN** each agent-advance invoke returns exactly one new agent trace and its events, with `agentPending: true` until the final action of the sequence, which returns `agentPending: false`

#### Scenario: Browser paced step
- **WHEN** a browser game in paced mode reaches an agent decision sequence
- **THEN** the `/games/{id}/agent-step` route advances exactly one agent action per call with the same `agentPending` semantics as the desktop wire

#### Scenario: Unpaced default preserved
- **WHEN** a game is created or stepped without the paced option
- **THEN** agent turns run to the next human decision point in a single request, byte-compatible with the pre-change response contract

### Requirement: Frontend pacing driver
The frontend SHALL, while `agentPending` is true and the bot speed setting is not Instant, automatically request the next agent step after the configured inter-action delay, rendering each action's trace, log entries, and animations individually, and locking human action submission for the duration.

#### Scenario: Bot turn rendered as a sequence
- **WHEN** the bot plays a card, digivolves, and attacks in one turn at Normal speed
- **THEN** the player sees three distinct beats — each with its own state change, log/ticker entry, and any animation — separated by the configured delay, with an "opponent is acting" indicator shown throughout

#### Scenario: Pacing failure recovery
- **WHEN** a paced agent-step request fails
- **THEN** the driver retries once and, on repeated failure, surfaces an error with a manual continue affordance instead of leaving the game silently stuck

### Requirement: Configurable, persisted bot speed
The UI SHALL expose a bot speed setting (Slow / Normal / Fast / Instant) persisted across sessions; Instant SHALL use the unpaced run-to-completion mode, and speed changes SHALL take effect from the next agent step without restarting the game.

#### Scenario: Speed persists
- **WHEN** the player selects Slow and relaunches the app
- **THEN** the next bot game paces agent actions at the Slow delay

#### Scenario: Instant reproduces legacy behavior
- **WHEN** the player selects Instant
- **THEN** bot turns resolve in a single request and render exactly as before this change

### Requirement: RL and training paths unaffected
Pacing SHALL exist only in the human-facing request loop; `HeadlessRunner`, the gym environment, training, and evaluation flows SHALL run unpaced with no added latency or contract changes.

#### Scenario: Training throughput unchanged
- **WHEN** a training or anchored-evaluation run executes
- **THEN** no paced code path is entered and per-step latency is unchanged
