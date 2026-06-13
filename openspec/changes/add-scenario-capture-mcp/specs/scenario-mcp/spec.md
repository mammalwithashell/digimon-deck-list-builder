## ADDED Requirements

### Requirement: Scenario MCP tool surface

The project SHALL provide a `digimon-scenario-mcp` MCP server exposing tools for the full stage→drive→capture→assert loop: stage a scenario (from a fixture file or inline zones), read full-information state, get the action mask, get the pending selection, step by an action, capture the current game to a fixture, list/load/save fixtures under `qa/scenarios/`, add assertions to a fixture, evaluate a fixture's assertions against current state, and emit a Playwright spec from a fixture. Each tool's result MUST be a normalized structure independent of which transport served it.

#### Scenario: Stage without playing

- **WHEN** the stage tool is called with inline zones (e.g. a named hand card over a specific stack at a given memory)
- **THEN** a game is created in that exact state and its rendered/internal state is returned, with no game having been hand-played to reach it

#### Scenario: Capture an in-progress board

- **WHEN** the capture tool is called against an in-progress game
- **THEN** a valid scenario fixture for that board is returned and can be saved under `qa/scenarios/`

### Requirement: Browser and desktop transports

State-touching MCP tools SHALL accept a `target` selector of `browser` or `desktop`. The `browser` target routes to the FastAPI `/debug` and `/games` surface (multi-game, addressed by game id); the `desktop` target routes to the Tauri localhost bridge (single implicit game, no id). The MCP MUST normalize the two response shapes so callers see one schema regardless of target.

#### Scenario: Same fixture stages on either target

- **WHEN** the same fixture is staged with `target: browser` and with `target: desktop`
- **THEN** both produce equal full-information state, and the MCP returns the same normalized shape for each

#### Scenario: Desktop target drives the real app

- **WHEN** a tool is called with `target: desktop` while the bridge-enabled desktop app is running
- **THEN** the actual desktop game is staged/inspected/stepped, exercising the desktop `engine_commands.rs` DTO wire that browser-mode cannot reach

### Requirement: Write-capable dev-only MCP, documented as an exception

The scenario MCP is write-capable (it mutates game state and writes files), unlike the read-only operator MCPs. This departure SHALL be explicitly documented in CLAUDE.md. The MCP MUST remain dev/test-only: it MUST NOT be importable from `server.*` or `digimon_gym.*`, MUST NOT be present in any production build or deployment, and MUST talk only to dev-gated surfaces (the `/debug` router and the gated bridge).

#### Scenario: MCP excluded from production

- **WHEN** the hosted API or desktop production bundle is built
- **THEN** the scenario MCP is not present and nothing in the shipped code imports it

#### Scenario: Convention exception is recorded

- **WHEN** a developer reads the CLAUDE.md "Read-only operator MCPs" guidance
- **THEN** the write-capable scenario MCP is listed as a documented, justified exception rather than an undocumented violation
