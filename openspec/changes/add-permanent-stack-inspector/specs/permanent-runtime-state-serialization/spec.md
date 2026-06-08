## ADDED Requirements

### Requirement: Active keywords and innate/granted breakdown
`to_ui_json` SHALL populate each battle-area permanent's `keywords` with the keywords currently active on it, and `keywordBreakdown` with an `innate` set (printed face keywords plus active inherited keywords from its digivolution sources) and a `gained` set (keywords active only via grant modifiers). The fields MUST be derived from live engine state, not hard-coded.

#### Scenario: Innate printed keyword
- **WHEN** a permanent's top card has a printed keyword (e.g. Blocker) and no granting modifier
- **THEN** the serialized `keywords` includes "Blocker" AND `keywordBreakdown.innate` includes "Blocker" AND `keywordBreakdown.gained` does not

#### Scenario: Modifier-granted keyword
- **WHEN** a permanent has a keyword present only through an active grant modifier (e.g. `GrantBlocker`) and not on its printed text
- **THEN** the serialized `keywords` includes that keyword AND `keywordBreakdown.gained` includes it AND `keywordBreakdown.innate` does not

#### Scenario: Inherited keyword from a source
- **WHEN** a non-top digivolution source confers an active inherited keyword on the permanent
- **THEN** the serialized `keywords` includes that keyword AND it appears in `keywordBreakdown.innate`

### Requirement: Security-attack modifier
`to_ui_json` SHALL set each Digimon permanent's `securityAttackModifier` to its live security-attack delta (keyword bonus plus summed security-attack-change modifiers), defaulting to 0 when none apply.

#### Scenario: Security attack increased
- **WHEN** a permanent has an effect granting +2 to its security attack
- **THEN** the serialized `securityAttackModifier` is 2

#### Scenario: No security attack modifier
- **WHEN** a permanent has no security-attack effect
- **THEN** the serialized `securityAttackModifier` is 0

### Requirement: DP breakdown reflects effective DP
`to_ui_json` SHALL populate `dpBreakdown.base` with the permanent's printed DP, `dpBreakdown.total` with its effective DP (including modifiers), and `dpBreakdown.temporary` with the difference between total and base.

#### Scenario: DP buff applied
- **WHEN** a permanent with 4000 printed DP has a +3000 DP modifier active
- **THEN** `dpBreakdown.base` is 4000 AND `dpBreakdown.total` is 7000 AND `dpBreakdown.temporary` is 3000

#### Scenario: No DP modifier
- **WHEN** a permanent has no DP modifier
- **THEN** `dpBreakdown.total` equals `dpBreakdown.base` AND `dpBreakdown.temporary` is 0

### Requirement: Per-source and inherited effect text
`to_ui_json` SHALL populate each stack source's `mainEffectText` and `inheritedEffectText` from the engine's card data, and the permanent-level `inheritedEffects` array from its non-top digivolution sources, instead of empty strings/arrays. The top card MUST NOT appear as an inherited effect.

#### Scenario: Stacked permanent exposes source effect text
- **WHEN** a permanent has a digivolution source whose card has printed inherited text
- **THEN** that source's serialized `inheritedEffectText` is the non-empty printed text AND the permanent's `inheritedEffects` includes an entry attributable to that source

#### Scenario: Single-card permanent has no inherited effects
- **WHEN** a permanent has only its top card (no sources beneath)
- **THEN** the serialized `inheritedEffects` array is empty

### Requirement: Active modifier list
`to_ui_json` SHALL emit a `modifiers` array for each battle-area permanent containing every display-relevant, permanent-scoped active modifier, each as a structured object `{ type, value, expiry, sourceCardId }`. `type` MUST be a stable string from an explicit mapping (not a debug formatting of the enum). Internal/bookkeeping modifier state and player-scoped modifiers MUST NOT be emitted.

#### Scenario: Immunity modifier emitted
- **WHEN** a permanent has an active `CannotBeDestroyed`/`CannotBeDeleted`-class modifier
- **THEN** the serialized `modifiers` array contains an entry whose `type` is the stable string for that modifier and whose `expiry` reflects the modifier's expiry

#### Scenario: Stat-change modifier emitted with value
- **WHEN** a permanent has a +3000 DP modifier active
- **THEN** the serialized `modifiers` contains an entry with the DP-change `type` and `value` 3000

#### Scenario: Restriction modifier emitted
- **WHEN** a permanent has an active `CannotSuspend` modifier
- **THEN** the serialized `modifiers` contains an entry with the stable string for `CannotSuspend`

#### Scenario: No active modifiers
- **WHEN** a permanent has no active modifiers
- **THEN** the serialized `modifiers` array is empty (present, not omitted)

### Requirement: Wire-shape stability
The change SHALL keep the existing `PermanentInfo` keys present and correctly typed; populating previously-empty fields and adding `modifiers` MUST be additive (no key removed or renamed).

#### Scenario: Shape regression guard
- **WHEN** a battle-area permanent is serialized
- **THEN** the output contains `keywords`, `keywordBreakdown`, `securityAttackModifier`, `dpBreakdown`, `sources`, `inheritedEffects`, and `modifiers` with their documented types
