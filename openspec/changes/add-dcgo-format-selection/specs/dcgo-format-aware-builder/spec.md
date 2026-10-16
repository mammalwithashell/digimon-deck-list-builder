## ADDED Requirements

### Requirement: Builder format selector

The deck builder SHALL expose a format selector that governs deck construction, occupying the settings position previously held by the banlist toggle. The selection SHALL persist across sessions and SHALL be locked while the client is in a room.

#### Scenario: Selecting a builder format takes effect immediately

- **WHEN** the player changes the builder format
- **THEN** subsequent deck-construction limits and card availability reflect the newly selected format without restarting the client

#### Scenario: Builder format persists

- **WHEN** the player sets the builder format and restarts the client
- **THEN** the builder format is the one they last selected

#### Scenario: Format is locked while in a room

- **WHEN** the client is in a match room
- **THEN** the builder format selector is not interactable, matching the existing behaviour of the banlist toggle

### Requirement: Copy limits follow the selected builder format

The per-card maximum-copies value used by the deck builder SHALL be derived from the selected builder format, bounded by the card's own printed maximum.

#### Scenario: Singleton caps every card at one

- **WHEN** the builder format is `singleton` and the player has one copy of a card in the deck
- **THEN** the builder reports that card as at its maximum and refuses to add a second copy

#### Scenario: Standard applies the official limits

- **WHEN** the builder format is `standard`
- **THEN** per-card limits are those of the official banlist, exactly as before this change

#### Scenario: No Banlist applies printed limits only

- **WHEN** the builder format is `no_restriction`
- **THEN** per-card limits are the cards' printed maximums with no banlist restriction applied

#### Scenario: Singleton caps eggs at one

- **WHEN** the builder format is `singleton` and the player has one copy of a Digi-Egg in the egg deck
- **THEN** the builder refuses to add a second copy of that Digi-Egg

### Requirement: Cards outside the format pool are indicated

The deck builder SHALL visually indicate cards that are not legal in the selected builder format, using the existing card-cover affordance in the card picker.

#### Scenario: Out-of-pool card is covered

- **WHEN** the builder format excludes a card from its pool
- **THEN** that card is shown as unavailable in the picker

#### Scenario: Singleton excludes no cards by pool

- **WHEN** the builder format is `singleton`
- **THEN** no card is covered on pool grounds, because `singleton` admits all rarities and applies no banlist

#### Scenario: EDEN covers rare-or-higher non-anomaly cards

- **WHEN** the builder format is `eden` and a main-deck card is rare or higher without qualifying under the Anomaly Protocol
- **THEN** that card is shown as unavailable in the picker

#### Scenario: EDEN does not cover Digi-Eggs by rarity

- **WHEN** the builder format is `eden` and a Digi-Egg card is rare or higher
- **THEN** that Digi-Egg is not covered on rarity grounds

### Requirement: The EDEN anomaly cap is enforced during deck construction

When the selected builder format defines a deck-wide Anomaly Protocol cap, the builder SHALL refuse an addition that would take the deck past that cap, and SHALL surface the current anomaly count against the cap.

#### Scenario: Adding past the cap is refused

- **WHEN** the builder format is `eden`, the deck already holds the maximum permitted Anomaly Protocol copies, and the player attempts to add another Anomaly Protocol card
- **THEN** the addition is refused and the reason cites the anomaly cap

#### Scenario: Adding within the cap is allowed

- **WHEN** the builder format is `eden` and the deck holds fewer than the maximum permitted Anomaly Protocol copies
- **THEN** adding a further Anomaly Protocol card is allowed

#### Scenario: Anomaly usage is visible

- **WHEN** the builder format is `eden`
- **THEN** the builder shows how many Anomaly Protocol cards the deck currently contains and the permitted maximum

#### Scenario: The cap does not apply to formats without one

- **WHEN** the builder format is `standard`, `no_restriction`, or `singleton`
- **THEN** no anomaly cap constrains deck construction

### Requirement: Per-deck legality is shown in the deck library

The deck list SHALL indicate, for each saved deck, whether it is legal under the currently selected builder format.

#### Scenario: Legality is visible per deck

- **WHEN** the deck list is displayed
- **THEN** each deck indicates whether it is legal under the selected builder format

#### Scenario: Indication updates with the format

- **WHEN** the player changes the builder format
- **THEN** the per-deck legality indications update to reflect the new format

### Requirement: Format selection never mutates stored decks

The selected format SHALL be a read-only lens over the deck library. Deck-repair logic that removes cards from a deck SHALL continue to consider only the official banlist and SHALL NOT be influenced by the selected builder or match format.

#### Scenario: Switching to Singleton does not strip saved decks

- **WHEN** the player switches the builder format to `singleton` with saved decks containing multiple copies of cards
- **THEN** no saved deck is modified, and every deck retains exactly the cards it had before the switch

#### Scenario: Switching to EDEN does not strip saved decks

- **WHEN** the player switches the builder format to `eden` with saved decks containing rare-or-higher cards and cards on EDEN's banned list
- **THEN** no saved deck is modified, and every deck retains exactly the cards it had before the switch

#### Scenario: Startup does not strip decks under Singleton

- **WHEN** the client starts with a persisted builder format of `singleton` and loads the deck library
- **THEN** the library-wide deck-repair pass does not remove cards on account of the singleton copy limit, and no deck is written back with fewer cards

#### Scenario: Import under Singleton preserves the list

- **WHEN** the player imports a decklist containing multiple copies of cards while the builder format is `singleton`
- **THEN** the imported deck retains every card in the list and is reported as illegal for `singleton` rather than being silently reduced

#### Scenario: Official banlist repair is unchanged for the legacy formats

- **WHEN** a saved deck exceeds an official banlist limit and the selected format is `standard` or `no_restriction`
- **THEN** the existing repair behaviour applies exactly as it did before this change for that format

#### Scenario: Repair never applies a format's own limits

- **WHEN** the selected format is `singleton` or `eden`
- **THEN** deck repair does not remove cards on account of that format's copy limit, rarity policy, or restricted list; those are reported as illegality instead
