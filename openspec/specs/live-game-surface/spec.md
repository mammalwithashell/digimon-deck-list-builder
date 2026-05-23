### Requirement: LiveGame Construction From Four Sources

The system SHALL provide a `LiveGame` type in `digimon-engine` constructable from four distinct sources so that all debug, smoke-test, and forensic workflows share a single live-game surface.

The four constructors SHALL be:

- `LiveGame::from_decks(deck1, deck2, seed)` — fresh game with shuffled libraries and dealt hands. `seed` is optional; when `None`, the engine's default RNG seeding applies.
- `LiveGame::from_debug(hands, decks, first_player)` — fresh game with deterministic ordering: `hands[player_id]` is the exact opening hand (no shuffle), `decks[player_id]` is the post-shuffle library (index 0 = top), and `first_player` selects who acts first. Mirrors the existing `DebugRunnerBuilder` shape.
- `LiveGame::from_recording(recording_json)` — game reconstructed deterministically from a `GameRecorder` recording, paused at step 0 (initial post-shuffle state, no actions applied).
- `LiveGame::from_recording_at_step(recording_json, step_n)` — same as `from_recording` but with the first `step_n` recorded actions already replayed.

Each constructor SHALL accept a card pool (set of `card_id` strings whose `CardData` is loaded into the game). When the pool is omitted, the default SHALL be the result of `digimon_engine::cards::build_registry().registered_card_ids()` — the same filter `pilot_training`, `gauntlet`, and the architect agents use.

A constructor SHALL return `Err(...)` when any required card is not in the pool. The error SHALL identify the missing card IDs so callers can suggest `--all-cards` or remediate.

#### Scenario: Construct from decks with default pool

- **WHEN** a caller invokes `LiveGame::from_decks(deck1, deck2, None)` with two 50-card decks whose every card is in `load_implemented_card_ids()`
- **THEN** the constructor returns `Ok(LiveGame)` and the resulting game is in the mulligan phase with hands dealt

#### Scenario: Construct from debug hands

- **WHEN** a caller invokes `LiveGame::from_debug({0: ["BT1-001"], 1: []}, {0: ["FILLER"; 50], 1: ["FILLER"; 50]}, 0)`
- **THEN** the resulting game has `BT1-001` as `hands[0][0]`, no shuffling has occurred, and player 0 acts first

#### Scenario: Construct from recording at step 0

- **WHEN** a caller invokes `LiveGame::from_recording(recording_json)` with a valid recording
- **THEN** the resulting game's state matches the recording's `initial_state` exactly, with `step_number == 0` and no actions applied

#### Scenario: Construct from recording at intermediate step

- **WHEN** a caller invokes `LiveGame::from_recording_at_step(recording_json, 47)` with a recording containing at least 47 actions
- **THEN** the resulting game's state matches what would be observed after replaying actions 1..=47 against the initial state

#### Scenario: Missing card in default pool

- **WHEN** a caller invokes `LiveGame::from_decks(deck1, deck2, None)` and `deck1` contains a card not present in `load_implemented_card_ids()`
- **THEN** the constructor returns `Err` and the error message names every missing card ID

#### Scenario: Custom pool override

- **WHEN** a caller passes a custom pool that omits an otherwise-implemented card
- **THEN** any deck referencing the omitted card fails construction with the missing-card error

---

### Requirement: View Serialization Layer

The system SHALL expose a `view` module providing compact, stable JSON-serializable views over `Game` state. These views are distinct from the frontend-oriented `to_ui_json` (which is lossy and player-perspective-specific) and exist specifically for tool consumers (CLI, MCP) that need precise state for debugging.

The following views SHALL exist:

- `StateView { phase, turn_count, memory, turn_player, game_over, winner, ... }`
- `HandView { player, cards: [{card_id, card_index, name}] }`
- `FieldView { player, permanents: [{handle, top_card_id, stack_card_ids, modifiers, summoning_sick, ...}] }`
- `SecurityView { player, count, card_ids_if_god_view }`
- `PendingSelectionView { kind, source, min, max, options: [{label, payload}], cancellable }`
- `EffectQueueView { pending: [{source, trigger, kind}] }`
- `ModifierView { handle, modifiers: [{type, source, value, expiry}] }`
- `EventLogView { events: [GameEvent], since_seq }`

Every view SHALL be serializable to JSON via `serde`. Field names SHALL be stable; renaming any field is a breaking change to consumers and SHALL be reflected in a spec delta.

A view SHALL accept a `perspective` parameter: `Perspective::Player(PlayerId)` filters opponent-hidden information (opponent hand IDs, opponent security IDs, opponent decklist order); `Perspective::God` exposes everything.

