# Archetype DSL Implementation: BT25 link-ts slice
Date: 2026-06-07
Total cards in pool: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

This slice re-evaluated six BT25 link-themed cards (TS / Appmon link payoffs) against the
post–DigiLink-Shape-B substrate (engine commit 5514135c, 2026-06-07). All six had a prior
BLOCKED verdict; per skill Phase 1c, BLOCKED cards are re-attempted (not SKIPped). DigiLink
Shape-B added the player-activated link of a *standing Digimon onto a host* plus the
`kind: link_condition` / `when: when_linked` / `scope: linked` authoring layer — but it did
**not** add an effect-driven "link a card chosen from trash / hand / digivolution-cards to one
of your Digimon" primitive (the deferred residual documented at
`docs/RUST_ENGINE_GAPS.md` §"[Link] subsystem", Shape-B note: "from-hand Digimon-link initiation
and the rarer source origins (trash / under-stack / re-link) are not yet wired"). Every card in
this slice depends on exactly that residual (plus, in two cases, the aura-`<Link +1>` and
App-Fuse gaps). All six remain BLOCKED — no stubs shipped.

## Summary
- IMPLEMENTED: 0
- PARTIAL: 0
- AUDITED-OK: 0
- AUDITED-MISSING-TESTS: 0
- AUDITED-DRIFT: 0
- BLOCKED (engine): 1   (BT25-102)
- BLOCKED (dsl): 1       (BT25-066)
- BLOCKED (hybrid): 4    (BT25-069, BT25-075, BT25-101, BT25-089)
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID   | Name                  | Mode      | Verdict          | Review | Tests | Notes |
|-----------|-----------------------|-----------|------------------|--------|-------|-------|
| BT25-069  | Raremon               | IMPLEMENT | BLOCKED (hybrid) | n/a    | 0/0   | On Play/When Digivolving link 1 [TS] from trash = deferred link-chosen-card primitive |
| BT25-066  | Guardromon            | IMPLEMENT | BLOCKED (dsl)    | n/a    | 0/0   | Leave-replacement by trashing a link card = G-DSL-LINK-TRASH-AS-REPLACEMENT-COST |
| BT25-075  | Vulcanusmon           | IMPLEMENT | BLOCKED (hybrid) | n/a    | 0/0   | Link up-to-2 chosen hand/trash cards + per-link De-Digivolve; aura <Link +1> = G-ENGINE-AURA-GRANT-LINK-MAX |
| BT25-101  | Divine Arms Version Ω | IMPLEMENT | BLOCKED (hybrid) | n/a    | 0/0   | "link 1 [TS] from trash" branch + inherited link-ESS (G-LINK-INHERITED-ESS) + leave-replacement |
| BT25-102  | Factorial Area        | IMPLEMENT | BLOCKED (engine) | n/a    | 0/0   | Conditional <Link +1> security aura = G-ENGINE-AURA-GRANT-LINK-MAX |
| BT25-089  | Kazuki & Itsuki       | IMPLEMENT | BLOCKED (hybrid) | n/a    | 0/0   | Suspend->link 1 [Appmon] from hand/sources (-2) = deferred link-chosen-card; App Fuse = no primitive |

## Engine-Gap Blocked Cards
### BT25-102 Factorial Area
- Effect text: "[Security] [All Turns] All of your Black or Red [TS] trait Digimon gain ＜Blocker＞. While you have [Vulcanusmon], they also gain ＜Link +1＞."
- Missing engine API: aura-granted `ModifierType::ChangeLinkMax` carrying a non-zero value (`G-ENGINE-AURA-GRANT-LINK-MAX`, `qa/archetype-qa/engine-gaps.md`). Auras currently apply ChangeLinkMax with a hardcoded 0.
- Suggested addition: thread an optional `modifier_value` through the aura lowering (`lower_aura.rs`) so numeric modifiers (ChangeLinkMax / ChangeLinkCost) can be granted with a value.

(BT25-075's `<Link +1>` aura clause shares this engine gap; BT25-075 is filed under hybrid because its
primary On Play/When Digivolving link clause is also blocked on the link-chosen-card primitive.)

## DSL-Vocab-Gap Blocked Cards
### BT25-069 Raremon / BT25-075 Vulcanusmon / BT25-101 Divine Arms Version Ω / BT25-089 Kazuki & Itsuki
- Effect texts: link 1–2 cards chosen from trash / hand / digivolution-cards to one of your Digimon (free or with a cost delta), as an [On Play]/[When Digivolving]/[Main] effect.
- Missing DSL verb: an effect-link-chosen-card step (select a card from {hand|trash|digivolution-cards}, attach it as a link to a selected own Digimon, with an optional link-cost delta). The shipping `link_to_own_digimon` links only the *carrier Option*; Shape-B's link-activate only absorbs a *standing Digimon*.
- Lowers to engine API: the (currently missing) effect-driven link-chosen-card primitive over `Permanent.linked_cards` / `attach_linked_card`; tracked at `docs/RUST_ENGINE_GAPS.md` §"[Link] subsystem" facet #9 + Shape-B residual.
- Suggested DSL syntax: a `link_card` step with `from: {hand|trash|digivolution_cards}`, `filter:`, `to:` (own Digimon selection), `count:`, and `cost_delta:`/`free:`.

### BT25-066 Guardromon (and the inherited leave-replacement on BT25-101)
- Effect text: "[All Turns] When this Digimon would leave the battle area, by trashing 1 of its link cards, it doesn't leave."
- Missing DSL verb: a would-leave replacement whose optional cost is "trash 1 of this permanent's own link cards" (`G-DSL-LINK-TRASH-AS-REPLACEMENT-COST`). No `from: linked_cards` for a replacement `choose:` / no `select_linked_card` + `trash_linked_card` pair.
- Lowers to engine API: `when_would_leave` replacement + a trash-own-link-card cost over `Permanent.linked_cards`.

## App-Fuse Gap (BT25-089)
- Effect text: "[End of Your Turn] [Once Per Turn] 1 of your Digimon may app fuse into a Digimon card in the hand."
- Missing engine primitive: App Fuse keyword/mechanic (no `app_fuse` engine path; see `docs/RUST_ENGINE_GAPS.md` App Fuse entry). Shared across the BT25 Appmon package.

## New Patterns Discovered
- None — all gaps are pre-existing and already tracked; BT25-075's tracker entry was newly added this run.
