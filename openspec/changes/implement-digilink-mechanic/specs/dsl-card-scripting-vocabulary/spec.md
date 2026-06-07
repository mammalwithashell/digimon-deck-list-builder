## ADDED Requirements

### Requirement: DSL can author a Digimon self link-condition

The YAML DSL SHALL let a `kind: digimon` card declare a self link-condition with a link cost and a host filter, lowering to the engine's card-level link-condition metadata. This is distinct from the Option-scoped `link_requirement` clause used by Plug-In Options.

#### Scenario: Digimon link-condition lowers to metadata

- **WHEN** a `kind: digimon` card declares a link condition with a cost and an `Appmon`-trait host filter
- **THEN** the compiled card exposes a self link-condition with that cost and filter
- **AND** the engine offers a link-activate ability for the card only against hosts the filter accepts

#### Scenario: Option link_requirement is unaffected

- **WHEN** an Option (Plug-In) card declares `kind: link_requirement` with `scope: inherited`
- **THEN** it continues to lower to an `OptionMain` link effect as before
- **AND** the Digimon self link-condition vocabulary does not change Option link behavior

### Requirement: DSL can author WhenLinked triggers and linked ESS grants

The YAML DSL SHALL let a card declare a `when_linked` trigger and a `scope: linked` ESS grant (DP and/or keyword) so that Shape-B Appmon Digimon are authored declaratively rather than as raw Rust.

#### Scenario: WhenLinked trigger lowers to the link-attach timing

- **WHEN** a card declares an effect with `when: when_linked`
- **THEN** the effect resolves when the card is linked to a host
- **AND** it does not resolve on a normal play or digivolve

#### Scenario: Linked ESS grant reaches the host

- **WHEN** a card declares a `scope: linked` effect granting `Raid`
- **THEN** the grant applies to the host while the card is linked
- **AND** it is removed when the link ends
