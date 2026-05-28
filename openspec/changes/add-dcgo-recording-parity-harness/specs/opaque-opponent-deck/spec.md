## ADDED Requirements

### Requirement: Opaque-Opponent-Deck Game Constructor

The `digimon-engine` crate SHALL provide a `Game::new_with_opaque_opponent(my_deck_in_order, opp_decklist_unordered, ...)` constructor that initializes a two-player game in which one player's deck is opaque to the engine — the engine knows its composition (the unordered card-ID multiset) but not its order. Draws, security pops, mill effects, and any other engine path that would read a card from the opaque pile SHALL retrieve the next card from an externally-supplied reveal source rather than from a pre-shuffled list.

The constructor SHALL accept the calling player's deck as an ordered `Vec<CardId>` (treated identically to the standard `Game::new` deck parameter) and the opponent's deck as an unordered `Vec<CardId>` (treated as a multiset; the engine SHALL validate that its size equals the standard deck size and that all card IDs are known to the loaded card pool).

The constructor SHALL register a `RevealSource` callable or queue that the engine consults whenever it needs a card from the opaque pile. The `RevealSource` API SHALL distinguish among `Reveal::Draw`, `Reveal::Security`, `Reveal::Mill`, and `Reveal::Effect` sources so the supplier can verify alignment with a recording.

#### Scenario: Construction accepts well-formed inputs

- **WHEN** `Game::new_with_opaque_opponent` is called with a 50-card ordered deck for the calling player AND a 50-card unordered decklist for the opponent AND a `RevealSource` supplying reveals on demand
- **THEN** the constructor returns `Ok(Game)` AND the opponent's deck zone reports `size = 50` AND the engine accepts subsequent step calls normally

#### Scenario: Construction rejects mismatched deck sizes

- **WHEN** `Game::new_with_opaque_opponent` is called with an opponent decklist of size N where N differs from the rules' standard deck size
- **THEN** the constructor returns `Err` with an error message naming the expected size and the observed size

#### Scenario: Construction rejects unknown card IDs

- **WHEN** `Game::new_with_opaque_opponent` is called with an opponent decklist containing a card ID absent from the loaded card pool
- **THEN** the constructor returns `Err` with an error message naming the offending card ID

### Requirement: Engine Consumes Reveals From the External Source

When the engine would draw a card from the opaque pile, the engine SHALL request the next reveal from the `RevealSource` and SHALL use the returned card ID as the drawn card. The engine SHALL NOT inspect, shuffle, or reorder the opaque pile internally; it SHALL treat the pile only as a count and a multiset for legality purposes (e.g., "does the deck have any cards left to draw").

When the engine resolves a security-pop, mill, or effect-driven peek into the opaque pile, it SHALL request the next reveal with the appropriate `Reveal::*` source tag. The `RevealSource` MAY use the tag to validate alignment with an expected recording stream; the engine SHALL NOT assume any particular validation behavior on the supplier's side beyond receiving back a card ID.

After a reveal is consumed, the opaque pile's reported `size` SHALL decrement by one. The revealed card SHALL be removed from the multiset's available pool — re-revealing the same physical card position is not permitted; the supplier SHALL reject duplicate reveals beyond the multiset's count of that card.

#### Scenario: Draw from opaque deck consumes one reveal

- **WHEN** a game runs in opaque-opponent mode AND the engine would draw a card from the opaque pile
- **THEN** the engine requests one reveal with `Reveal::Draw` AND the returned card ID becomes the drawn card AND the opaque pile's `size` decreases by one

#### Scenario: Security pop from opaque deck uses the Security tag

- **WHEN** a game runs in opaque-opponent mode AND a security check pops a card from the opaque opponent's security stack (which was sourced from the opaque pile)
- **THEN** the engine requests one reveal with `Reveal::Security` source tag

### Requirement: Reveal Source Exhaustion Is a Recoverable Error

If the engine requests a reveal from the `RevealSource` AND the source has no more reveals to supply (e.g., a truncated recording), the engine SHALL surface a structured error indicating reveal exhaustion at the current step. The engine SHALL NOT panic, deadlock, or fabricate a card identity.

The error SHALL be distinguishable by callers from other game-state errors so the replay harness can report it as a recording-corruption failure rather than a parity failure.

#### Scenario: Empty reveal source mid-game halts cleanly

- **WHEN** a game runs in opaque-opponent mode AND the `RevealSource` is exhausted AND the engine would draw a card
- **THEN** the engine returns a `RevealExhaustedError` distinguishable from `GameStateError` AND no further state mutation occurs

### Requirement: Opaque Mode Composes With Standard Engine Capabilities

A game initialized via `Game::new_with_opaque_opponent` SHALL behave identically to a standard `Game::new` game with respect to: action masking, action decoding, effect resolution, modifier application, the effect queue, the selection state machine, BO1 win condition, concede semantics, recording (the native `GameRecorder` SHALL be capable of recording an opaque-mode game), and tensor observation generation.

The calling-player side of the game SHALL be fully transparent — the calling player's deck is ordered and the engine draws from it normally.

#### Scenario: Calling-player draws follow the supplied deck order

- **WHEN** a game runs in opaque-opponent mode AND the calling player draws a card on turn 1
- **THEN** the drawn card is the next card in the supplied `my_deck_in_order` list AND no `RevealSource` call is made for that draw

#### Scenario: Action mask is generated identically in opaque mode

- **WHEN** a game runs in opaque-opponent mode AND the action mask is queried for the calling player
- **THEN** the resulting mask is byte-identical to a standard-mode game with the same observable state
