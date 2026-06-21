## ADDED Requirements

### Requirement: Opponent shown under a Digimon alias in all game types

The system SHALL present the player opposing the local viewer under a display name drawn
from a shared roster of Digimon-franchise character names, for every in-game type — bot
games, human-vs-human (PvP), and vs-AI-online. The alias MUST replace the prior placeholders
(`GREEDY BOT`, `AI`, `Opponent`) and any real account name in the in-game display. The
selection is purely cosmetic and MUST NOT alter any gameplay, engine, or RL behavior.

#### Scenario: Opponent label is a roster member

- **WHEN** any game is rendered for a seated player
- **THEN** the opponent seat's display label is one of the names in the shared roster
- **AND** it is not `GREEDY BOT`, `AI`, `Opponent`, `Agent`, `Player 1`, or `Player 2`

#### Scenario: Applies to PvP as well as bot games

- **WHEN** a human-vs-human game is rendered
- **THEN** the opposing human's seat is shown under a roster alias, not their real account name

#### Scenario: Themed name flows to all in-game label consumers

- **WHEN** an opponent alias has been chosen for a game
- **THEN** the same alias is shown by every consumer of the player labels (board name tag, action log, and result overlay)

### Requirement: Alias is stable within a game and varies across games

The system SHALL keep an opponent's alias constant for the entire duration of a single game —
it MUST NOT change on state updates, reloads, or reconnects — while different games produce
independently varied aliases. To achieve this the alias MUST be derived deterministically from
the game's identifier rather than re-rolled on each render.

#### Scenario: Alias does not change on state updates

- **WHEN** the game receives successive state updates (e.g. WebSocket ticks) within one game
- **THEN** the opponent's displayed alias remains the same across all of those updates

#### Scenario: Alias survives reload

- **WHEN** the same game is reloaded or reconnected
- **THEN** the opponent's alias resolves to the same name as before

#### Scenario: Different games vary

- **WHEN** two different games (distinct game identifiers) are rendered
- **THEN** their opponent aliases are chosen independently and are not required to match

### Requirement: Local player's own seat is preserved; spectators see both seats aliased

The system SHALL leave the local player's own seat label unchanged (`YOU` / `You`) and alias
only the opposing seat. When there is no local seat (spectator or replay viewing), the system
SHALL alias BOTH seats with roster names rather than leaving the default `Player 1` /
`Player 2` labels.

#### Scenario: Seated player keeps their own label

- **WHEN** a seated player views their game
- **THEN** their own seat shows `YOU` / `You`
- **AND** only the opposing seat is aliased

#### Scenario: Spectator sees two aliases

- **WHEN** a spectator or replay viewer (no local seat) views a game
- **THEN** both seats are shown under roster aliases
- **AND** neither seat shows `Player 1` or `Player 2`

### Requirement: Display-only, not identity confidentiality

The system's aliasing SHALL be a presentation-layer concern only. It MUST NOT be relied upon
as a guarantee that the opponent's real account identity is withheld — the real `display_name`
may still be transmitted on the wire and shown on pre-game lobby/matchmaking surfaces, which
are out of scope for this change.

#### Scenario: Gameplay and engine state unaffected

- **WHEN** aliasing is applied
- **THEN** no engine state, action mask, tensor, or game outcome differs from the un-aliased behavior

#### Scenario: Pre-game identity surfaces are unchanged

- **WHEN** a player is on a pre-game lobby or matchmaking screen
- **THEN** this change does not modify what those screens display (they remain as-is, outside this capability)
