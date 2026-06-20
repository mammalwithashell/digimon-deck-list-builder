# Design — collapse step idioms

## Guiding principle

Reduce the *steps per intent*, not the expressiveness. Every collapse must keep each player choice a distinct RL-visible `pending_selection` — a composite verb is sugar over the same selection sequence, never an auto-pick. Back-compat: the longhand forms keep working; migration of existing cards is optional.

## D1 — `then:` generalization reuses a shipped mechanism

`then: Vec<StepSpec>` already exists end-to-end on seven select structs (args → `CompiledStep` → lowering). The work is exposing it on the field/zone selectors (`select_*_permanent`, `select_hand`, `select_trash`, `select_security`), not building new machinery. The tail runs with the selection's binding in scope, exactly as it does for `select_own_sources` today. This is the lowest-risk, highest-reach item — do it first.

Composite aliases (`discard_from_hand = select_hand { then: [trash_from_hand_by_index ...] }`, `recover_from_trash`, etc.) are optional thin sugar; gate them on whether they materially out-read the `then`-tail form. Prefer the general mechanism over a proliferation of aliases (that would re-introduce the sprawl we're removing).

## D2 — `reveal_search` is a bucketed composite, not an auto-search

Shape: `reveal_search { of, count, buckets: [{ filter, to: hand|trash|deck, max, optional, prompt }...], remainder: top|bottom|choose }`. Lowers to the exact existing sequence: `reveal_top_deck` → one `select_reveal`/`select_reveal_buckets` per bucket → the appropriate per-bucket move → `place_remainder_on_deck`. Each bucket is an RL-visible pick with its own mask; `optional` per bucket preserves "may". The `no_duplicate_cards` semantics of `select_reveal_buckets` carry over when multiple buckets draw from one pool. Make `add_to_hand_from_reveal` (and siblings) accept a multi-card list so "add up to N" is one bucket with `max: N` (fixes EX5-015's single-card cap).

This complements the `then`-tail (D1): `then` collapses the *select-then-act* half of the pool, `reveal_search` collapses the *reveal-search* half.

## D3 — Security placement: position first, consolidation second

Two moves, independently shippable:
1. **`position: choice`.** Add a `Choice` variant to `StackPosition` (and `compile_stack_position`, `compile.rs:278-285`) that installs a binary top/bottom `pending_selection`, so the `select_effect_choice` + two `if`-equals arms collapse into the placement step. This alone cuts the worst boilerplate (BT25-038 clause A ~100→~20 lines).
2. **Verb consolidation.** Replace the ~6 place-on-security verbs (4 near-identical arg structs at `step.rs:1587-1626` + `place_on_security` at `1587-1595`) with one source-polymorphic verb whose `source:` accepts a hand binding, a material/permanent binding, or self, and whose replacement behavior is a `SecurityReplacementDisposition` enum (`none | cancel | handle | observed`). This fixes the asymmetry (today bottom+cancel exists but bottom+handle doesn't) by making disposition orthogonal to position/source. The replacement-disposition variants remain usable only inside `kind: replacement` clauses (unchanged contract).

Cite the arg structs precisely: `PlaceOnSecurityArgs` (1587-1595), `PlacePermanentSecurityReplacementArgs` (1599-1603), `PlacePermanentOnSecurityReplacementArgs` (1605-1614), `PlacePermanentOnSecurityObservedArgs` (1616-1626).

## D4 — `link_card_to_self` migration gating

Do not delete before the successor covers the cases. First confirm `link_cards` expresses single-card self-host and chosen-host links (close `G-DSL-LINK-N-CARDS-PER-HOST` / `G-DSL-LINK-FROM-ANY-OWN-DIGIMON-SOURCES` if a residual exists). Then migrate the 11 cards (ST22-12, BT21-023/073/101, BT25-052/056/060/069/070/072/089) with per-card parity tests, then delete the verb + lowerer. The deprecation note (separate change) discourages new uses in the interim.

## D5 — RL contract

All four items are additive over the existing selection machinery:
- `then`-tail: the tail's selections are normal `pending_selection`s (already true for the seven structs that have it).
- `reveal_search`: each bucket is an existing reveal-selection.
- `position: choice`: one ordinary binary selection.
- link migration: behavioral parity, no new selection shapes.
No action-space size or tensor change is expected. Add a guard test that the action-mask/tensor for a representative collapsed card matches its longhand equivalent.

## Risks

- **`then`-tail binding scope** must match the seven-struct precedent exactly (the tail sees the selection binding). Mitigate by mirroring the existing lowering, not re-deriving it.
- **Security consolidation blast radius**: the place-on-security verbs touch replacement flows. Keep the replacement-only verbs' contract intact; the consolidation is a re-parametrization, validated by behavioral parity on every current user before deleting the old verbs.
- **`reveal_search` corner cases** (empty pool, all-optional buckets declined, remainder ordering) must match the hand-rolled idiom; port the existing reveal tests onto the composite.
