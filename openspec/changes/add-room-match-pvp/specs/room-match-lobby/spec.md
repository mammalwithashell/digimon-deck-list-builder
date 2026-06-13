# room-match-lobby

## ADDED Requirements

### Requirement: Room creation with numeric code
The server SHALL let an authenticated user (including guests) create a private room via `POST /lobby/create` and SHALL return a unique 5-digit numeric join code (string, leading zeros preserved). Rooms SHALL expire after the lobby TTL if not started.

#### Scenario: Host creates a room
- **WHEN** an authenticated user calls `POST /lobby/create`
- **THEN** the response contains a `game_id` and a 5-digit numeric `join_code` unique among pending rooms

#### Scenario: Stale room expires
- **WHEN** a room remains unstarted past the lobby TTL
- **THEN** the room and its join code are pruned and subsequent joins return 404

### Requirement: Seat reservation on join
`POST /lobby/join/{code}` SHALL reserve the joiner seat without requiring a deck and without starting the game. The seat SHALL be exclusive (second join attempt by a different user returns 409), re-join by the seated user SHALL be idempotent, and the host SHALL NOT be able to join their own room.

#### Scenario: Guest joins with code only
- **WHEN** an authenticated user calls `POST /lobby/join/{code}` for a valid pending room with an empty seat
- **THEN** the seat is reserved for that user, no game is created, and the response identifies the room (`game_id`)

#### Scenario: Seat is exclusive
- **WHEN** a third user calls `POST /lobby/join/{code}` while the joiner seat is occupied
- **THEN** the server responds 409 and the room is unchanged

#### Scenario: Host cannot join own room
- **WHEN** the room's host calls `POST /lobby/join/{code}` with their own room's code
- **THEN** the server responds 400

### Requirement: Per-seat deck locking
`PUT /lobby/{id}/deck` SHALL accept a deck from the host or the seated joiner, lock it to the caller's seat, and allow replacing it any time before start. Room state SHALL report each seat's deck-readiness without exposing the opponent's deck contents.

#### Scenario: Joiner locks a deck
- **WHEN** the seated joiner submits a valid deck via `PUT /lobby/{id}/deck`
- **THEN** room state reports `joiner_deck_ready: true` and the host's polled state reflects it

#### Scenario: Non-participant cannot lock a deck
- **WHEN** a user who is neither host nor seated joiner calls `PUT /lobby/{id}/deck`
- **THEN** the server responds 403

### Requirement: Host selects the first player
The host SHALL be able to set the room's first-player choice to `1`, `random`, or `2` (default `random`) before start; the choice SHALL be visible in room state and SHALL determine which seat takes the first turn of the created game.

#### Scenario: Host picks player 2 first
- **WHEN** the host sets first player to `2` and starts the room
- **THEN** the created game's first turn belongs to the joiner's seat

#### Scenario: Joiner cannot set first player
- **WHEN** the seated joiner attempts to set the first-player choice
- **THEN** the server responds 403

### Requirement: Host-gated start
`POST /lobby/{id}/start` SHALL be host-only and SHALL create the game exactly once, only when both seats are occupied and both decks are locked; otherwise it SHALL respond 409. After start, room state SHALL report `started: true` and retain the seat mapping.

#### Scenario: Start with both decks locked
- **WHEN** both seats are occupied, both decks are locked, and the host calls `POST /lobby/{id}/start`
- **THEN** a game is created under the room's `game_id` and room state reports `started: true`

#### Scenario: Premature start is rejected
- **WHEN** the host calls `POST /lobby/{id}/start` before the joiner has locked a deck
- **THEN** the server responds 409 and no game is created

### Requirement: Seat-aware polled room state
`GET /lobby/{id}/state` SHALL require authentication and SHALL return the room's readiness picture — host and joiner display names, per-seat deck-readiness, first-player choice, `started` — plus `your_seat` (1, 2, or null) for the calling user, sufficient for both clients to drive the room UI and to navigate into the started game by polling alone.

#### Scenario: Host sees the joiner arrive
- **WHEN** the host polls `GET /lobby/{id}/state` after a guest reserves the joiner seat
- **THEN** the response includes the joiner's display name and `joiner_deck_ready: false`

#### Scenario: Both clients learn the game started
- **WHEN** either participant polls state after the host starts the room
- **THEN** the response reports `started: true` and that caller's `your_seat`, enabling navigation to the game as that player

### Requirement: Leave and cancel
The seated joiner SHALL be able to vacate their seat before start (clearing their deck), returning the room to the waiting state; the host SHALL be able to cancel a pending room, after which its code no longer resolves.

#### Scenario: Joiner leaves before start
- **WHEN** the seated joiner calls the leave endpoint
- **THEN** the seat and joiner deck are cleared and the host's polled state shows the room waiting for an opponent again

#### Scenario: Host cancels the room
- **WHEN** the host cancels a pending room
- **THEN** the room is removed and both the join code and `GET /lobby/{id}/state` return 404

### Requirement: Room flow UI for host and guest
The frontend SHALL provide a Create-or-Join choice in the room-match flow, a join page accepting a 5-digit code, and a seat-aware room screen that polls room state: both seats see the opponent's presence and deck-readiness; the host additionally sees a first-player selector and a start control enabled only when both decks are locked; on `started`, both clients SHALL automatically navigate into the game as their seat.

#### Scenario: Guest joins via code entry
- **WHEN** a user enters a valid 5-digit code on the join page
- **THEN** they land in the room screen as the joiner, can lock a deck, and see the host's readiness

#### Scenario: Game entry is automatic for both
- **WHEN** the host presses start while both clients are on the room screen
- **THEN** both clients navigate to the game view as player 1 and player 2 respectively, without a manual enter-game action
