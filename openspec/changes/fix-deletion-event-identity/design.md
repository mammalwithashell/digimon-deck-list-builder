## Context

The reproduction uses real DSL cards: BT26-016 Chronomon: Holy Mode deletes EX12-031 MarineBullmon during its When Attacking effect. MarineBullmon has an EX12-026 Shellmon source. EX12-070 Sanmyojin Arrival observes the impending leave. The current cause inference treats any deletion during an attack as battle deletion, so Decode is initially excluded. Arrival's hand reward adapter claims the removal as handled, trashes itself, then restarts the subject's deletion as OwnEffect before offering its reward. Whether Decode appears in that second dispatch depends on the compacted subject slot colliding with `replacement_fired`.

CR 4.3 sections 15-8-5 and 16-36 establish immediate interruption, once per interrupting cause, and Decode's non-battle condition. Arrival's printed text and the DCGO EX12_070 coroutine both resolve its self-trash and reward without independently performing the observed removal. Existing typed replacement windows already retain stable subjects, original causes, destinations, remaining candidates, and parked body state.

## Goals / Non-Goals

**Goals:**

- Preserve effect versus battle provenance through all covered deletion routes.
- Finish nested Delay processing inside the original immediate window before committing the original removal.
- Make replacement suppression independent of field indices and safe across clones and replayed cards.
- Demonstrate legal order and decline branches with native masked actions, including a failing test before production edits.

**Non-Goals:**

- Changing action/tensor contracts, search budgets, live training, or deck lists.
- Redesigning every immediate-effect dispatcher rule or claiming all outstanding prevention/order limitations solved.
- Treating historical raw action prefixes as valid replays after prior timing corrections.

## Decisions

### Use mutation provenance instead of ambient attack phase

Effect-driven deletion must use its effect source/controller; actual battle resolution uses explicit Battle provenance. A pending attack or security resolution is insufficient evidence that a nested effect's deletion is battle destruction. Review resumed selections and mixed-owner batch normalization so the original controller remains available. Merely swapping one branch in inference without auditing call sites would leave ambiguous continuations.

### Resume the owned replacement window

Remove the hand/union Delay adapters' manual `handled` plus `finish_owned_leave(OwnEffect)` workaround. Resolve successful self-trash and optional reward as the immediate effect's body, then use the existing parked-window machinery to continue remaining effects and the original commit. Preserve the original cause, destination, subject incarnation, and consumed candidates. This also preserves return-to-hand events instead of converting them into deletions.

A stable guard alone would suppress the accidental second Decode without restoring its first legal opportunity. A second dispatch with the original cause would still lose window ordering/consumption state. The coupled repair is required.

### Stable identity and interrupting-event scope

Permanent suppression/commit keys must identify the battle-area incarnation, not a numeric slot; card/player subjects retain their existing identity. Digivolution and field compaction must preserve that incarnation, while a card leaving and re-entering has a new one. A resumed commit uses the original interrupting event's guard. Independent deletion attempts must not inherit another event's consumed activations; use existing window ownership where sufficient and an explicit owned event identity where necessary.

Fresh deletion API calls use a panic-safe save/restore scope for the fired ledger, while commit/redirect continuations retain their existing guard. A newly resolving effect body clears the continuation marker for its own nested deletion. The owned window retains remaining candidates across parking and reconstructs its completed timing stamps at commit, so no global event nonce is needed.

Keep activated/declined candidates distinct from temporarily ineligible processing conditions. CR 15-8-5-5-2 permits a previously ineligible immediate effect to become eligible within the same window; the repair must not replace reevaluation with a blanket permanent blacklist.

## Risks / Trade-offs

- Shared Delay adapters serve other cards: run their focused tests and full replacement/selection suites, preserving successful-payment gates and optional rewards.
- Nested effects can compact fields or create new permanents: assert physical identities, final zones, prompt masks, and cloned continuation equivalence.
- Existing dispatcher tests document broader cross-timing suppression and prevention limitations: keep them explicit and avoid claiming this work closes unrelated rules gaps.
- Correct timing changes previously asserted prompt order: rewrite the real-card regression to legal timing while retaining a separate genuine nested-continuation witness.

## Migration Plan

Ship a scoped follow-up PR from the merge of PR #752. Validate focused regressions, affected engine suites, strict OpenSpec validation, and repository Tier 0/CI. No deployed training image or run changes are part of this work. Engine changes can be reverted as one scoped patch if verification reveals a regression.

## Open Questions

No product or user decisions are required. Existing owned windows plus scoped deletion guards provide event isolation without an additional global token; focused nested-event regressions verify that boundary.
