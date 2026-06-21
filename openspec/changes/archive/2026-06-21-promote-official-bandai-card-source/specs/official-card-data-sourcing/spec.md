## ADDED Requirements

### Requirement: Official Bandai DB is the authoritative source for printed card data
The system SHALL treat the official Bandai global card database (`world.digimoncard.com`) as the authoritative source for printed/structured card data — traits (including Rule-granted traits and attributes), digivolution costs and conditions, printed effect/inherited/security text, and official Q&A rulings — ranking it above both `data/cards.json` (the lossy digimoncard.io API ingest) and DCGO for those data classes. DCGO MUST remain the authority for behavioral resolution (processing order, interaction edges, multi-pick flow), and `general_rule.pdf` MUST remain canonical for rules, keyword semantics, and timing.

#### Scenario: Official DB and cards.json disagree on a printed trait
- **WHEN** a contributor or agent needs a card's printed traits and the official DB lists a trait that `cards.json` omits
- **THEN** the official DB value is treated as authoritative and `cards.json` is corrected to match

#### Scenario: Official DB versus DCGO for data versus behavior
- **WHEN** a question concerns a card's printed data (a trait, a digivolve cost, printed text)
- **THEN** the official Bandai DB is consulted ahead of DCGO
- **WHEN** a question concerns how a card resolves (processing order, interaction edges)
- **THEN** DCGO remains the authority

#### Scenario: Documented source priority reflects the promotion
- **WHEN** the source-priority guidance in `CLAUDE.md`, the `digimon-card-lookup` skill, and the source-priority feedback memory are read
- **THEN** each states that `world.digimoncard.com` outranks DCGO and `cards.json` for printed/structured card data, while DCGO stays authoritative for behavior and `general_rule.pdf` stays canonical for rules

### Requirement: Rule-granted traits and attributes are recovered from the official text
The system SHALL recover `(Rule) Trait: Has [X] Type.` grants as traits and `Has [X] attribute.` grants as attributes by parsing each card's official text from `data/card_official.json`, and the recovery process MUST refresh `card_official.json` across the full Digimon card pool so coverage is not limited to a prior partial scrape.

#### Scenario: Type grant recovered from official text
- **WHEN** a card's official effect text contains "(Rule) Trait: Has [Ice-Snow] Type." while its trait line lists only other traits
- **THEN** the recovery process records `Ice-Snow` as a granted trait for that card

#### Scenario: Attribute grant recovered from official text
- **WHEN** a card's official effect text contains "Rule: Trait: Has [Free] attribute."
- **THEN** the recovery process records `Free` as a granted attribute for that card

#### Scenario: Full-pool refresh before recovery
- **WHEN** the recovery is run
- **THEN** `card_official.json` is first refreshed over the full Digimon pool, not only the cards previously scraped

### Requirement: Recovered grants are propagated into production card data without dropping existing traits
The system SHALL propagate recovered traits/attributes into `data/cards.json` through `data/card_overrides.json` so that production `CardData.traits` (built from `cards.json`) carries them for every consumer, and each override MUST preserve all pre-existing trait-line entries (the granted value is added to, never replaces, the existing set).

#### Scenario: Production CardData carries the recovered trait
- **WHEN** the engine builds `CardData` from `cards.json` for a Rule-granted card after propagation
- **THEN** `CardData.traits` includes the recovered trait (e.g. CrysPaledramon EX7-021 includes `Ice-Snow`)

#### Scenario: Alt-digivolution requirement recognizes the recovered trait
- **WHEN** a card's alt-digivolution path requires a base with the `Ice-Snow` trait and the base is a Rule-granted Ice-Snow Digimon
- **THEN** the path is offered in production (the requirement matches against the recovered trait)

#### Scenario: Override preserves an existing multi-trait line
- **WHEN** a card already carries a trait and also has a Rule grant for a different trait (e.g. P-215 Icemon: `Ice-Snow` line + `Mineral` grant)
- **THEN** the propagated `type_eng` contains both traits, dropping neither

#### Scenario: Corrections survive re-ingestion
- **WHEN** the card ingestion pipeline re-runs from the API
- **THEN** the recovered trait/attribute corrections remain applied (carried by `card_overrides.json`)

### Requirement: Authored-trait divergences are reconciled from the official DB
The system SHALL reconcile every card whose authored DSL `traits:` exceeds its production `CardData` trait set against the official Bandai DB, treating the official DB's `type` / `form` / `attribute` split (plus any Rule grants) as authoritative: a trait the official DB confirms but `cards.json` lacks MUST be added to the appropriate `cards.json` field (via overrides), and a trait the DSL declares that the official DB does NOT list MUST be corrected in the DSL spec instead. This explicitly covers traits the API drops by emptying the `form` field — including the `Appmon` mechanic trait (printed in form as `<grade>/Appmon`, required by many cards).

#### Scenario: Form-field trait recovered into production
- **WHEN** a card carries a trait in the official DB's `form` field that `cards.json` dropped (e.g. an Appmon card whose official form is `Stnd./Appmon`)
- **THEN** the `Appmon` trait is present in the card's production `CardData.traits` after reconciliation

#### Scenario: A required Appmon-trait interaction recognizes the card
- **WHEN** an effect requires a Digimon card with the `[Appmon]` trait and an Appmon card is a candidate
- **THEN** the candidate is recognized (its production traits include `Appmon`)

#### Scenario: DSL-declared trait absent from the official DB is corrected in the spec
- **WHEN** a DSL spec declares a trait the official DB does not list for that card (an authoring error)
- **THEN** the DSL spec is corrected to the official trait set rather than the trait being injected into `cards.json`

### Requirement: YAML-versus-production trait divergence is guarded against
The system SHALL provide an automated guard ensuring that, for every DSL-authored card, the production `CardData` trait set (built from `cards.json`) is a superset of the card's authored YAML `traits:` field, so a trait declared only in YAML can no longer pass behavioral tests while being absent in production.

#### Scenario: Guard fails when a YAML trait is missing from production data
- **WHEN** a DSL card's YAML `traits:` declares a trait that the production `cards.json`-built `CardData` lacks
- **THEN** the guard reports a failure identifying the card and the missing trait

#### Scenario: Guard passes when production data covers the YAML traits
- **WHEN** every DSL card's YAML traits are present in the production `CardData` trait set
- **THEN** the guard passes
