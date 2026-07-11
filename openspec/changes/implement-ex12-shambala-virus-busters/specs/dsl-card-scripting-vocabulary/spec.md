# Delta: dsl-card-scripting-vocabulary

## ADDED Requirements

### Requirement: EX12 keywords in the DSL keyword surface
The DSL validator's keyword allowlist (`KNOWN_KEYWORD_KEYS`) SHALL accept `Guard` and `Engage`, and `grant_keyword` clauses naming them SHALL lower consistently with the established native-printed-keyword pattern (as with `Training`/`Ascension`: the runtime behavior rides the printed-keyword parse and keyword machinery; a `grant_keyword` clause is the visible compiled-DSL declaration and any aura-granted instance MUST activate the same behavior as a printed instance).

#### Scenario: dsl-lint accepts the new keywords
- **WHEN** a YAML card declares `grant_keyword: Guard` or `grant_keyword: Engage`
- **THEN** `dsl-lint` reports no unknown-keyword error and the card compiles

#### Scenario: Aura-granted Guard behaves like printed Guard
- **WHEN** an aura grants ＜Guard＞ to a Digimon for a duration (e.g. EX12-072's [Security] effect granting all [ME] Digimon Guard)
- **THEN** the granted Digimon offers the same protect-others leave replacement as a printed Guard carrier while the grant is active

### Requirement: Play or use from digivolution sources
The DSL SHALL provide a unified source-origin verb for effects that play or use
one card from a Digimon's digivolution cards. This verb SHALL share the face and
kind routing semantics of `play_or_use_from_hand`, while moving the selected card
from its original digivolution-source stack.

#### Scenario: Mixed source candidates route by kind
- **WHEN** an effect selects a Digimon, Tamer, Option, or DUAL card from one of the player's Digimon's digivolution cards and resolves `play_or_use_from_sources` for free
- **THEN** Digimon and Tamer cards are played, Option cards are used, and DUAL cards expose the same play-face/use-face choice as `play_or_use_from_hand`

#### Scenario: EX12-077 source-origin choice is player-visible
- **WHEN** EX12-077 resolves its [On Play]/[When Digivolving]/[When Attacking]/[Counter] source-card effect
- **THEN** the action mask exposes the legal card choices across all of the player's battle-area digivolution sources and routes the chosen card without hidden auto-selection

### Requirement: Assessment-surfaced vocabulary additions are spec'd before implementation
Any DSL verb, predicate, or timing the EX12 gap assessment surfaces beyond the two keywords SHALL be added to this delta (with its own requirement and scenarios) before the closure round that implements it, so the vocabulary contract is reviewable ahead of code.

#### Scenario: New vocabulary lands with spec coverage
- **WHEN** a closure round adds a new DSL step/predicate for an EX12 card
- **THEN** this delta contains a requirement + scenario for it, and the vocab-doc drift gate (`docs/RUST_DSL_AGENT_GUIDE.md` regen) passes

### Requirement: Formula-count repeated effect choices
The DSL SHALL provide a control-flow step for effects that repeat a player-visible modal choice a formula-resolved number of times, binding each chosen mode before resolving that mode's body. The repeat count SHALL be evaluated once at activation time and negative counts SHALL behave as zero.

#### Scenario: Repeated choice resolves each mode before the next prompt
- **WHEN** a `repeat_effect_choice` step has count `2`, labels for two modes, and a body that may park on nested selections
- **THEN** the engine exposes the first mode choice, resolves that mode's body completely, then exposes the second mode choice and resolves its body

#### Scenario: Source-count modes snapshot at activation time
- **WHEN** a card uses `repeat_effect_choice.count: { floor_div: [{ source_material_count: {} }, 5] }`
- **THEN** the number of repeated choices is based on that source count when the effect activates, even if later mode bodies change the source count

### Requirement: Same-level source-pair predicate
The DSL SHALL provide a carrier-scoped predicate for effects gated by pairs of
same-level cards in the effect carrier's digivolution cards. The predicate SHALL
count only source cards below the top card, SHALL group by printed level, SHALL
ignore non-Digimon or level-less sources, and SHALL fail closed when no carrier
permanent is available.

#### Scenario: Predicate reads source stack levels
- **WHEN** `self_same_level_source_pairs_gte: 1` is evaluated on a Digimon whose sources include two level-4 cards
- **THEN** the predicate is true
- **AND** when the carrier has only one card at each source level, the predicate is false

#### Scenario: EX12-032 prompt is gated
- **WHEN** EX12-032 attacks without two same-level source cards
- **THEN** its trash digivolve prompt is not installed
- **AND** when EX12-032 has a same-level source pair, the optional trash digivolve prompt is exposed through pending selection

### Requirement: Event-target whole-card text predicate
The DSL SHALL provide an event-target analogue of `in_text_contains` for
triggered clauses that gate on whether the digivolving, attacking, played, or
otherwise event-targeted card has a token in its whole printed identity. The
predicate SHALL scan the same broad whole-card surface as `in_text_contains`
(name, aliases, traits, and printed text), SHALL work for both live permanents
and deleted-object snapshots, and SHALL fail closed when the trigger has no
event target card.

#### Scenario: Event-target whole-card text matches attacking Digimon text
- **WHEN** an `on_ally_attack` clause uses `event_target_in_text_contains: Gammamon`
- **THEN** an attacking Digimon with `[Gammamon]` in its text satisfies the predicate
- **AND** an attacking Digimon without that whole-card text token does not

#### Scenario: Hiro-style attack trigger is masked off for nonmatching attackers
- **WHEN** EX12-066 observes one of your Digimon attacking
- **THEN** the optional Hiro trigger is exposed only when the attacking Digimon has `[Gammamon]` in its text or the `[VB]` trait
