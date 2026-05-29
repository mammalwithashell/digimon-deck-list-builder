## ADDED Requirements

### Requirement: DSL can author source-zone effect digivolve

The card-scripting DSL SHALL provide declarative vocabulary for selecting a
digivolution card from source-like zones such as cards under own Tamers and
using that selected card in an effect-initiated digivolution.

#### Scenario: YAML selects a card under a Tamer for digivolution

- **WHEN** a card YAML declares an effect that selects a matching card under an
  own Tamer and digivolves one of the controller's Digimon into it
- **THEN** DSL lowering emits the source-zone selection and effect-initiated
  digivolve steps without raw Rust placeholders

#### Scenario: YAML uses an unsupported source zone

- **WHEN** a card YAML requests source-zone effect digivolve from a zone that the
  DSL does not support
- **THEN** compilation fails with an explicit unsupported-source-zone error

### Requirement: DSL can author stack-derived effect metrics

The card-scripting DSL SHALL expose predicates and formulas for source-stack
metrics, including no-source filters, fewest-source selection, source-color
counts, and comparison against the acting Digimon's current DP.

#### Scenario: YAML declares a fewest-source target selector

- **WHEN** a card YAML declares an effect targeting the opponent Digimon with the
  fewest digivolution cards
- **THEN** DSL lowering emits a selector that preserves ties and excludes
  higher-source candidates

#### Scenario: YAML declares a source-color formula

- **WHEN** a card YAML declares a modifier amount based on colors represented in
  a source stack
- **THEN** DSL lowering emits `source_color_count` or
  `per: source_color_count` formula data that resolves against source cards
  beneath the resolving effect carrier's top card at effect resolution time

### Requirement: DSL can author temporary lockouts

The card-scripting DSL SHALL provide vocabulary for applying temporary lockouts
that suppress specific timing-effect families and/or unsuspend behavior until a
declared expiry.

#### Scenario: YAML declares a timing-effect lockout

- **WHEN** a card YAML declares that a selected Digimon cannot activate When
  Digivolving effects until the end of the opponent's turn
- **THEN** DSL lowering emits an expiring timing-effect lockout modifier with
  that timing family and duration

#### Scenario: YAML declares an unsupported timing family

- **WHEN** a card YAML declares a temporary effect lockout for an unknown timing
  family
- **THEN** compilation fails with an explicit unsupported-lockout-timing error

### Requirement: DSL can play selected revealed cards for free

The card-scripting DSL SHALL allow `choose_from_reveal` to route a selected
revealed card to a free play destination while preserving the normal reveal
selection and remainder-ordering flow.

#### Scenario: YAML plays a selected revealed Tamer for free

- **WHEN** a card YAML reveals cards, declares `choose_from_reveal` with
  `destination: play_free`, and then orders the remainder
- **THEN** DSL lowering emits a reveal selection whose selected card is played
  without paying its cost and whose unselected revealed cards remain available
  to `order_remainder`

#### Scenario: Reveal free play is cancelled by a would-play replacement

- **WHEN** the selected revealed card cannot be committed because a would-play
  replacement or play gate prevents it
- **THEN** the card is restored to the reveal pool rather than being hidden in
  hand or lost from the remainder flow
