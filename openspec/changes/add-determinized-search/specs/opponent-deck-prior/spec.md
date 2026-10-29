## ADDED Requirements

### Requirement: Deck prior supplies the hidden-card distribution for determinization
The system SHALL provide a `DeckPrior` over an opponent's unrevealed cards that the world materializer samples from, consistent with cards already revealed during the game (revealed cards reduce the prior's remaining mass).

#### Scenario: Revealed cards condition the prior
- **WHEN** opponent cards have been revealed (played, trashed, security-checked) during a game
- **THEN** the `DeckPrior` for that opponent excludes/decrements those cards from the distribution of remaining unknown cards

### Requirement: Prior follows the declared information regime
Open-decklist self-play and review SHALL use the entitled exact opponent list minus currently accounted-for cards. Closed-decklist self-play, training, evaluation and PvP review SHALL infer the prior from viewer-entitled evidence and a frozen legal population rather than consult the actual opposing list. The closed-list provider, knowledge lifecycle and training integration are owned by `add-fair-belief-selfplay`; replay presentation remains in this change.

#### Scenario: Open-list self-play uses the exact deck
- **WHEN** a `DeckPrior` is requested during explicitly open-decklist self-play
- **THEN** it is the entitled opponent decklist composition minus currently accounted-for cards

#### Scenario: Closed-list self-play cannot consult the true list
- **WHEN** a prior is requested during closed-decklist self-play
- **THEN** it uses the acting seat's knowledge and frozen prior configuration even though the private game runner knows the opposing list

#### Scenario: PvP infers from revealed signals
- **WHEN** a `DeckPrior` is requested for a PvP recording with an unknown opponent decklist
- **THEN** it is derived from revealed cards plus a meta prior (e.g. `data/deck_library.json` archetype distribution), not assumed to be a specific decklist
