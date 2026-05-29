## Context

`close-xros-heart-digixros-gaps` introduced the base DigiXros transaction, and
`author-xros-heart-reusable-primitives` added the first reusable under-Tamer,
source rescue, wildcard, and effect-driven attack primitives. Resolving the live
Xros Heart deck pool shows that the remaining high-frequency cards are no
longer blocked by base DigiXros itself. They are blocked by adjacent authoring
shapes: effect digivolving from cards under Tamers, stack-derived selectors and
formulas, and temporary effects that suppress printed timing hooks or
unsuspension.

The implementation must keep the no-approximations contract. Every gameplay
choice must be exposed through existing pending-selection/action-mask machinery,
and this change must not expand `ACTION_SPACE_SIZE` or alter observation tensor
contracts.

## Goals / Non-Goals

**Goals:**

- Add reusable source-zone effect digivolve helpers for cards like `BT19-008`
  and `BT19-057`.
- Add stack-derived metrics needed by `BT19-014`, `AD1-006`, `AD1-013`,
  `BT19-026`, and `BT21-030`.
- Add temporary effect lockouts needed by `BT19-038`, `BT20-037`, and similar
  cards that prevent timing effects or unsuspension.
- Add reveal-pool free-play routing needed by `BT19-008`-style reveal clauses
  that play a selected revealed Tamer or Digimon without paying its cost.
- Extend YAML DSL vocabulary so the remaining Xros Heart pool can be authored
  without raw Rust placeholders.
- Use representative Xros Heart cards as acceptance fixtures and update gap
  trackers as each primitive closes.

**Non-Goals:**

- Author every card in the resolved Xros Heart pool as part of this substrate
  change.
- Change base DigiXros material selection, cost reduction, or Material Save
  semantics except where source-zone digivolve needs to reuse existing zone
  selectors.
- Add action IDs, tensor fields, PyO3 contract changes, or frontend action
  constants.
- Introduce Xros Heart-specific schema keys when a reusable primitive can cover
  Blue Flare, Twilight, Bagra Army, or future cards.

## Decisions

### D1 - Model source-zone digivolve as effect-initiated digivolution with an origin binding

Effects that digivolve using a card under a Tamer should first select the source
card through the existing under-Tamer source selector, bind both the card and its
origin Tamer, and then invoke the same effect-initiated digivolve helper used
for other effect digivolutions. The selected card is removed from the Tamer only
when the digivolution commits.

Rationale: these effects are true digivolutions, so level/color/trait/path
validation, on-digivolve timing, and source attachment order should come from
the normal digivolution machinery. Binding the origin prevents fragile
index-based removals after pending selections.

Alternative considered: play a copied card or synthesize a hand card before
digivolving. That would distort card identity and bypass the source-zone origin
rules.

### D2 - Add stack-derived metrics as selector/formula primitives

The engine should expose small reusable queries for "fewest source cards",
"source stack has no cards", "distinct colors among source cards", "count colors
among source cards", "count source cards matching a predicate", and "compare
selected target DP to this Digimon's current DP". DSL formulas and predicates
can compose these rather than hardcoding each card's math.

Rationale: the remaining Xros Heart cards repeatedly inspect source stacks, but
the inspected quantity differs by card. A metric layer keeps the behavior
auditable and avoids one-off card predicates.

Alternative considered: encode each fixture card in custom Rust. That would
close card tests but leave the authoring substrate incomplete.

### D3 - Treat temporary lockouts as ordinary status modifiers with timing gates

Temporary lockouts should be represented as expiring modifiers on the affected
Digimon or Tamer. Each modifier declares the suppressed timing family, such as
On Play or When Digivolving, and/or the unsuspend restriction, plus an explicit
expiry such as "until end of opponent's turn".

Rationale: the engine already routes many continuous and temporary effects
through modifier checks. Extending that path keeps lockouts visible to all
trigger collection points instead of adding local checks in specific cards.

Alternative considered: skip trigger execution inside the card that applied the
lockout. That only works for immediate effects and fails when another card would
observe the same locked permanent later.

