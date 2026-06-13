# Archetype DSL Implementation: thomas-data-squad (BT25)
Date: 2026-06-06
Total cards in slice: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

## Summary
- IMPLEMENTED: 2 (BT25-002, BT25-027)
- BLOCKED (engine): 4 (BT25-087, BT25-096, BT25-029, BT25-104)
- BLOCKED (dsl): 0
- BLOCKED (hybrid): 0
- SKIPPED (prior verdict): 0

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT25-002 | Wanyamon | IMPLEMENT | IMPLEMENTED | 5/5 | Inherited [Your Turn][OPT] on DATA SQUAD Tamer play, both players draw 1 |
| BT25-027 | MachGaogamon | IMPLEMENT | IMPLEMENTED | 5/5 | Alt-digivolve; WD/WA bounce+trash-FD-unsuspend; main+inherited leave-prevention |
| BT25-087 | Thomas H. Norstein | IMPLEMENT | BLOCKED (engine) | 0 | OnAddToHand inert + BeforePayCost-Parked |
| BT25-096 | Mirage Beast Knight | IMPLEMENT | BLOCKED (engine) | 0 | BeforePayCost selection-bearing pay_cost Parked-drop |
| BT25-029 | MirageGaogamon | IMPLEMENT | BLOCKED (engine) | 0 | OnAddToHand trigger inert |
| BT25-104 | ShineGreymon: Burst Mode | IMPLEMENT | BLOCKED (engine) | 0 | Option cross-side Main activation + treated-as-Digimon aura |

## Verification note
The shared `cards_behavioral` test binary was repeatedly uncompilable during this
run because several concurrent BT25 slices (orphan-b/orphan-c, bt25_014/034/050,
bt25_067) were mid-flight with broken WIP test files and the test-discovery
`mod.rs` was clobbered multiple times by concurrent agents. The two IMPLEMENTED
cards were therefore verified via isolated standalone test targets
(`tests/zz_thomas_*_standalone.rs`, since removed) that exercised the identical
DebugRunner logic and ALL PASSED (BT25-002 5/5, BT25-027 5/5). The canonical
per-card test files are committed at
`code/digimon-engine/tests/cards_behavioral/bt25/bt25_002.rs` and `bt25_027.rs`
and their `mod` declarations are wired in `bt25/mod.rs`; they will run under the
full `cards_behavioral` binary once the concurrent slices' WIP files compile.

## Engine-Gap Blocked Cards

### BT25-087 Thomas H. Norstein — BLOCKED (engine)
- Clause 2: "[All Turns] When effects add cards to your opponent's hand, by
  suspending this Tamer, you may place the top 2 cards of your deck face down
  under this Tamer." → needs the `OnAddToHand` trigger (engine timing variant
  exists but is **never enqueued**; no event-target context for whose-hand /
  by-an-effect). See docs/RUST_ENGINE_GAPS.md "OnAddToHand trigger is inert".
- Clause 3: "[Your Turn][OPT] When any of your Digimon would digivolve into a
  [DATA SQUAD] trait Digimon card, by trashing the bottom face-down card from
  under any of your Tamers, reduce the cost by 1." → the trash-FD-under-Tamer
  pay_cost installs a PendingSelection (Tamer pick) → `before_pay_cost` reducer
  returns `Parked` → reduction silently dropped. See "BeforePayCost cost-reducer
  with selection-bearing pay_cost (Parked-outcome handling)".
- Expressible (not the blocker): clause 1 SoT set-memory-to-3-if-≤2; inherited
  [Security] play-self.

### BT25-096 Mirage Beast Knight — BLOCKED (engine)
- Clause 1: "When this card would be used, by trashing the bottom face-down card
  from under any of your Tamers, reduce the use cost by 2." → same BeforePayCost
  Parked-drop gap (multi-Tamer pick parks; single-Tamer auto would work but the
  multi-Tamer case silently drops the reduction → §17 violation).
- Expressible: [Main] place Gaogamon+MachGaogamon from trash under a Gaomon then
  alt-digivolve into MirageGaogamon in hand free; [Security] play Gaomon/Thomas
  free then add this to hand.

### BT25-029 MirageGaogamon — BLOCKED (engine)
- Clause: "[All Turns][OPT] When effects add cards to your opponent's hand or
  trash cards from under your Tamers, this Digimon may unsuspend." The
  OnDigivolutionCardTrashed half is supported; the **OnAddToHand half is inert**.
  Splitting into only the supported half would silently drop the printed
  add-to-hand reaction (§17). See "OnAddToHand trigger is inert".
- Expressible: Reboot/Blocker/Evade; alt-digivolve onto Gaogamon-name/DATA SQUAD
  cost 3; WD/WA bounce opp lvl≤5 then trash-FD return opp lowest-level.

### BT25-104 ShineGreymon: Burst Mode — BLOCKED (engine)
- DUAL card (Burst Mode Digimon side + Final Shining Burst Option side).
- Clause "[WD][WA][OPT] Activate 1 [Main] effect on this card's Option side"
  needs cross-side Option-Main activation from the Digimon side
  (`activate_own_main_effects` — Option play-flow gap, RUST_ENGINE_GAPS ~L572).
- Clause "[Your Turn] all your [Marcus Damon] are also treated as 12000 DP
  Digimon and gain <Rush>" needs a treated-as-Digimon player-aura targeting
  Tamers by name (the "Also treated as [X]/[Y]" name-overlay + TreatAsDigimon
  via aura is a PARTIAL gap, no precedent).
- Expressible: Raid/Piercing/Security A.+1/Blocker/Barrier keywords; alt-digivolve
  + burst-digivolve; Option-side [Main] -15000 DP then play Tamer free (as a
  standalone Option, if cross-side activation existed).

## DSL-Vocab-Gap Blocked Cards
None — all four BLOCKED cards are gated on engine primitives, not DSL vocabulary.

## Substrate that WORKED (compounding-coverage wins)
- `trash_bottom_face_down_source_under_tamer` verb (Phase A DATA SQUAD stash)
  inside a triggered `process` and inside a `replacement` `process` — both
  correctly run their tail only after the FD source is trashed (BT25-027).
- `alt_paths: [{ kind: digivolve, from: { level_eq, trait_has }, cost }]` for the
  DATA SQUAD alt-digivolve (BT25-027).
- `replacement` + `trigger: when_would_leave_battle_area` + `cancel_replacement`
  with a face-down-trash cost, both main and `scope: inherited` (BT25-027).
- `on_ally_played` + `event_target_kind: tamer` + `event_target_trait_has` +
  `active_when: { your_turn: true }` + `once_per_turn` + dual `draw` (BT25-002).
