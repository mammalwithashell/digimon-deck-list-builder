## Context

ST-2 Cocytus Blue is a small but useful starter-deck target for the Rust DSL engine. The local card database already contains JSON metadata for `ST2-01` through `ST2-16`, and `ST2-13` Hammer Spark is already promoted to production DSL with behavioral tests. The other ST2 cards remain metadata-only, so they are not present in the implemented-card registry and cannot form a fully verified starter deck for training, deck tools, or game simulation.

The starter deck is also a good substrate probe. Its cards are simple enough to finish in one change, but they exercise reusable blue behaviors: bottom-source trash from opponent Digimon, "no digivolution cards" predicates, bounce to hand, playing a selected source as a Digimon, inherited battle-only DP modifiers, Tamer security play, attack/block restrictions, and once-per-turn unsuspend.

The repository's no-approximations policy applies: every printed player choice must surface through pending selections/action masks, and non-choice printed effects must not create artificial choices merely because an existing generic selector can express the mutation.

## Goals / Non-Goals

**Goals:**

- Represent the official English/Worldwide ST-2 Cocytus Blue deck composition as a verified deck artifact.
- Promote all missing ST2 cards to production Rust DSL YAML with behavioral tests.
- Add reusable DSL/engine substrate only where current primitives cannot express printed ST2 text faithfully.
- Reconcile gap trackers so stale claims, especially around opponent source selection, do not mislead future archetype work.
- Preserve existing tensor and action contracts unless a separate contract-change proposal is opened.

**Non-Goals:**

- Do not implement Korean-only ST-2 promo additions.
- Do not change `ACTION_SPACE_SIZE`, observation profiles, or model metadata as part of this change.
- Do not add legacy Python card scripts.
- Do not use no-op placeholders, hidden auto-selections for real choices, or `raw_rust` escapes to claim ST2 readiness.
- Do not broaden this into a general blue starter/deck-library cleanup beyond ST-2.

## Decisions

### Use Card-Text First, Then ST-2 Deck List as Composition Evidence

Printed effects in `data/cards.json` remain the authority for card behavior. The public ST-2 product page/wiki data is used only for deck composition: `ST2-01 x4`, `ST2-02 x4`, `ST2-03 x4`, `ST2-04 x4`, `ST2-05 x4`, `ST2-06 x2`, `ST2-07 x4`, `ST2-08 x4`, `ST2-09 x4`, `ST2-10 x2`, `ST2-11 x2`, `ST2-12 x4`, `ST2-13 x4`, `ST2-14 x4`, `ST2-15 x2`, `ST2-16 x2`.

Alternative considered: derive the deck from existing scraped `deck_library.json`. That file currently contains only incidental ST2 inclusions in later decks, not the starter product itself.

### Separate Deck Availability from Card Faithfulness

The deck artifact should be valid only when every card ID is known and deck-size rules pass. Full simulation readiness requires every unique card in the deck to be in the Rust implemented-card registry via production DSL YAML and passing behavioral tests.

Alternative considered: add the deck list first and allow unimplemented cards. That would help deck parsing but would create a trap for gauntlet/training code that filters by `load_implemented_card_ids()`.

### Add No-Choice Bottom-Source Trash Instead of Reusing Source Selection

ST2-03, ST2-06, and ST2-09 say to trash the bottom digivolution card(s) under an opponent Digimon. The player chooses the opponent Digimon, but does not choose which source cards are trashed. The DSL should expose a no-choice step such as `trash_bottom_sources: { target: <binding>, count: N }`, routing each bottom source to its owner's trash in bottom-up order.

Alternative considered: `select_opponent_sources` with `is_bottom_source` filters. That would surface a source-selection prompt, which is an artificial player choice for these ST2 cards and violates the no-approximations policy.

### Reuse Existing Source-Play Substrate for Kaiser Nail If It Is Faithful

ST2-15 should select one source card under one of the controller's Digimon and play it without paying the cost. The current `select_material` / `play_from_materials` substrate appears close to this printed text. Implementation should prove it with ST2-15 tests before adding vocabulary. If it cannot preserve ownership, source removal, on-play behavior, and no hidden choices, add the narrowest reusable source-play requirement under `dsl-card-scripting-vocabulary`.

Alternative considered: write a bespoke `kaiser_nail` raw Rust function. That would finish one card but miss the reusable capability that other source-play cards need.

### Model "No Digivolution Cards" as Source Count Zero

For battle-area Digimon, "no digivolution cards" means the permanent has exactly one card in its stack: the top card and no source cards. ST2-08, ST2-12, and ST2-14 can use existing permanent predicates if they correctly evaluate `stack_size_lte: 1` or equivalent. ST2-01 is different because it is battle-contextual: the inherited source grants DP only while battling an opponent Digimon that has no sources. This needs a battle-context predicate or equivalent aura scope that evaluates the current opposing battler, not a broad board-state predicate.

Alternative considered: approximate ST2-01 as active whenever the opponent controls any no-source Digimon. That would over-buff unrelated battles and security checks.

### Prefer Pure DSL and Enabled Tests for All ST2 Cards

Each ST2 card should get a behavioral test file or be covered in grouped tests that assert card data, effect shape, and runtime behavior. Vanillas still need registry/YAML coverage so the starter deck is fully implemented. `ST2-13` remains the reference for simple Option main/security effects and should not be rewritten unless tests reveal a real mismatch.

Alternative considered: skip vanillas because they have no effects. That would leave them out of `load_implemented_card_ids()` unless placeholder YAML exists, blocking a fully simulatable ST-2 deck.

## Risks / Trade-offs

- **Risk: stale gap trackers conflict with code reality** -> Mitigation: verify each cited gap against `code/digimon-dsl/src` and `code/digimon-engine/src` before updating docs; do not trust tracker names alone.
- **Risk: bottom-source trash overlaps existing source-trash soft-fail rules** -> Mitigation: extend `source-trash-soft-fail` so the new primitive inherits the no-panic, no-error behavior of `trash_card_source`.
- **Risk: ST2-01 battle-context predicate expands beyond a small starter-deck need** -> Mitigation: define it generically but narrowly: it may inspect the current battle opponent/source-count context only during battle/combat DP calculation.
- **Risk: Kaiser Nail reveals a deeper source-play gap** -> Mitigation: make proving existing `select_material` / `play_from_materials` the first task, and only add vocabulary if a failing test demonstrates the missing reusable behavior.
- **Risk: deck artifact location is ambiguous** -> Mitigation: follow the existing deck-library/test-deck conventions discovered during implementation and add tests at the consuming boundary.
