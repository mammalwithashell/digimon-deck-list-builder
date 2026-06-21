## ADDED Requirements

### Requirement: Deck prior supplies the hidden-card distribution for determinization
The system SHALL provide a `DeckPrior` over an opponent's unrevealed cards that the world materializer samples from, consistent with cards already revealed during the game (revealed cards reduce the prior's remaining mass).

#### Scenario: Revealed cards condition the prior
- **WHEN** opponent cards have been revealed (played, trashed, security-checked) during a game
- **THEN** the `DeckPrior` for that opponent excludes/decrements those cards from the distribution of remaining unknown cards

### Requirement: Exact prior in self-play, inferred prior in PvP
In self-play/training where both decklists are drawn from a known pool, the `DeckPrior` SHALL be the exact decklist multiset minus revealed cards. In PvP review where the opponent's decklist is unknown, the `DeckPrior` SHALL be inferred (from revealed cards plus a meta-deck prior) rather than assumed known.

#### Scenario: Self-play uses the exact deck
- **WHEN** a `DeckPrior` is requested during self-play
- **THEN** it is the opponent's known decklist composition minus revealed cards (no inference)

#### Scenario: PvP infers from revealed signals
- **WHEN** a `DeckPrior` is requested for a PvP recording with an unknown opponent decklist
- **THEN** it is derived from revealed cards plus a meta prior (e.g. `data/deck_library.json` archetype distribution), not assumed to be a specific decklist
