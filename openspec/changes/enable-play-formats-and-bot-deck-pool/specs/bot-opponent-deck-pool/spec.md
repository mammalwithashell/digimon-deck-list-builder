## ADDED Requirements

### Requirement: Bot opponent deck pool
Local bot matches SHALL choose the greedy bot's opponent deck from a random pool containing the built-in ST1 through ST6 starter decks and any saved player decks that are valid for the selected format. The selected player deck SHALL NOT be eligible as a saved-deck opponent candidate.

#### Scenario: Starter decks are always eligible
- **WHEN** a player launches a bot match in any selectable play format
- **THEN** the bot opponent pool includes exactly one built-in candidate for each of ST1, ST2, ST3, ST4, ST5, and ST6

#### Scenario: Saved decks must match the selected format
- **WHEN** the selected play format is EDEN
- **AND** the player has saved decks for EDEN and Standard
- **THEN** only saved decks that pass the EDEN deck-selection legality gate are added to the bot opponent pool

#### Scenario: Selected player deck is excluded
- **WHEN** the player launches a bot match with a saved deck selected
- **THEN** that selected deck is excluded from the saved-deck bot opponent candidates
- **AND** the bot launch payload does not use the selected deck as both player decks

#### Scenario: Bot match launches without saved opponent decks
- **WHEN** the player has no saved decks other than the selected deck
- **THEN** the bot match can still launch using one of the built-in ST1 through ST6 starter decks as the opponent deck

### Requirement: Bot opponent selection is launch-local
The system SHALL choose the bot opponent deck at bot-match launch time and pass the chosen deck through the existing bot-game creation path. The shuffle seed SHALL continue to control game setup and SHALL NOT be required to make the bot opponent deck choice deterministic.

#### Scenario: Existing game creation path receives a concrete opponent deck
- **WHEN** the bot opponent pool chooses a candidate
- **THEN** `createBotGame` receives the player's selected deck as player one and the chosen opponent candidate as player two

#### Scenario: Shuffle seed does not select the opponent deck
- **WHEN** two bot launches use the same shuffle seed
- **THEN** the seed is forwarded to game creation for shuffle/setup behavior
- **AND** the bot opponent deck picker is not required to choose the same opponent deck for both launches