#### Scenario: Player perspective hides opponent hand

- **WHEN** a caller requests `LiveGame::hand(opponent, Perspective::Player(me))`
- **THEN** the returned `HandView` contains card *count* but **not** `card_id` or `name` values

#### Scenario: God perspective exposes everything

- **WHEN** a caller requests `LiveGame::security(opponent, Perspective::God)`
- **THEN** the returned `SecurityView` includes the full `card_ids` array (bottom-up ordering)

#### Scenario: Pending selection view enumerates options

- **WHEN** a `PendingSelection` is active on a `LiveGame`
- **THEN** `LiveGame::pending_selection()` returns a `PendingSelectionView` with one entry per legal option, each entry's `label` is a human-readable string, and `payload` carries the integer index the engine expects for `resolve_selection`

#### Scenario: Effect queue view preserves order

- **WHEN** multiple triggered effects are queued
- **THEN** `LiveGame::effect_queue()` returns them in the same order the engine will resolve them, with no items dropped or reordered

---

### Requirement: Action Submission Surface

The system SHALL expose action-submission methods on `LiveGame` that mirror existing `Game` operations and emit results suitable for tool consumers.

The methods SHALL be:

- `play(player, hand_idx) -> ActionResult`
- `digivolve(host, source_hand_idx, paid_costs?) -> ActionResult`
- `attack(attacker, target) -> ActionResult`
- `resolve_selection(choice_indices) -> ActionResult`
- `end_turn() -> ActionResult`
- `pass_turn() -> ActionResult`
- `step(action_id) -> ActionResult` — low-level escape hatch, accepts a raw action ID from the RL action space.

Every method SHALL return an `ActionResult` carrying:

- `ok: bool` — whether the action was accepted
- `error: Option<String>` — engine-level rejection reason if `ok == false`
- `events_emitted: Vec<GameEvent>` — events generated by this action
- `new_phase: GamePhase` — phase after the action resolved
- `pending_selection_after: Option<PendingSelectionView>` — selection waiting for input, if any

Action methods SHALL NOT panic on illegal actions; they SHALL return `ok: false` with an error.

#### Scenario: Successful play emits events

- **WHEN** a caller invokes `LiveGame::play(0, 2)` and hand index 2 is legal to play
- **THEN** the result has `ok == true`, `events_emitted` is non-empty, and any pending selection triggered by the play surfaces in `pending_selection_after`

#### Scenario: Illegal play returns error

- **WHEN** a caller invokes `LiveGame::play(0, 99)` where hand index 99 is out of bounds
- **THEN** the result has `ok == false` and `error` carries a descriptive message; game state is unchanged

#### Scenario: Selection resolution clears pending state

- **WHEN** a `PendingSelection` is active and the caller invokes `LiveGame::resolve_selection([0])`
- **THEN** the result is `ok == true`, the previously-pending selection is gone, and any consequent selection appears in `pending_selection_after`

---

### Requirement: Legal Actions Enumeration

The system SHALL expose `LiveGame::legal_actions(player) -> Vec<DecodedAction>` returning every action ID currently legal for the specified player, each decoded into a human-readable label.

`DecodedAction` SHALL contain:

- `action_id: u16` — the raw integer the engine consumes via `step()`
- `kind: ActionKind` — enum tag (`Play`, `Digivolve`, `Attack`, `EndTurn`, `PassTurn`, `ResolveSelection`, `MulliganKeep`, `MulliganRedraw`, etc.)
- `label: String` — human-readable description (e.g., `"play hand[2]: Agumon"`, `"digivolve field[0] from hand[3]: Greymon"`)
- `payload: serde_json::Value` — kind-specific structured detail

The decoder SHALL use the existing `build_action_mask` infrastructure to determine legality; it MUST NOT re-implement legality logic.

#### Scenario: Enumerate legal plays

- **WHEN** a caller invokes `LiveGame::legal_actions(0)` during player 0's main phase
- **THEN** every action whose bit is set in `build_action_mask(0)` appears in the returned vector with a corresponding `DecodedAction` entry

#### Scenario: Labels include card names

- **WHEN** the legal actions list contains a play of a card whose `name` is `"Agumon"`
- **THEN** the corresponding `DecodedAction.label` includes `"Agumon"`

#### Scenario: Pending-selection legal actions

- **WHEN** a `PendingSelection` is active and `legal_actions` is called
- **THEN** every legal `ResolveSelection` action appears in the list, and unrelated actions (plays, attacks, etc.) do not
