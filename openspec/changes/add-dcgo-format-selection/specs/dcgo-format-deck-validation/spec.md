## ADDED Requirements

### Requirement: Both decks are validated against the agreed format

Before a match in a given format begins, each client SHALL validate both its own deck and its opponent's deck against that format, using the opponent's published deck data rather than any self-declared claim of legality.

#### Scenario: Opponent deck is read from published deck data

- **WHEN** a client needs to validate its opponent's deck
- **THEN** it reads the opponent's deck from the deck data the opponent has already published to the room, without requesting anything additional from the opponent

#### Scenario: Both clients reach the same verdict

- **WHEN** two clients in the same room validate the same pair of decks against the same format
- **THEN** both reach the same legal-or-illegal verdict for each deck

#### Scenario: A self-declared claim is not trusted

- **WHEN** an opponent's client reports its deck as legal but the published deck data is illegal under the agreed format
- **THEN** the validating client treats the deck as illegal

### Requirement: Validation blocks the match in room match

In room match, a client SHALL NOT signal readiness while either its own deck or its opponent's deck is illegal under the room's format, and the match SHALL NOT start until both decks are legal.

#### Scenario: Own illegal deck blocks readiness

- **WHEN** the player attempts to signal readiness with a deck that is illegal under the room format
- **THEN** readiness is refused and the player is shown why the deck is illegal

#### Scenario: Opponent illegal deck blocks the start

- **WHEN** both players have signalled readiness by count but the opponent's published deck is illegal under the room format
- **THEN** the match does not start and the blocking reason is surfaced

#### Scenario: Deck change re-validates

- **WHEN** a player changes their selected deck while in a room
- **THEN** validation runs again and readiness state reflects the new deck

### Requirement: Validation blocks the match in random match

In random match, a client SHALL validate the opponent's deck after being paired and before the battle begins, and SHALL leave the room and resume queuing if the opponent's deck is illegal under the queued format.

#### Scenario: Illegal opponent causes requeue

- **WHEN** a client is paired in a format queue and the opponent's published deck is illegal under that format
- **THEN** the client leaves the room and resumes queuing in the same format

#### Scenario: Player is informed of the requeue

- **WHEN** a client requeues because an opponent failed validation
- **THEN** the player is told that the pairing was rejected for deck legality rather than being returned silently

#### Scenario: Requeue does not spin

- **WHEN** a client requeues after a failed validation
- **THEN** it does not immediately rejoin the same rejected room, and repeated failures do not produce an unbounded tight loop

#### Scenario: Legal pairing proceeds

- **WHEN** a client is paired in a format queue and both decks are legal under that format
- **THEN** the match proceeds without additional prompting

### Requirement: Validation fails closed

Validation SHALL treat any inability to establish legality as a failure rather than a pass.

#### Scenario: Unparseable deck data fails

- **WHEN** an opponent's published deck data cannot be parsed into a decklist
- **THEN** validation fails for that opponent

#### Scenario: Unknown card fails

- **WHEN** an opponent's decklist contains a card ID the validating client cannot resolve in its card database
- **THEN** validation fails for that opponent

#### Scenario: Missing deck data fails

- **WHEN** an opponent has not published deck data at the point validation runs
- **THEN** validation does not pass, and the match does not start on the basis of the missing data

### Requirement: Legacy formats retain current behaviour

Matches in `standard` and `no_restriction` SHALL behave exactly as they do today, so that no existing match can newly fail to start as a result of this change.

#### Scenario: Standard match is unaffected

- **WHEN** two clients match in `standard`
- **THEN** the match starts under the same conditions that applied before this change

#### Scenario: No new block against an unmodded opponent

- **WHEN** an updated client is paired with an unmodded client in a legacy format
- **THEN** the updated client does not block the match on format-validation grounds
