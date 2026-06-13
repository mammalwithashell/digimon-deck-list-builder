# Archetype DSL Implementation: BT25 orphan-b slice
Date: 2026-06-06
Total cards in slice: 6
Processed this run: 6
Pipeline: batch-implement-cards-rust-dsl

Cards (low->high stage): BT25-011 Aquilamon, BT25-014 Meramon, BT25-034 Angemon,
BT25-036 Craftmon, BT25-037 Pegasusmon, BT25-050 Kiwimon.

## Summary
- IMPLEMENTED: 4 (BT25-011, BT25-014, BT25-034, BT25-037)
- BLOCKED (engine): 1 (BT25-050)
- BLOCKED (dsl): 1 (BT25-036)

## Per-Card Verdicts
| Card ID | Name | Mode | Verdict | Tests | Notes |
|---------|------|------|---------|-------|-------|
| BT25-011 | Aquilamon | IMPLEMENT | IMPLEMENTED | 8 | Raid; OnPlay/WD suspend opp + may DNA into [Silphymon] (hand, `may_dna_digivolve_now` w/ free-bound anchor); inherited [Your Turn] +2000 DP |
| BT25-014 | Meramon | IMPLEMENT | IMPLEMENTED | 8 | [Main][OPT] trash Flame/TS cost → delete <=4000; else Draw 2 (`effect_deleted_any_opponent_digimon:false`); inherited [When Attacking] delete <=4000 |
| BT25-034 | Angemon | IMPLEMENT | IMPLEMENTED | 6 | `scope:security` `on_discard_security` → play Lv4- Angel/Iliad free; `<Ascension>` keyword; inherited `<Barrier>` |
| BT25-037 | Pegasusmon | IMPLEMENT | IMPLEMENTED | 5 | `<Armor Purge>`; OnPlay/WD add top security to hand + may place [Angel]/[Archangel]/[Three Great Angels]/[Iliad] Digimon or [TS] Tamer at top/bottom security (EffectChoice top/bottom) |
| BT25-036 | Craftmon | IMPLEMENT | BLOCKED (dsl) | 0 (ignored stub) | G-DSL-WHEN-LINKED-TIMING — no `when:` timing maps to `CompiledTiming::Linked`; the [When Linking] trash-Appmon→Draw2 clause is unexpressible. Ships no YAML. |
| BT25-050 | Kiwimon | IMPLEMENT | BLOCKED (engine) | 8 pass + 1 ignored | G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED — the count-gated lock `if` step after the suspend selection never runs. Suspend + inherited +1000 DP aura work and are tested green; the conditional lock can't fire faithfully. YAML authored faithfully (count-gated). |

## Engine-Gap Blocked Cards
### BT25-050 Kiwimon
- Effect text: "[On Play][When Digivolving] You may suspend 1 Digimon. Then, if there are 2 or more suspended Digimon, 1 of your opponent's Digimon can't unsuspend until their turn ends."
- Missing engine behavior: a trailing `if` (conditional) step is not executed when the DSL process resumes after a parked interactive selection. An UNCONDITIONAL step after the same selection resumes fine.
- Logged: docs/RUST_ENGINE_GAPS.md (G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED).

## DSL-Vocab-Gap Blocked Cards
### BT25-036 Craftmon
- Effect text (inherited): "[When Linking] By trashing 1 [Appmon] trait card from your hand, <Draw 2>."
- Missing DSL verb/timing: no `when:` timing maps to `CompiledTiming::Linked` (the `Timing` enum lacks a `WhenLinked`/`Linked` variant).
- Lowers to engine API: engine already fires a link-established event (DCGO `EffectTiming.WhenLinked`).
- Suggested DSL syntax: `- when: when_linked`.
- Logged: qa/dsl-vocab-gaps.md (G-DSL-WHEN-LINKED-TIMING).

## Notes / New Idioms Worth Documenting
- `may_dna_digivolve_now` with a non-self `anchor: { binding: <select_own_permanent> }` expresses "2 of your Digimon may DNA digivolve into [X]" where neither material is fixed to the source (BT25-011).
- Activated [Main] effect with a mandatory hand-trash COST must gate activation on `count_gte` of payable cost cards — otherwise the engine's empty-mandatory-select skips the cost yet still runs subsequent steps (an unfaithful free effect). Surfaced on BT25-014.
- A count-gated consequence AFTER an interactive selection is currently non-functional (see G-ENGINE-IF-AFTER-SELECTION-NOT-RESUMED); an unconditional post-selection step works.
