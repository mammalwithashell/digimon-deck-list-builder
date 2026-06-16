## ADDED Requirements

### Requirement: Effective seed metadata for deck-created games

The live game surface SHALL expose the effective seed used for any fresh game constructed from deck lists. The effective seed SHALL be serializable as a decimal string so tool, API, and frontend consumers can reproduce initial shuffle state without numeric precision loss.

#### Scenario: Deck-created game with explicit seed exposes seed

- **WHEN** a caller constructs a fresh game from two deck lists with seed `12345`
- **THEN** the live game metadata exposes effective seed `"12345"`
- **AND** constructing the same decks again with seed `12345` reproduces the same initial deck order, opening hands, and initial player

#### Scenario: Deck-created game without explicit seed exposes generated seed

- **WHEN** a caller constructs a fresh game from two deck lists without providing a seed
- **THEN** the live game metadata exposes the generated effective seed as a decimal string
- **AND** constructing the same decks again with that exposed seed reproduces the original initial deck order, opening hands, and initial player
