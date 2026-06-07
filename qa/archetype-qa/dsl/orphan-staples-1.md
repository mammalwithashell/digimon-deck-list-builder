# Archetype DSL Implementation: orphan-staples-1 (BT25 slice)
Date: 2026-06-06
Total cards in pool: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

Slice cards (low→high stage): BT25-001, BT25-004, BT25-005, BT25-007, BT25-010, BT25-045.

## Summary
- IMPLEMENTED: 2
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 4
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Review | Tests | Notes |
|---------|------|------|---------|--------|-------|-------|
| BT25-001 | Tokomon | IMPLEMENT | IMPLEMENTED | self | 7/7 | Inherited [When Attacking][OPT] TS-gated Draw 1. |
| BT25-010 | Hawkmon | IMPLEMENT | IMPLEMENTED | self | 10/10 | Digivolve-into-trait (Avian/Bird/Beast/Animal/Sovereign, not Sea Animal) cost −1 (self-source only) + 2 cost-0 alt paths (TS / Poromon) + inherited [Your Turn] +2000 DP. |
| BT25-004 | Tapmon | IMPLEMENT | BLOCKED (engine) | — | 0/0 | Inherited `WhenWouldLink` ActivateClass "you may reduce link cost by 1" — no would-link cost-reduction ActivateClass primitive. |
| BT25-005 | Pagumon | IMPLEMENT | BLOCKED (engine) | — | 0/0 | `OnAddDigivolutionCards` trigger ("when [Three Musketeers] placed in this Digimon's digivolution cards") — no such timing. |
| BT25-007 | Gatchmon | IMPLEMENT | BLOCKED (engine) | — | 0/0 | On Play reveal-3 + 2-bucket add IS expressible, but self-link-condition + `WhenLinked` "delete ≤3000 DP" (its printed inherited slot) belong to the [Link] subsystem gap; can't ship faithfully without them. |
| BT25-045 | Onmon | IMPLEMENT | BLOCKED (engine) | — | 0/0 | `WhenWouldLink` ActivateClass "may reduce link cost by 1" + `WhenLinked` "suspend 1 opp Digimon" — [Link] subsystem gap. |

## Engine-Gap Blocked Cards

### BT25-004 Tapmon
- Effect text: "Inherited Effect [Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game] trait card would link to this Digimon, you may reduce the cost by 1."
- Missing engine API: a `WhenWouldLink`-timing triggered `ActivateClass` (optional, OPT, host==self + linking-card-trait gated) that registers a fixed-cost-time link-cost reducer (DCGO `UntilCalculateFixedCostEffect` + `GrantedReduceLinkCostClass`). The engine's `WhenWouldLink` is a replacement only (cancel/redirect/substitute), fires post-payment, and `ChangeLinkCost` is a static player-scoped modifier, not an optional/host-scoped reactive reduction.
- Routed to: `docs/RUST_ENGINE_GAPS.md` → `[Link]` keyword subsystem entry (facet #10).

### BT25-045 Onmon
- Effect text: "[Your Turn] [Once Per Turn] When a [Social], [Tool] or [Game] trait card would link to this Digimon, you may reduce the cost by 1." (+ inherited "Suspend 1 of your opponent's Digimon" = DCGO `WhenLinked`.)
- Missing engine API: same `WhenWouldLink` cost-reduction ActivateClass (facet #10) + `WhenLinked` self-effect (facet #11).
- Routed to: `docs/RUST_ENGINE_GAPS.md` → `[Link]` keyword subsystem entry.

### BT25-007 Gatchmon
- Effect text: "[On Play] Reveal the top 3 cards of your deck. Add 1 [Appmon] trait card and 1 [Social], [Tool], [Reboot] or [Creation] trait card among them to the hand. Return the rest to the bottom of the deck." (inherited slot = DCGO `WhenLinked` "Delete 1 of your opponent's Digimon with 3000 DP or less"; also `AddSelfLinkConditionStaticEffect`.)
- Missing engine API: self-link-condition + `WhenLinked` self-effect (facet #11) and the broader [Link] subsystem. The On Play reveal-add is fully expressible (reveal_top_deck → select_reveal_buckets 2 buckets → add_to_hand_from_reveal ×2 → order_remainder deck_bottom) — but shipping only that and dropping the link condition + WhenLinked delete violates no-approximations.
- Routed to: `docs/RUST_ENGINE_GAPS.md` → `[Link]` keyword subsystem entry.

### BT25-005 Pagumon
- Effect text: "Inherited Effect [Your Turn] [Once Per Turn] When [Three Musketeers] trait cards are placed in this Digimon's digivolution cards, it may digivolve into a Digimon card with [Three Musketeers] in its text or the [TS] trait in the hand with the cost reduced by 2."
- Missing engine API: `OnAddDigivolutionCards` trigger timing (fires when sources are placed under a permanent; gate on host==self + added-card trait). DSL only has the opposite `OnDigivolutionCardTrashed`.
- Routed to: `docs/RUST_ENGINE_GAPS.md` → new "OnAddDigivolutionCards trigger timing" entry.

## DSL-Vocab-Gap Blocked Cards
(none — all blockers are engine-level)

## New Patterns Discovered
- Digivolve-into-target-trait cost reduction with a trait-exclusion veto (`none_of: [Sea Animal]` inside `cost_target`) — BT25-010. Confirms `cost_target` + `none_of` compose for "X but not Y" target gates. Already covered by the BT21-011 pattern; BT25-010 is the first with an explicit exclusion.
