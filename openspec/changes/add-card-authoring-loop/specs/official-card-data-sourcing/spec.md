## ADDED Requirements

### Requirement: Official Q&A rulings are mirrored completely and keyed by Q-number
The system SHALL mirror every official Q&A ruling published on the Bandai card database into `data/card_qa.json`, storing for each ruling its Q-number, its question, its answer, its date when printed, and every card id whose official page lists it, keyed by Q-number so a ruling shared by several cards is stored once.

#### Scenario: A card with several rulings keeps all of them
- **WHEN** a card's official page lists rulings Q1601, Q1602, Q1603 and Q1604
- **THEN** `data/card_qa.json` maps that card to all four Q-numbers, and each ruling entry holds both its question and its answer

#### Scenario: Rulings repeated per printing are stored once
- **WHEN** a card's official page repeats the same rulings once per printing
- **THEN** each Q-number appears once for that card

#### Scenario: A shared ruling is linked to every card that prints it
- **WHEN** two different card ids both list ruling Q1601
- **THEN** the single Q1601 entry lists both card ids

#### Scenario: Ruling text is plain text
- **WHEN** a ruling's question or answer contains inline markup or HTML entities
- **THEN** the stored text has the markup removed and the entities decoded, with words joined by inline markup kept intact

### Requirement: A search that finds no card is never recorded as a card without rulings
The Q&A mirror MUST distinguish "the official search found no such card" from "the card exists and has no rulings", recording the former in a `failed` list and never as an empty ruling list.

#### Scenario: No-results page
- **WHEN** the official search page for a card id contains no card block with that exact card number
- **THEN** the card id is listed under `failed` and has no entry in the card-to-rulings map

#### Scenario: Card exists with no rulings
- **WHEN** the official page shows the card but no Q&A block
- **THEN** the card maps to an empty ruling list

#### Scenario: A page listing a different card number
- **WHEN** the requested card's search page lists only other card numbers
- **THEN** no rulings from those other cards are attributed to the requested card

### Requirement: Conflicting text for one Q-number is surfaced, not overwritten
The Q&A mirror SHALL record a conflict, and keep the first-seen text, when the same Q-number appears with a different question or answer on a different card.

#### Scenario: Divergent ruling text across cards
- **WHEN** Q1601 is already stored from one card and another card's page lists Q1601 with different answer text
- **THEN** the stored Q1601 text is unchanged and a `{q_id, card_id}` conflict is recorded

### Requirement: The Q&A mirror is resumable and deterministic
The Q&A scraper SHALL skip cards already present in the dataset unless asked to refresh, SHALL checkpoint its progress during long runs, and SHALL write its output with sorted keys and numerically ordered Q-numbers so an unchanged source produces an unchanged file.

#### Scenario: Resume after interruption
- **WHEN** the scraper is re-run without `--refresh` after an interrupted run
- **THEN** only cards not yet present in the dataset are fetched

#### Scenario: Stable ordering
- **WHEN** rulings Q601 and Q1601 are both stored
- **THEN** Q601 is written before Q1601
