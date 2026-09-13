## ADDED Requirements

### Requirement: EDEN format definition

The `eden` format SHALL require a main deck of exactly 50 cards, a Digi-Egg deck of 0 to 5 cards, and at most 4 copies of any card ID subject to EDEN's restricted list. It SHALL apply EDEN's rarity policy, EDEN's Anomaly Protocol, and EDEN's banned, limited, and banned-pair list. Its rules source is the EDEN Format Rules & Guidance as recorded in `docs/EDEN_FORMAT_RULES.md`.

#### Scenario: A conforming deck is legal

- **WHEN** a 50-card main deck of common and uncommon cards, with up to five Digi-Eggs, contains no banned or over-limit card and no Anomaly Protocol cards
- **THEN** the deck is reported legal for `eden`

#### Scenario: Deck size still applies

- **WHEN** a deck under `eden` has a main deck that is not exactly 50 cards
- **THEN** the deck is reported illegal for `eden`

### Requirement: EDEN rarity policy

Under `eden`, common and uncommon cards SHALL be legal by default, and main-deck cards of rare or higher rarity SHALL be illegal unless they qualify under the Anomaly Protocol. A card SHALL be treated as rare-or-higher when its rarity is anything other than common or uncommon, including rarities the client does not recognise and cards whose rarity is absent.

#### Scenario: Common and uncommon are legal

- **WHEN** a main-deck card under `eden` has rarity common or uncommon
- **THEN** it is legal on rarity grounds

#### Scenario: Rare main-deck card without anomaly qualification is illegal

- **WHEN** a main-deck card under `eden` has rare rarity and does not qualify under the Anomaly Protocol
- **THEN** the deck containing it is reported illegal for `eden`, identifying that card

#### Scenario: Secret and other high rarities are illegal

- **WHEN** a main-deck card under `eden` has a rarity above rare, such as super rare, ultra rare, or secret, and does not qualify under the Anomaly Protocol
- **THEN** the deck containing it is reported illegal for `eden`

#### Scenario: Unknown or missing rarity fails closed

- **WHEN** a main-deck card under `eden` has no resolvable rarity
- **THEN** it is treated as rare-or-higher and is illegal unless it qualifies under the Anomaly Protocol

### Requirement: Digi-Eggs are exempt from the EDEN rarity policy

Under `eden`, Digi-Egg cards SHALL be legal at any rarity, and SHALL remain subject to EDEN's banned and limited list.

#### Scenario: A rare Digi-Egg is legal

- **WHEN** a Digi-Egg card of rare or higher rarity is placed in the Digi-Egg deck under `eden`
- **THEN** it is legal on rarity grounds

#### Scenario: Digi-Eggs do not consume the anomaly allowance

- **WHEN** a deck under `eden` contains Digi-Eggs of rare or higher rarity
- **THEN** those Digi-Eggs do not count toward the Anomaly Protocol total

#### Scenario: A banned Digi-Egg is still illegal

- **WHEN** a Digi-Egg card appears on EDEN's banned list and is placed in the Digi-Egg deck
- **THEN** the deck is reported illegal for `eden`

### Requirement: EDEN Anomaly Protocol qualification

A card SHALL qualify as an EDEN Anomaly Protocol card when it matches a category defined by the format data — a card kind, an optional case-insensitive match on the English card name, and a set of qualifying rarities — or when its card ID is listed explicitly in the format data. Qualification SHALL NOT be determined by logic hardcoded per card.

#### Scenario: Rare and promo Tamers qualify

- **WHEN** a Tamer card of rare or promo rarity is evaluated under `eden`
- **THEN** it qualifies as an Anomaly Protocol card

#### Scenario: Memory Boost options qualify at rare, super rare, and promo

- **WHEN** an Option card whose English name contains "memory boost" has rare, super rare, or promo rarity
- **THEN** it qualifies as an Anomaly Protocol card

#### Scenario: Promo Training and Scramble options qualify

- **WHEN** an Option card whose English name contains "training" or "scramble" has promo rarity
- **THEN** it qualifies as an Anomaly Protocol card

#### Scenario: A non-promo Training option does not qualify

- **WHEN** an Option card whose English name contains "training" has a rarity other than promo
- **THEN** it does not qualify as an Anomaly Protocol card, and is illegal under `eden` if it is rare or higher

#### Scenario: A rare Digimon does not qualify

- **WHEN** a Digimon card of rare rarity is evaluated under `eden` and matches no anomaly category
- **THEN** it does not qualify as an Anomaly Protocol card

#### Scenario: An explicitly listed card qualifies

- **WHEN** a card ID appears in the format data's explicit anomaly card list
- **THEN** it qualifies as an Anomaly Protocol card regardless of category matching

### Requirement: EDEN Anomaly Protocol total cap

A deck under `eden` SHALL contain at most the configured maximum number of Anomaly Protocol cards, counted as total copies rather than distinct card IDs.

#### Scenario: At the cap is legal

- **WHEN** a deck under `eden` contains exactly the configured maximum number of Anomaly Protocol card copies
- **THEN** the deck is not reported illegal on anomaly-cap grounds

#### Scenario: Over the cap is illegal

- **WHEN** a deck under `eden` contains more than the configured maximum number of Anomaly Protocol card copies
- **THEN** the deck is reported illegal for `eden`, stating the count and the cap

#### Scenario: Copies count individually

- **WHEN** a deck under `eden` contains four copies of a single qualifying rare Tamer and the cap is four
- **THEN** the anomaly allowance is fully consumed and adding any further Anomaly Protocol card makes the deck illegal

### Requirement: EDEN restricted list

The `eden` format SHALL apply EDEN's own banned, limited, explicitly-limited, and banned-pair entries, independently of the official banlist used by `standard`.

#### Scenario: An EDEN-banned card is illegal

- **WHEN** a deck under `eden` contains a card on EDEN's banned list
- **THEN** the deck is reported illegal for `eden`

#### Scenario: An EDEN-limited card is capped

- **WHEN** a deck under `eden` contains more copies of a card than EDEN's list permits for it
- **THEN** the deck is reported illegal for `eden`

#### Scenario: EDEN banned pairs are enforced

- **WHEN** a deck under `eden` contains cards from both sides of an EDEN banned pair or choice group
- **THEN** the deck is reported illegal for `eden`

#### Scenario: EDEN list is independent of the official list

- **WHEN** a card is banned on the official list but not on EDEN's list
- **THEN** it is not reported illegal under `eden` on account of the official list

### Requirement: EDEN format data ships with the client

EDEN's restricted list and Anomaly Protocol definition SHALL be carried as client data rather than as code, encoded so the client's existing JSON deserializer can read it, and SHALL correspond to this repository's `data/deck_formats.json` by a documented one-to-one mapping so that a rules revision is a data-only change and the two copies can be compared mechanically.

#### Scenario: Rules revision requires no code change

- **WHEN** EDEN publishes a revised banlist or an additional Anomaly Protocol category
- **THEN** the change is made by editing the shipped format data, without modifying validation logic

#### Scenario: No new network dependency

- **WHEN** the client starts and resolves the `eden` format
- **THEN** it obtains EDEN's data without contacting any service, and the existing official-banlist fetch is unchanged

#### Scenario: Data shape supports comparison

- **WHEN** the shipped EDEN data is compared against this repository's format registry
- **THEN** the restricted-list and Anomaly Protocol structures correspond field for field, so a divergence is detectable
