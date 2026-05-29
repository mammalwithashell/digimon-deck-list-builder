## ADDED Requirements

### Requirement: Debug game construction with staged zones

The engine SHALL expose a debug-only game surface, `RustDebugGame`, that constructs a game with fully staged per-player zones and mid-game state without playing through the opening sequence. Construction MUST accept, per player: an ordered hand, an ordered deck (top-first, no shuffle), zero or more field permanents (each a bottom-to-top digivolution stack with suspended flag and turn-played value), an optional breeding stack, an ordered security stack (top-to-bottom), and a trash pile. Construction MUST also accept the initial memory value, the current phase, the first player, and an auto-mulligan disposition. `RustDebugGame` MUST present the same `step`, `get_action_mask`, `to_ui_json`, and event-draining surface as `RustHeadlessGame` so existing HTTP routes operate against it unchanged.

#### Scenario: Stage a mid-game board deterministically
- **WHEN** a `RustDebugGame` is constructed with player 1 holding a specified hand, a specified field stack of `Paildramon` over `ExVeemon` and `Stingmon`, memory at a specified value, and phase set to Main
- **THEN** `to_ui_json()` reflects exactly that board state with no shuffling, no drawn opening hand, and no mulligan prompt outstanding

#### Scenario: Staged game drives like a live game
- **WHEN** a staged `RustDebugGame` is stepped with a legal action id
- **THEN** the action resolves, the action mask and `to_ui_json()` update, and events drain identically to how `RustHeadlessGame` would handle the same action

### Requirement: Debug game state mutation

`RustDebugGame` SHALL allow direct mutation of an active staged game for incremental test setup: setting the memory gauge to an exact value, injecting a card into a named zone (hand, deck top, security top, trash) for a given player, placing a digivolution stack onto a player's field, placing a stack into breeding, and a bulk-setup operation that replaces multiple zones in one call. Each mutation MUST leave the game in a self-consistent state that subsequent `step` / `get_action_mask` calls operate on correctly.

#### Scenario: Set memory to an exact value
- **WHEN** `set_memory` is called with a target value on a staged game
- **THEN** `to_ui_json()` reports the memory gauge at that value from the acting player's perspective

#### Scenario: Inject a card into a zone
- **WHEN** `inject_card` is called for player 1 with a card id and zone "hand"
- **THEN** the card appears in player 1's hand and the action mask updates to include any actions that card newly enables

#### Scenario: Place a digivolution stack on the field
- **WHEN** `place_on_field` is called with a bottom-to-top card-id list, a suspended flag, and a turn-played value
- **THEN** the resulting permanent has that exact stack, suspend state, and summoning-sickness status

### Requirement: Debug game internal-state inspection

`RustDebugGame` SHALL expose an internal-state read that returns the full per-player zone contents (hand, field stacks, breeding, security, trash, deck counts) and scalar state (memory, phase, turn, current player) in a structured form, so a test can assert on any board fact after staging or after an action without parsing the UI-facing JSON.

#### Scenario: Read back staged zones
- **WHEN** internal-state is read after staging player 1 with a known hand and field
- **THEN** the returned structure lists player 1's hand card ids and each field permanent's stack contents in bottom-to-top order

### Requirement: Debug surface excluded from the desktop bundle

The debug staging surface SHALL exist only in the Python-binding/test path and MUST NOT be reachable from the Python-free desktop Tauri bundle. The desktop gameplay path MUST continue to use only the non-debug engine surface.

#### Scenario: Desktop bundle carries no debug staging
- **WHEN** the desktop Tauri application is built
- **THEN** it links no `RustDebugGame` surface and no debug staging command is registered in its `invoke` handlers
