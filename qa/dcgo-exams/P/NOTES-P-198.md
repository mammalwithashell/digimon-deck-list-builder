# NOTES — DemiDevimon (P-198)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids P-198`):
**4 clauses** — `effect#0`, `effect#1`, `inherited#0`, `inherited#1`.
DCGO script: `P/Purple/P_198.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 4 clauses, 4 scenarios, 0 unreachable.** All four were audited and
re-lowered unchanged on 2026-09-18 with `tm_p_198_pool.json` (decks
`tm-demidevimon`, `quiet-opponent`). None digivolves into BT25-085 and none
resolves a `<De-Digivolve>`, so the campaign-wide re-lowering findings do not
apply.

| Clause | Text | Scenario |
|---|---|---|
| `P-198#effect#0` | `[Digivolve] Lv.2 w/[TS] trait: Cost 0` | `P-198-effect0.yaml` |
| `P-198#effect#1` | `[Start of Your Main Phase] If you have 4 or less memory, this Digimon may digivolve into a Digimon card with the [Fallen Angel] or [TS] trait in the hand without paying the cost.` | `P-198-effect1.yaml` |
| `P-198#inherited#0` | `[When Attacking] [Once Per Turn]` (the extractor's timing half; empty text) | `P-198-inherited0.yaml` |
| `P-198#inherited#1` | `<Draw 1> and trash 1 card in your hand.` (keyword + rider half) | `P-198-inherited1.yaml` (same line) |

`inherited#0` / `inherited#1` are ONE printed sentence the extractor splits at
the `<Draw 1>` keyword; the two files run the same line and differ only in
`clause:`.

## Adversarial pre-Unity review (from `P_198.cs`)

- `[Start of Your Main Phase]`: `SetUpActivateClass(…, -1, true, …)` →
  `OptionalSkill`, then `DigivolveIntoHandOrTrashCard(isHand: true, payCost:
  false)` → `SelectHandEffect` (`canNoSelect: true`). Ours folds the effect-level
  `optional` into ONE declinable `select_hand`, so the pick row carries
  `expect: OptionalSkill` — the documented `optional_gate_fold` (wire:
  `OptionalSkill(yes)` + the pick).
- **DCGO's `CanActivateCondition` does not look at the hand**: it is
  `IsExistOnBattleAreaDigimon && IsOwnerTurn && MemoryForPlayer <= 4`. So DCGO
  opens the `OptionalSkill` on EVERY own main phase at 4-or-less memory while
  DemiDevimon is the top card on the battle area, even with no eligible card in
  hand. Every line keeps that window shut except on the clause turn: T5 opens on
  7 memory (p1's Phoenixmon overshoot), and after T7 DemiDevimon is a
  digivolution card (the effect is not inherited, so it is not collected).
- Inherited `[When Attacking] [Once Per Turn]`: `isOptional: false` → no
  `OptionalSkill`; `DrawAndDiscardCards` draws, then ONE `SelectHandEffect`
  (`canNoSelect: false`). Ours `draw` + a plain `select_hand`. 1:1.
- Devimon BT2-074's only texts are `OnDestroyedAnyone`-timed
  (`<Retaliation>`); Pagumon BT25-005's inherited observer needs an EFFECT to
  place a digivolution card (normal digivolution does not raise it) — silent.

## FINDING (engine, measured; still present 2026-09-18): the trigger fires from the BREEDING area, ahead of the breeding step

Re-measured by replacing p1's T4 Phoenixmon play with a pass in
`P-198-effect1.yaml`: lowering stops with

```
FAILED: step 8: our engine asks a selection here; the scenario must answer it
with a `select:` step (pending kind: Hand, prompt: 'Digivolve this Digimon into
a Fallen Angel or TS Digimon')
```

Step 8 is p0's T5 **breeding** step (`move: { from: breeding }`): DemiDevimon is
still a Lv.3 in the breeding area and the main phase has not begun. Two things
are wrong at once — a breeding-area Digimon's effect is activating, and a
`[Start of Your Main Phase]` trigger is parked before the breeding phase's own
decision. DCGO gates the trigger on `IsExistOnBattleAreaDigimon`, and asks
nothing here. The committed lines make the trigger's own condition false on
that turn (memory 7) instead of papering over it, so they measure the clause
and not this bug. **Not fixed here**; first seen on the first draft of
`../BT25/BT25-058-effect5.yaml`.
