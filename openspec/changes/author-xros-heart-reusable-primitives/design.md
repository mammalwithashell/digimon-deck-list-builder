## Context

The `close-xros-heart-digixros-gaps` change closed the base DigiXros
transaction substrate: recipe material selection, cost reduction, post-payment
source attachment, transaction-scoped material zones, pre-attached materials,
and recipe-filtered Material Save. Current Xros Heart deck lists still depend
on effects around that transaction. The repeated shapes are cards stored under
Tamers, cards played from under Tamers, leave-battle source rescue, wildcard
DigiXros requirements, and effect-created attack windows.

The implementation must preserve the no-approximations policy. Any gameplay
choice that matters to a player must flow through pending selections and action
masks. This change must not expand `ACTION_SPACE_SIZE` or change active tensor
contracts.

## Goals / Non-Goals

**Goals:**

- Add reusable Rust primitives for card flow into, out of, and through Tamer
  source stacks.
- Add source snapshot and source-stack payoff helpers that work beyond the
  recipe-filtered Material Save case.
- Add scoped DigiXros wildcard substitution for cards such as `BT10-111`.
- Add event-driven attack windows for cards that allow a played/digivolved
  Digimon to attack or that attack after effect costs resolve.
- Extend the DSL so the primitives can be authored declaratively and audited
  through behavioral tests.
- Use current Xros Heart competitive cards as acceptance fixtures without
  making "complete every Xros Heart card" the architectural goal.

**Non-Goals:**

- Implement every historical Xros Heart, Blue Flare, Twilight, or Bagra Army
  card in this change.
- Add dedicated action IDs for under-Tamer play, source rescue, or effect
  attacks.
- Rework the base DigiXros transaction already introduced by
  `close-xros-heart-digixros-gaps` except where follow-up hooks are required.
- Add raw Rust placeholders to claim card readiness.

## Decisions

### D1 - Treat under-Tamer cards as a reusable zone family

Selection helpers should model "cards under one or more own Tamers" as a
reusable source family rather than encoding each Tamer card separately. The
same selector should support choosing a Tamer destination, choosing a card
under a specific Tamer, and choosing a card under any own Tamer.

Rationale: `BT21-083`, `BT11-095`, `P-224`, `BT19-090`, `BT21-092`, and
`BT19-061` all depend on this zone family. A shared surface avoids one-off
source-index encodings and makes action-mask coverage auditable.

Alternative considered: author each Tamer stash effect with card-specific Rust.
That would unblock individual cards but repeat zone traversal and invite hidden
auto-picks.

### D2 - Under-Tamer play should reuse play transaction helpers

Playing a selected card from under a Tamer should route through existing
free-play or cost-modified play helpers, with the selected source card removed
from the Tamer stack only when the play succeeds. Effects may declare free
play, fixed cost, or play-cost reduction.

Rationale: These effects are still plays, so on-play timing, action legality,
and failed payment behavior should match normal engine contracts.

Alternative considered: move the card directly to battle area. That bypasses
play timing and would break cards that care about "when played."

### D3 - Generalize snapshot-backed source rescue beyond Material Save

Material Save filters source snapshots through the carrier's DigiXros recipe.
Other Xros Heart cards need sibling behavior, such as "up to 4 [Xros Heart] or
[Blue Flare] Digimon cards from this Digimon's digivolution cards under a
Tamer" or "place all Digimon cards in one of your Xros Heart Digimon's sources
under a Tamer." These should use reusable snapshot and source-move helpers with
explicit filters and counts.

Rationale: The hard part is not Material Save's recipe filter; it is preserving
pre-removal source identity and presenting legal choices before zones mutate.

Alternative considered: treat all source rescue as Material Save variants.
That overfits to recipe-based cards and fails for trait-filtered or all-source
effects.

### D4 - Wildcard DigiXros substitution is a transaction modifier

Cards such as `BT10-111` that can replace one DigiXros requirement should
register a scoped wildcard material substitution against the next or current
DigiXros transaction. The transaction should still validate that the wildcard
card is selected, consumes only one requirement slot, and expires at the printed
duration.

Rationale: Wildcards affect requirement matching, not card identity globally.
Keeping the behavior transaction-scoped avoids leaking fake names or traits to
unrelated effects.

Alternative considered: add synthetic `also_treated_as` names for the turn.
That would be too broad and could satisfy effects that are not DigiXros
requirements.

### D5 - Effect-driven attacks create temporary attack windows

Effects that say a Digimon may attack, or that resolve a cost and then attack a
player with one of your Digimon, should create a pending attack window with
explicit attacker and target prompts as needed. The attack should still pass
through combat legality, blocker, and attack-resolution machinery.

Rationale: These are real attacks, not damage shortcuts. RL agents must see the
attacker and target decisions through the mask.

Alternative considered: call combat directly from effect bodies. That would
hide target choice and skip normal attack hooks.

### D6 - DSL vocabulary follows primitive names, not card names

YAML should expose generic steps such as "place card under Tamer", "select from
under Tamers", "play selected source", "register DigiXros wildcard", "move
sources under Tamer", and "initiate attack". Card names such as Shoutmon or
Taiki belong in predicates, not schema keys.

Rationale: The same primitives should be reusable for Blue Flare, Twilight, and
Bagra Army cards.

Alternative considered: add Xros Heart specific DSL shortcuts. That would be
fast but hard to reuse and harder to audit.

## Risks / Trade-offs

- Under-Tamer source addressing can be fragile if field indexes shift ->
  Prefer handles and card identities internally, and sort removals in an order
  that does not invalidate later picks.
- Effect-driven attacks can accidentally bypass normal timing hooks -> Route
  through the same attack-resolution path used by ordinary attacks after the
  temporary attack window chooses legal actions.
- Wildcard substitution can leak beyond the printed duration -> Store wildcard
  state on the transaction or a scoped turn effect with explicit expiry tests.
- Playing from under Tamers can consume the card before payment failure -> Move
  cards only after the play succeeds, or implement rollback tests for paid
  under-Tamer plays.
- DSL can silently accept unsupported source-flow shapes -> Reject unsupported
  fields with explicit compile errors.

## Migration Plan

1. Add failing behavioral tests for the representative primitive fixtures:
   `BT21-083`, `BT11-095`, `P-224`, `BT19-090`, `BT21-092`, `BT10-111`,
   `BT21-027`, and `BT19-061`.
2. Land under-Tamer selectors and movement helpers first, then verify stash
   placement and under-Tamer play in isolation.
3. Add generalized source rescue and source-count plumbing, then author the
   leave-battle/source-move fixtures.
4. Add DigiXros wildcard substitution and event-driven attack windows.
5. Extend DSL vocabulary in the same order as the engine primitives and migrate
   acceptance fixture cards to production YAML.
6. Update gap trackers and Xros Heart QA notes as each primitive closes.

Rollback: the new helpers should be opt-in through explicit DSL steps or effect
helpers. Reverting a later fixture should not change already-authored DigiXros
or Material Save behavior.

## Open Questions

- Should paid play-from-under-Tamer effects be supported in the first slice, or
  should this change initially cover free play and fixed reductions only?
- Should "under any of your Tamers" selection preserve Tamer grouping in the
  prompt metadata for UI clarity, even if action IDs reuse existing source
  selection ranges?
- Which card should be the first event-driven attack acceptance fixture:
  `BT21-083`'s played/digivolved may-attack hook, or `BT19-090`'s option mode?
