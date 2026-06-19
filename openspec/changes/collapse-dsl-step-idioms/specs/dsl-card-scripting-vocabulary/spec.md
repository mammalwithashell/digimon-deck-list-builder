## ADDED Requirements

### Requirement: All selection verbs accept an inline `then:` action tail

Every selection verb (`select_opponent_permanent`, `select_own_permanent`, `select_hand`, `select_trash`, `select_security`, in addition to the source/budget/union selectors that already support it) SHALL accept an inline `then: <step list>` that runs after the selection resolves, with the selection's binding in scope. Each step in the tail SHALL still surface its own player choices through `pending_selection` (the tail is sugar over the existing select-then-act sequence, never an auto-pick). The longhand two-step form SHALL remain valid.

#### Scenario: select-then-act collapses into one clause

- **WHEN** a card uses `select_opponent_permanent { filter: {kind: digimon}, bind_as: t, then: [ delete_permanent: { target: t } ] }`
- **THEN** it behaves identically to the longhand `select_opponent_permanent { bind_as: t }` followed by `delete_permanent { target: t }`

#### Scenario: tail selections remain RL-visible

- **WHEN** a `then:` tail contains a step that itself requires a choice
- **THEN** that choice is surfaced as its own `pending_selection`, not auto-resolved

#### Scenario: longhand still works

- **WHEN** a card uses the separate select step and act step (no `then:`)
- **THEN** it parses and behaves as before

### Requirement: `reveal_search` composite for the reveal-search idiom

The DSL SHALL provide a `reveal_search { of, count, buckets: [{ filter, to, max, optional, prompt }...], remainder }` verb expressing reveal-top-N → pick into one or more destination buckets → place the remainder, lowering to the same selection sequence as the longhand idiom. Each bucket SHALL be an independent player-visible pick honoring its `optional` flag, and `add_to_hand_from_reveal` (and siblings) SHALL accept multiple cards so an "add up to N" bucket is a single `max: N`.

#### Scenario: single-bucket search

- **WHEN** a card uses `reveal_search { of: you, count: 3, buckets: [{ filter: {trait_has: X}, to: hand, max: 1, optional: true }], remainder: bottom }`
- **THEN** it reveals 3, offers a may-pick of one matching card to hand, and places the rest on the bottom — identical to the four-step longhand

#### Scenario: multi-card add in one bucket

- **WHEN** a bucket specifies `to: hand, max: 2`
- **THEN** up to two revealed cards may be added to hand through that one bucket (no per-card verb repetition)

#### Scenario: empty / all-declined behavior matches longhand

- **WHEN** the revealed pool has no bucket matches, or all optional buckets are declined
- **THEN** the remainder placement still runs, exactly as the hand-rolled idiom did

### Requirement: Security placement supports a chosen position and a single source-polymorphic verb

Security-placement steps SHALL support `position: choice` (a player-elected top/bottom installed as a `pending_selection`), and the family of per-source/per-disposition place-on-security verbs SHALL be consolidated into one verb whose `source:` accepts a hand binding, a permanent/material binding, or self, with replacement behavior expressed as an orthogonal disposition (`none | cancel | handle | observed`). The replacement dispositions SHALL remain usable only within `kind: replacement` clauses.

#### Scenario: top-or-bottom collapses into the step

- **WHEN** a card places a card "as the top or bottom of security" using `position: choice`
- **THEN** a single binary `pending_selection` offers top/bottom and no `select_effect_choice` + paired `if`-arms are needed

#### Scenario: one verb covers the prior sources

- **WHEN** a card places a hand card, a field permanent, or itself onto security
- **THEN** the same consolidated verb expresses all three via its `source:` parameter

#### Scenario: disposition is orthogonal to position

- **WHEN** a replacement-flow card needs "place on the bottom of security and handle the replacement"
- **THEN** that combination is expressible (fixing the prior bottom+handle asymmetry)

### Requirement: `link_card_to_self` is removed in favor of `link_cards`

The deprecated `link_card_to_self` verb SHALL be removed once `link_cards` covers single-card self-host and chosen-host linking, and the cards using it SHALL be migrated to `link_cards` with behavioral parity.

#### Scenario: migrated card behaves identically

- **WHEN** a card previously using `link_card_to_self` is authored with `link_cards`
- **THEN** the link is established with the same host, cost, and source-zone behavior

#### Scenario: deprecated verb is gone

- **WHEN** the vocabulary is enumerated
- **THEN** `link_card_to_self` is absent and `link_cards` is the single link-a-card verb

### Requirement: Link primitives cover relinking, heterogeneous choice, host filtering, and leave-prevention

The DSL/engine SHALL provide link primitives sufficient to faithfully express EX11-027 Maquinamon (folded in from `fix-dsl-substrate-rot-and-bugs`): moving a standing battle-area permanent to become a link card on a chosen own Digimon; a single player selection that chooses between two distinct link operations (with a branch offered only when its precondition holds); host filtering with link-requirement enforcement that excludes the source permanent; and a replacement cost that places a chosen link card as the carrier's bottom digivolution card to cancel a would-leave. Once EX11-027 carries no test-only raw_rust, the `dsl-substrate-integrity` loader guard SHALL be a hard error.

#### Scenario: a standing permanent is relinked as a link card

- **WHEN** EX11-027's `[On Play]` chooses to link this Digimon onto another of the controller's Digimon
- **THEN** the standing permanent becomes a link card on the chosen host, with the host's link requirement enforced and the source excluded as a host

#### Scenario: leave-prevention by placing a link card as a bottom source

- **WHEN** EX11-027 would leave the battle area and the controller elects the link-card replacement
- **THEN** a chosen link card is placed as the carrier's bottom digivolution card and the leave is cancelled

#### Scenario: guard promotes to hard error once no unregistered refs remain

- **WHEN** EX11-027 has migrated off raw_rust and the embedded pack has zero unregistered raw_rust references
- **THEN** the loader guard rejects any future unregistered reference as a hard load error (no warn-mode fallback)