### D4 - DSL keys describe primitives and reject partial lockout shapes

YAML should expose generic vocabulary for selecting source-zone digivolve cards,
binding stack metrics, using metric formulas in DP/cost/target predicates, and
applying temporary lockouts. Unsupported timing families or source metrics should
fail compilation with explicit errors.

Rationale: silent no-op fields are especially dangerous for card effects because
they can make an archetype appear implemented while hiding a rules decision from
the action mask.

Alternative considered: accept broad free-form strings for timing families and
metrics. That is flexible but makes typo-driven no-ops too likely.

### D5 - Acceptance fixtures lead with high-frequency missing cards

The first implementation slice should target `BT19-008`, `BT19-057`,
`BT19-014`, `BT19-038`, `BT19-051`, `BT19-035`, `AD1-006`, `AD1-013`,
`BT19-079`, `BT19-026`, and `BT21-030`. Lower-frequency tech cards can follow
after the reusable substrate proves out.

Rationale: the resolved pool shows these cards are the largest blockers to
authoring realistic lists. They also exercise the reusable primitives without
making the change a full-archetype completion grab bag.

Alternative considered: author cards strictly by set number. That is tidy but
less useful for surfacing the substrate gaps that actually block deck play.

### D6 - Route reveal-pool free play through the centralized play pipeline

`choose_from_reveal` should support `destination: play_free` for effects that
play one selected revealed card without paying its cost. The chosen revealed
card remains player-selected through the existing reveal pending selection, then
the engine consumes that `CardHandle` from the reveal pool and routes it through
the normal effect-play pipeline.

Rationale: cards like `BT19-008` combine reveal/search flow with a free-play
payoff. Using the centralized play pipeline preserves field-capacity checks,
effect-play floodgates, would-play replacement prompts, On Play dispatch, and
entered-field broadcasts.

Implementation note: the helper uses an internal hand-transit shape only as a
play-pipeline bridge. It does not fire add-to-hand effects, clears reveal
overlays before play, and records a reveal-origin restore point so a declined or
failed would-play replacement can put the card back into `revealed_cards` at the
original reveal index.

## Risks / Trade-offs

- Source-zone digivolve can consume a card before a later legality check fails
  -> Commit source movement only after digivolution legality and payment are
  accepted, and add rollback/no-legal-path tests.
- Stack metrics can drift from live state if cached too early -> Compute metrics
  at selection/formula evaluation time unless a printed effect explicitly needs a
  snapshot.
- Temporary lockouts can accidentally suppress inherited or unrelated timings ->
  Scope timing families explicitly and add negative tests for unaffected triggers.
- "Until end of opponent's turn" expiry can be off by one phase -> Test both the
  locked turn and the next turn after expiry.
- DSL vocabulary can become too generic too fast -> Add only the metrics and
  timing families needed by acceptance fixtures, with compile errors for the
  rest.

## Migration Plan

1. Add failing Rust behavioral tests for representative fixture cards and narrow
   unit tests for each primitive.
2. Implement source-zone effect digivolve using existing under-Tamer selectors
   and effect digivolve helpers.
3. Add stack-derived metric predicates/formulas and wire them into target
   selection and effect math.
4. Add temporary lockout modifiers and trigger/unsuspend gate checks.
5. Extend DSL schema/lowering in the same order as the engine primitives and
   author fixture YAML only after the relevant behavior passes.
6. Update Xros Heart QA deck-pool/readiness notes and gap trackers.

Rollback: the new helpers are opt-in through explicit DSL steps and modifiers.
Reverting a fixture YAML or primitive should not alter already-authored base
DigiXros behavior.

## Open Questions

- Should the first source-zone digivolve slice support only cards under own
  Tamers, or should it also support trash/hand source-card origins immediately?
- Resolved 2026-05-24: source color counting uses distinct colors once per
  color over source cards beneath the resolving effect carrier's top card. A
  multi-color source card can contribute multiple colors, but duplicate colors
  are counted once.
- Should On Play lockouts apply to security-effect plays in the same way as
  normal plays, or do any card-specific rulings require a narrower hook?
