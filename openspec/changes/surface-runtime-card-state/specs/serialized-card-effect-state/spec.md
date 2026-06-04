## ADDED Requirements

### Requirement: Stack sources carry their printed effect text

The engine's `to_ui_json` serialization SHALL populate each battle-area permanent stack source's `mainEffectText` and `inheritedEffectText` from the engine's card data, instead of empty strings. A source with no corresponding printed text for a field SHALL serialize that field as an empty string.

#### Scenario: A stacked source exposes its inherited effect text

- **WHEN** a battle-area permanent has a digivolution source whose card has a printed inherited effect
- **THEN** that source's serialized `inheritedEffectText` contains the card's printed inherited-effect text (non-empty)

#### Scenario: A source's main effect text is exposed

- **WHEN** a battle-area permanent's source card has printed main-effect text
- **THEN** that source's serialized `mainEffectText` contains the printed main-effect text

#### Scenario: Cards without text serialize empty, not missing

- **WHEN** a source card has no printed text for a given effect field
- **THEN** that field serializes as an empty string and the serialization does not error or omit the field

### Requirement: Permanent exposes its active inherited effects

The engine's `to_ui_json` serialization SHALL populate a battle-area permanent's `inheritedEffects` array to reflect the inherited effects currently conferred on the permanent by its digivolution sources (excluding the top card), each identified with enough information to attribute it to its source. Where engine state changes which inherited effects apply to the permanent, the serialized set SHALL reflect that runtime state rather than printed text alone.

#### Scenario: Inherited effects from sources are listed

- **WHEN** a permanent has one or more digivolution sources beneath the top card that grant inherited effects
- **THEN** the permanent's serialized `inheritedEffects` lists those inherited effects, attributable to their source cards

#### Scenario: Top card is excluded from inherited effects

- **WHEN** a permanent's inherited effects are serialized
- **THEN** the top (active) card's own main effect is not listed as an inherited effect

#### Scenario: Runtime substitution or removal is reflected

- **WHEN** engine state has substituted or removed an inherited effect that a source would otherwise confer
- **THEN** the serialized `inheritedEffects` reflects the effect actually in force, not merely the source's printed inherited text

### Requirement: Existing runtime fields are unchanged

This serialization change SHALL be limited to the previously-stubbed effect-text fields. The runtime DP, DP breakdown, keyword, and keyword-breakdown fields already exposed per permanent and per source SHALL be unchanged in shape and value.

#### Scenario: DP and keyword breakdowns are preserved

- **WHEN** a permanent is serialized after this change
- **THEN** its `dp`, `dpBreakdown`, `keywords`, `keywordBreakdown`, and each source's `dpContribution` are identical to before the change
