# NOTES — AvengeKidmon (P-170)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids P-170`):
**6 clauses** — `effect#0` … `effect#5`. DCGO script `P/Red/P_170.cs` exists —
the card is **not** `unavailable`. No verdict is stored yet
(`qa/qa-reports/exam-verdicts/P-170.json` does not exist): all six are
`unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `P-170#effect#0` | `[Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 4` | `P-170-effect0.yaml` | lowers, asserts pass |
| `P-170#effect#1` | "When this card would be played, by returning 3 cards with [Three Musketeers] in their texts … reduce the play cost by 6." | `P-170-effect1.yaml` | lowers, asserts pass (repaired, below) |
| `P-170#effect#2` | `<Raid>` | `P-170-effect2.yaml` | lowers, asserts pass |
| `P-170#effect#3` | `<Blocker>` | `P-170-effect3.yaml` | lowers, asserts pass |
| `P-170#effect#4` | `<Retaliation>` | `P-170-effect4.yaml` | lowers, asserts pass |
| `P-170#effect#5` | `[On Deletion] You may play 1 [Three Musketeers] trait Digimon card with a play cost of 12 or less from your hand or trash …` | `P-170-effect5.yaml` | lowers, asserts pass |

Book: `qa/dcgo-exams/P/tm_p_170_pool.json` (decks `tm-avengekidmon`,
`tm-red-opponent`).

## Audit of the crashed agent's six files (resumed stage, 2026-09-18)

All six were on disk, uncommitted. Each was re-read against `P_170.cs` and
re-lowered against the rebuilt harness (`d5f23792f`). None digivolves into
BeelStarmon BT25-085 and none resolves a `<De-Digivolve>`, so neither campaign
heads-up (`../BT25/NOTES-BT25-083.md` cost row, `../BT21/NOTES-BT21-074.md`
`IDegeneration` count row) applies. One file needed a repair:

### `effect#1` — two-Digimon board (slot-addressing artefact, pre-empted)

The draft had P1 field **Biyomon + Dracomon** and attack with `field.0` /
`field.1`. DCGO seats Digimon centre-out (`CardSource.PreferredFrame`: frames 4,
3, 5, …) and `attack: field.N` reaches it as a COMPACT frame-order index, so on
a two-Digimon board `field.0` names the OTHER Digimon there — the artefact that
cost `EX7-070#effect#2`, `P-180#effect#2` and `BT25-083#inherited#0` an oracle
run each (`../BT25/NOTES-BT25-083.md`: "never address a slot on a TWO-Digimon
board"). After the T6 attack the projected `suspended` flags would have
differed (ours: Biyomon suspended; DCGO: Dracomon). Repaired by fielding **two
copies of Biyomon** (`play: … from: hand.0` twice — two copies in one zone are
ambiguous to the lowering pass until pinned): whichever one DCGO suspends, the
projected field multiset is identical. Arithmetic re-derived (T2: 3 → 1 → −1;
P0 opens T3 at 1).

### Prompt-shape facts the six lines rest on (re-derived from the C#)

- **Cost prompt.** Megadramon (red Lv.5, "[Three Musketeers]" in text) opens
  BOTH of AvengeKidmon's routes (Red Lv.5 / 5 and the special / 4), two DISTINCT
  costs → `CardController.cs` ~720 "Which digivolution cost do you pay?"
  `SelectCountEffect` (answer is the VALUE, 4). Ours: `EffectChoice`.
- **`effect#1`.** `SetUpActivateClass(..., -1, false, ...)` at
  `EffectTiming.BeforePayCost` — no `OptionalSkill`. ONE
  `SelectCardEffect(Root.Trash, maxCount 3, canEndNotMax: false)`; `canNoSelect`
  is true only while the unreduced cost is affordable (`PayingCost <=
  MaxMemoryCost = 10 + memory`), which it is at memory 3. The click order IS the
  deck-bottom order. Our `cost_reduction … optional: true` parks an
  accept/decline gate first → that row is `sim_only`.
- **`effect#2`.** `RaidSelfEffect`: `OptionalSkill` + `SelectPermanentEffect`,
  authored as the documented OptionalSkill+pick fold.
- **`effect#4`.** On the battle deletion BOTH `OnDestroyedAnyone` effects are
  collected, but `MultipleSkills.cs` keeps only candidates whose `CanActivate`
  holds (`skillInfos_active`, ~262) and short-circuits at one: the `[On
  Deletion]` has no candidate (no trait Digimon in hand; the trash holds
  Megadramon + the 13-cost AvengeKidmon), so `<Retaliation>` resolves alone with
  no prompt.
- **`effect#5`.** `isOptional: true` → `OptionalSkill`; hand is the only
  populated zone → `SetBool(canSelectHand)` (a direct set, no `generic_bool`
  row); `SelectHandEffect(canNoSelect: true)`; `PlayPermanentCards(payCost:
  false)`.

## ENGINE-SIDE FINDING — a DUAL card is offered (and mis-played) by `[On Deletion]`

Re-measured 2026-09-18 on the current engine (probe: `P-170-effect5.yaml` with
BeelStarmon **BT25-085** in hand instead of EX7-073, deck book copy with
BT25-085 swapped in):

- **DCGO** never offers it. The candidate filter requires
  `cardSource.HasPlayCost && GetCostItself <= 12`; BT25-085 is a DUAL card whose
  Digimon face prints play cost "D" ("can't play to the field"), and
  `CEntity_Base.HasPlayCost` (`!cardKind.Contains(Option) && PlayCost >= 0`,
  CEntity_Base.cs:340) is false for it. `CanPlayAsNewPermanent` is never
  reached.
- **Our engine** offers BT25-085 in the union hand+trash pick (the YAML filter
  is `kind: digimon` + `trait_has: "Three Musketeers"` + `play_cost_lte: 12`; a
  dual card passes `kind: digimon`, and `cards.json` carries 6 as its cost)
  and, once picked, runs the OPTION face: the next prompt is
  `OppField` "Delete 1 of your opponent's highest level Digimon" — Fly Bullet's
  `[Main]`. A free "play 1 Digimon card" became a free Option use.

Class: **our bug** (card filter / dual-card play routing), not a P-170 text
issue alone — any "play 1 … Digimon card … without paying the cost" effect
whose filter admits a dual card has the same exposure. Citations: printed
BT25-085 "Play cost: D" (`data/card_bundles/BT25-085.md`); `P_170.cs` "On
Deletion" `CanSelectCardCondition`. **Not fixed here** (exam stage: scenario /
NOTES / book files only). The committed `effect#5` line keeps every dual card
out of P0's hand and trash, so it measures the clause, not this finding.

Same probe, second observation: with BT25-085 in hand our engine ALSO opens
Megadramon (EX7-011)'s `[On Play]` union pick (a dual card counts as "an Option
card with the [Three Musketeers] trait"). `EX7_011.cs` filters on
`cardSource.IsOption && ContainsTraits(...)`; whether DCGO's `IsOption` is true
for a dual card in hand was not measured. Recorded for the EX7-011 triage, not
a P-170 matter.
