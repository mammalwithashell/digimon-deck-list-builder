## ADDED Requirements

### Requirement: Live-game capture to fixture

The engine SHALL provide a `Game::to_scenario()` operation that serializes the current full-information game state into the existing declarative scenario-fixture schema: schema-version, per-player decks, per-player zone staging (hand, deck order, field stacks with suspended/turn-played, breeding, security, trash), and initial scalar state (memory, phase, turn, first player). The emitted fixture MUST carry an empty assertion list — capture records state, not expectations. Zone ordering in the output MUST use the same conventions as the staging importer (deck index 0 = next draw = engine-vec tail; security and deck emitted bottom-first) so the two are exact inverses.

#### Scenario: Capture round-trips through staging

- **WHEN** `to_scenario()` is called on a game and the resulting fixture is applied to a fresh debug game via the `/debug` staging surface
- **THEN** the staged game's full-information state (all zones and scalar state) equals the captured game's state exactly, before any assertions run

#### Scenario: Captured fixture is schema-valid

- **WHEN** a game is captured
- **THEN** the emitted fixture conforms to the documented scenario-fixture schema (validates against it) and is loadable by both the Rust headless runner and the Playwright fixture without modification

### Requirement: Capture exposed on both engine wires

The capture operation SHALL be reachable from both serialization paths so that any game — live or staged, browser or desktop — can be captured through one engine implementation. It MUST be exposed on the PyO3 bindings (`RustHeadlessGame` and `RustDebugGame`) and over HTTP via `POST /games/{id}/export-scenario` on the engine-only games router. The HTTP export route MUST NOT introduce a database or auth dependency.

#### Scenario: Live browser game is capturable

- **WHEN** a normal (non-debug) browser-dev game reaches an interesting board and `POST /games/{id}/export-scenario` is called
- **THEN** the response body is a valid scenario fixture that, when re-staged, reproduces that board

#### Scenario: Single capture implementation

- **WHEN** the same board is captured via the PyO3 binding and via the HTTP route
- **THEN** both produce an identical fixture, because both delegate to the one `Game::to_scenario()` implementation
