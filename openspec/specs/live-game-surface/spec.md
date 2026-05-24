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

`GameEvent` and every type it contains SHALL derive `serde::Serialize` with `#[serde(tag = "type")]` so that `events: [GameEvent]` in `EventLogView` and `events_emitted: Vec<GameEvent>` in `ActionResult` are emitted as structured JSON objects (NOT `Debug`-formatted strings). The `type` field SHALL match the variant name as returned by `GameEvent::type_str()` (e.g., `"MemoryChange"`, `"Play"`, `"GameOver"`). Variant-specific fields SHALL appear as siblings of `type` at the top level (no `meta` wrapper). A new `EffectFizzled` variant SHALL be added with fields `seq`, `source_permanent: Option<PermanentHandle>`, and `reason: String` to support the mandatory-selection fizzle path above.

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

#### Scenario: Event log is structured

- **WHEN** a caller requests `LiveGame::events(since_seq)` after actions that emitted events
- **THEN** each entry in `events` is a JSON object with at least a `type` field naming the variant (matching `GameEvent::type_str()`, e.g., `"MemoryChange"`, `"Play"`) and per-variant fields at the top level (e.g., `MemoryChange` exposes `seq`, `player`, `delta`, `total`; `Play` exposes `seq`, `player`, `card_id`, `field_index`) — entries are NOT `Debug`-formatted strings and there is no `meta` sub-object

---

### Requirement: Action Submission Surface

The system SHALL expose action-submission methods on `LiveGame` that mirror existing `Game` operations and emit results suitable for tool consumers.

The methods SHALL be:

- `play(player, hand_idx) -> ActionResult`
- `digivolve(host, source_hand_idx, paid_costs?) -> ActionResult`
- `attack(attacker, target) -> ActionResult`
- `resolve_selection(player, action_id) -> ActionResult`
- `end_turn() -> ActionResult`
- `pass_turn() -> ActionResult`
- `move_from_breeding(player) -> ActionResult`
- `step(action_id) -> ActionResult` — low-level escape hatch, accepts a raw action ID from the RL action space.

Every method SHALL return an `ActionResult` carrying:

- `ok: bool` — whether the action was accepted
- `error: Option<String>` — engine-level rejection reason if `ok == false`
- `events_emitted: Vec<GameEvent>` — events generated by this action, serialized as **structured JSON objects** with stable field names (NOT `Debug`-format strings)
- `new_phase: GamePhase` — phase after the action resolved
- `pending_selection_after: Option<PendingSelectionView>` — selection waiting for input, if any

Action methods SHALL NOT panic on illegal actions; they SHALL return `ok: false` with an `error` string describing the rejection reason. The engine state SHALL be unchanged when `ok == false`.

Illegal actions include but are not limited to:
- Out-of-bounds hand indices
- Actions submitted by a player who is not `current_decision_player()`
- Actions whose required phase precondition does not hold (e.g., `play` outside Main, `end_turn` during Mulligan)
- `step(action_id)` where `action_id` is not legal for `current_decision_player()` in the current state

The `LiveGame::step` semantic explicitly diverges from `HeadlessRunner::step` (which is fire-and-forget for the RL pipeline) — `LiveGame::step` SHALL validate and surface rejections so debug/MCP callers can detect failed actions.

#### Scenario: Successful play emits events

- **WHEN** a caller invokes `LiveGame::play(0, 2)` during P0's Main phase and hand index 2 is legal to play
- **THEN** the result has `ok == true`, `events_emitted` is non-empty and contains structured event objects with field-level access (not `Debug` strings), and any pending selection triggered by the play surfaces in `pending_selection_after`

#### Scenario: Illegal play — out of bounds returns error

- **WHEN** a caller invokes `LiveGame::play(0, 99)` where hand index 99 is out of bounds
- **THEN** the result has `ok == false`, `error` names the index and the actual hand size, and game state is unchanged

#### Scenario: Illegal play — wrong decision player returns error

