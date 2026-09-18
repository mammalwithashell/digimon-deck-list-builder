# NOTES — Gewalt Schwärmer (BT6-105)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT6-105`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `BT6/Black/BT6_105.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 3 clauses, 3 scenarios, 0 unreachable.** All three lower sim-only
with `tm_bt6_105_pool.json` (decks `tm-gewalt`, `quiet-opponent`), authored
2026-09-18.

| Clause | Text | Scenario |
|---|---|---|
| `BT6-105#effect#0` | `If you have a Digimon with [Three Musketeers] in its type in play, you may use this Option card without meeting its color requirements.` | `BT6-105-effect0.yaml` |
| `BT6-105#effect#1` | `[Main] Delete all Digimon with play costs of 7 or less.` | `BT6-105-effect1.yaml` |
| `BT6-105#effect#2` | `[Security] Add this card to its owner's hand.` | `BT6-105-effect2.yaml` |

## Authoring decisions

- **`effect#0` — the witness is legality.** `tm-gewalt` holds no black card but
  the Option itself and red Koromon eggs that are never hatched, so p0 cannot
  meet the black colour requirement; the only thing p0 has in play is
  **Deputymon EX7-010** (red Lv.4, printed trait `[Mutant]`), which carries
  `[Three Musketeers]` only through its own `[Your Turn]` trait grant. That is
  the cheapest [Three Musketeers] Digimon a legal line reaches (every
  printed-trait carrier is a Lv.6) and it makes both engines read the LIVE
  trait list: DCGO `IgnoreColorConditionClass.CanUseCondition` →
  `TopCard.CardTraits.Contains("Three Musketeers")`, and
  `CardSource.CardTraits` folds in the card's own `IChangeTraitsEffect`
  (`CardSource.cs` ~2603); ours `use_requirement: any_permanent { trait_has }`
  over the `kind: aura` grant. It lowered first time — our engine sees the
  granted trait. Dependency: Deputymon's trait clause is `EX7-010#effect#2`,
  itself still `unmeasured`; if that clause diverges under the oracle, this
  line aborts on its `play:` step and must be re-read in that light.
- **`effect#1`** puts one Digimon on each side of every word: own Deputymon
  (cost 6) and the opponent's Biyomon (cost 2) die TOGETHER, Phoenixmon
  (cost 10) survives.
- **`effect#2`**: standard security skeleton; the witness is BT6-105 in p0's
  hand with an empty trash.

## Adversarial pre-Unity review (from `BT6_105.cs`)

- `[Main]`: `SetUpActivateClass(null, …, -1, false, …)` → no `OptionalSkill`;
  the target list is computed, not picked (`Players_ForTurnPlayer …
  Filter(HasPlayCost && GetCostItself <= 7)` into ONE `DestroyPermanentsClass`)
  → **no prompt at all** after the `play:` row. Ours `delete_all_permanents`.
- Deputymon is HARD-PLAYED in both lines: its `[When Digivolving]` /
  `[When Attacking]` shared effect — the one that makes DCGO ask a vacuous
  `OptionalSkill` on every digivolve/attack (`../EX7/NOTES-EX7-010.md`) —
  never triggers, and it has no `[On Play]`. None of Deputymon / Biyomon /
  Phoenixmon has an `[On Deletion]`.
- `[Security]`: `isOptional: false`, `AddThisCardToHand`, no prompt.
- The overshoot (memory 1 → -6 / 3 → -4) ends p0's turn by itself, so the
  trailing step is p1's breeding prompt, not a p0 pass.
