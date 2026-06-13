# Archetype DSL Implementation: BT25 link-finish-replacement slice
Date: 2026-06-07
Total cards in slice: 3
Processed this run: 3
Pipeline: batch-implement-cards-rust-dsl

These three cards share the `[All Turns] When this would leave the battle area,
by trashing 1 of its link cards, it doesn't leave` leave-replacement mechanic.
All three were prior-BLOCKED; this rerun re-adjudicated them against the
gap-2 / gap-3a / gap-3b link substrate landed on this branch (commits
d21d4d34, 297a00ab, 15c80b3d) plus targeted widening below.

## Summary
- IMPLEMENTED: 2 (BT25-066, BT25-101)
- BLOCKED (dsl): 1 (BT25-073)

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT25-066 | Guardromon | IMPLEMENT | IMPLEMENTED | 8/8 | Blocker + leave-replacement (cost: trash_own_link_card) + inherited +1000 DP + TS Lv3 cost2 alt-digivolve |
| BT25-101 | Divine Arms Version Ω | IMPLEMENT | IMPLEMENTED | 7/7 | TS link-Option: use_req color-ignore; [Main]/[Security] trash TS → Draw 2 → link self/TS-from-trash; inherited link-ESS Sec A.+1 + Reboot + leave-replacement; link_requirement |
| BT25-073 | Dragomon | IMPLEMENT | BLOCKED (dsl) | 0 | Main-clause activation cost "trash 1 of your Digimon's link cards" has no DSL step (G-DSL-LINK-TRASH-AS-COST). Inherited leave-replacement now expressible, but Main is the defining clause |

## Engine / DSL widening done this slice (rule 28)
To make BT25-101's `scope: linked` inherited link-ESS reach the host (the
substrate's host-side collectors already scan `linked_cards`, but the DSL
lowering never marked these effects `.linked()`):
1. `src/dsl_cards/lower_aura.rs::lower` — emit `.linked()` for `CompiledScope::Linked`
   (the flat `security_attack: 1` / DP self-aura link-ESS → host).
2. `src/dsl_cards/lower_replacement.rs::lower_with_raw` — emit `.linked()` for
   `CompiledScope::Linked` (the leave-replacement as a link-ESS).
3. `src/dsl_cards/lower_replacement.rs::source_permanent_is_still_active` — accept
   the source card residing in the host's `linked_cards` (not only `card_sources`).
4. `src/replacement.rs::collect_candidates` — scan each permanent's `linked_cards`
   for `.linked()` would-* replacement effects (new `push_linked_from_perm`), so a
   link-card leave-replacement fires for the host the card is attached to.

Regression: `cargo test --lib` 208/208; `cargo test --test option_flow` 126/126;
`cargo test --test timing_dispatch` 51/51. (Pre-existing unrelated failure in
`dsl_eval_arm_coverage::step_variants_have_exec_arms` — Digixros/DnaDigivolve
step arms, untouched by this slice.)

## DSL-Vocab-Gap Blocked Cards
### BT25-073 Dragomon  [G-DSL-LINK-TRASH-AS-COST]
- Main clause: `[On Play][When Digivolving] By trashing 1 of your Digimon's link
  cards, you may play or use 1 [TS] cost<=5 card from hand free`.
- Missing DSL verb: an activation-cost step that selects an own Digimon (with
  >=1 link card), selects one of ITS link cards, and trashes it (gating a tail).
  The shipped `trash_own_link_card` is replacement-only (cancels leave, reads
  `replacement_subject`); the `link_cards` family only attaches.
- Lowers to engine API: `Permanent.linked_cards` + `Game::trash_specific_link_card`
  (exists) + standard play/use-free. Only the DSL cost step is missing.
- Full entry + suggested syntax: `qa/dsl-vocab-gaps.md` § G-DSL-LINK-TRASH-AS-COST.