- **WHEN** a caller invokes `LiveGame::play(0, 0)` while `current_decision_player() == 1` (e.g., during P1's Main phase)
- **THEN** the result has `ok == false`, `error` indicates the decision-player mismatch, and game state is unchanged (no card lands, no events, no phase advance)

#### Scenario: Illegal play — wrong phase returns error

- **WHEN** a caller invokes `LiveGame::play(0, 0)` during the Mulligan phase
- **THEN** the result has `ok == false`, `error` indicates the phase mismatch, and game state is unchanged

#### Scenario: Illegal step returns error

- **WHEN** a caller invokes `LiveGame::step(action_id)` where `action_id` is not in the legal-action set for `current_decision_player()` in the current phase
- **THEN** the result has `ok == false`, `error` names the action_id and the current decision player / phase, and game state is unchanged (`event_seq`, `current_phase`, and `pending_selection` are all identical to pre-call values)

#### Scenario: Illegal end_turn returns error and preserves state

- **WHEN** a caller invokes `LiveGame::end_turn()` during the Mulligan phase or any other phase where turn-end is not engine-legal
- **THEN** the result has `ok == false`, `error` indicates the phase mismatch, and `turn_count` / `current_phase` / `pending_selection` are unchanged

#### Scenario: Illegal pass_turn returns error

- **WHEN** a caller invokes `LiveGame::pass_turn()` during a phase where PASS is not legal
- **THEN** the result has `ok == false`, `error` describes the rejection, and game state is unchanged

#### Scenario: Selection resolution clears pending state

- **WHEN** a `PendingSelection` is active and the caller invokes `LiveGame::resolve_selection(player, action_id)` with a valid choice
- **THEN** the result is `ok == true`, the previously-pending selection is gone, and any consequent selection appears in `pending_selection_after`

#### Scenario: Selection resolution rejects when no pending

- **WHEN** no selection is active and the caller invokes `LiveGame::resolve_selection(player, action_id)`
- **THEN** the result has `ok == false` and `error` is `"no pending selection"`

---

### Requirement: Legal Actions Enumeration

The system SHALL expose `LiveGame::legal_actions(player) -> Vec<DecodedAction>` returning every action ID currently legal for the specified player, each decoded into a human-readable label.

`DecodedAction` SHALL contain:

- `action_id: u16` — the raw integer the engine consumes via `step()`
- `kind: ActionKind` — enum tag (`Play`, `Digivolve`, `Attack`, `EndTurn`, `PassTurn`, `ResolveSelection`, `MulliganKeep`, `MulliganRedraw`, etc.)
- `label: String` — human-readable description (e.g., `"play hand[2]: Agumon"`, `"digivolve field[0] from hand[3]: Greymon"`)
- `payload: serde_json::Value` — kind-specific structured detail

The decoder SHALL use the existing `build_action_mask` infrastructure to determine legality; it MUST NOT re-implement legality logic.

`legal_actions(player)` SHALL return an empty `Vec` when `player != current_decision_player()`. Returned actions SHALL be executable by the caller via `step(action_id)` at the moment of the call — callers SHALL NOT receive phantom actions for inactive players or for phases where the player cannot act.

#### Scenario: Enumerate legal plays

- **WHEN** a caller invokes `LiveGame::legal_actions(0)` during player 0's main phase
- **THEN** every action whose bit is set in `build_action_mask(0)` appears in the returned vector with a corresponding `DecodedAction` entry

#### Scenario: Labels include card names

- **WHEN** the legal actions list contains a play of a card whose `name` is `"Agumon"`
- **THEN** the corresponding `DecodedAction.label` includes `"Agumon"`

#### Scenario: Pending-selection legal actions

- **WHEN** a `PendingSelection` is active and `legal_actions` is called for the selecting player
- **THEN** every legal `ResolveSelection` action appears in the list, and unrelated actions (plays, attacks, etc.) do not

#### Scenario: Inactive player returns empty

- **WHEN** a caller invokes `LiveGame::legal_actions(player)` and `player != current_decision_player()`
- **THEN** the returned `Vec` is empty, regardless of what actions the player would have if it were their turn

#### Scenario: Returned actions are step-executable

- **WHEN** a caller takes any `DecodedAction` returned by `legal_actions(player)` and immediately invokes `LiveGame::step(action.action_id)`
- **THEN** the step succeeds (`ok == true`) and produces at least one event or a phase / selection change (no silent no-ops)

---

### Requirement: Mandatory pending selections must always be advanceable via fizzle

A pending selection SHALL NOT surface options whose `step` invocation is a no-op (no events emitted, no state change). When a mandatory pending selection's only legal options all result in no-ops, the engine SHALL fizzle the selection automatically: clear `pending_selection`, emit a `GameEvent::EffectFizzled` carrying the source permanent handle and reason (e.g., `"no valid target"`), and continue normal phase / turn progression.

The engine already fizzles at install time when the target-filter passes zero entities ([selections.rs:1626-1657](code/digimon-engine/src/effect_context/selections.rs:1626)). This requirement extends that policy to the execution path: if the install-time filter is more permissive than the resulting action's actual legality, the wrapper SHALL detect the no-op and fizzle as a safety net.

#### Scenario: Mandatory selection with no fulfillable target fizzles at install

- **WHEN** a triggered effect attempts to install a mandatory pending selection whose target predicate matches zero entities (e.g., "Select 1 of your [Omnimon]-named Digimon to attack" while the player has no Omnimon-named Digimon in the battle area)
- **THEN** the engine does not install the pending selection; instead it emits a `GameEvent::EffectFizzled` event identifying the source and the empty-target reason; the effect's callback is not invoked; `current_phase` and turn progression continue

#### Scenario: Mandatory selection with unfulfillable option fizzles at execute

- **WHEN** a mandatory pending selection is installed with one or more legal options, the caller invokes `LiveGame::step(option.action_id)` for the only remaining option, and the step results in no events and no state change (the option passed the install-time filter but the resulting callback was itself unfulfillable)
- **THEN** the engine clears `pending_selection`, emits a `GameEvent::EffectFizzled { source_permanent, reason: "no executable target" }`, returns `ok: true` from `step` (with the fizzle event in `events_emitted`), and continues normal phase / turn progression — the caller is never left stuck offering a no-op option

#### Scenario: Mandatory selection with multiple options does not fizzle on a single no-op step

- **WHEN** a mandatory pending selection has more than one legal option AND `step` on one option is a no-op AND other options exist in `legal_actions`
- **THEN** the engine does NOT fizzle; it returns `ok: false, error: "action <id> not legal in current state"` (per the general step-validation rule above), leaving the pending selection intact so the caller can try another option
