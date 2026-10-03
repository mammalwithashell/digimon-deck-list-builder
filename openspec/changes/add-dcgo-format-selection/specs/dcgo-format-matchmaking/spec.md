## ADDED Requirements

### Requirement: Per-format random-match queues

Random match SHALL segregate players by the format chosen for the match, such that a client queuing in one format is never paired with a client queuing in a different format. Each format SHALL have its own room key, and the key for legacy formats SHALL remain exactly as it is today.

#### Scenario: Singleton players match each other

- **WHEN** two clients both queue for random match in `singleton`
- **THEN** they are paired with each other

#### Scenario: EDEN players match each other

- **WHEN** two clients both queue for random match in `eden`
- **THEN** they are paired with each other

#### Scenario: Singleton does not match Standard

- **WHEN** one client queues for random match in `singleton` and another queues in `standard`
- **THEN** neither client joins the other's room, and both continue waiting

#### Scenario: Two new formats do not match each other

- **WHEN** one client queues for random match in `singleton` and another queues in `eden`
- **THEN** neither client joins the other's room, and both continue waiting

#### Scenario: Legacy queue key is unchanged

- **WHEN** a client queues for random match in `standard` or `no_restriction`
- **THEN** the room key used is the one the client used before this change

#### Scenario: Existing mixed pool is preserved

- **WHEN** two clients queue for random match in `standard` and `no_restriction` respectively
- **THEN** they may still be paired, exactly as they are today

### Requirement: Per-format room-match handshake

Room match SHALL encode the chosen format in the room name so that creating and joining a room agree on the format without a separate exchange. The encodings for legacy formats SHALL be byte-identical to the current ones.

#### Scenario: Legacy room names are unchanged

- **WHEN** a room is created in `standard` or `no_restriction`
- **THEN** the room name is exactly the string the client would have produced before this change for the corresponding `useBanlist` value

#### Scenario: New format uses a distinct room name

- **WHEN** a room is created in `singleton` or `eden`
- **THEN** the room name carries a token distinct from both legacy encodings and from every other format's token

#### Scenario: Joining requires a matching format

- **WHEN** a client attempts to join a room code while its chosen format differs from the format the room was created in
- **THEN** the join does not succeed and the player is told the room was not found or the format does not match

#### Scenario: Room code is displayed without the format token

- **WHEN** a player views the room code to share it
- **THEN** the displayed code is the shareable identifier, not the internal format-qualified room name

### Requirement: Compatibility with clients that lack format support

Clients without this change SHALL be able to create, discover, and join legacy-format rooms exactly as they do today, and SHALL NOT be able to discover or join rooms in formats they do not implement.

#### Scenario: Unmodded client keeps matching

- **WHEN** an unmodded client uses random match
- **THEN** it discovers and joins the same rooms it would have discovered before this change

#### Scenario: Unmodded client cannot see a new-format room

- **WHEN** an unmodded client scans the room list while a `singleton` random-match room exists
- **THEN** that room does not satisfy the unmodded client's matching criteria and is not joined

#### Scenario: Updated client can still meet an unmodded client

- **WHEN** an updated client queues in `standard` and an unmodded client queues at the same time
- **THEN** the two may be paired

### Requirement: Format keys are compared exactly

The client SHALL determine a room's format by exact comparison of the format token, and SHALL NOT rely on substring containment that could match one format key inside another.

#### Scenario: A longer format key does not satisfy a shorter one

- **WHEN** the room list contains a room whose format token has another defined format id as a prefix or substring
- **THEN** a client queuing for the shorter-named format does not treat that room as a match

#### Scenario: Malformed room name is ignored

- **WHEN** a room name does not carry a parseable format token
- **THEN** the client does not join it as part of a new-format queue
