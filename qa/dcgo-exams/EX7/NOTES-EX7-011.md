# NOTES — Megadramon (EX7-011)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-011`):
**3 clauses** — `effect#0`, `effect#1`, `inherited#0`. DCGO script
`EX7/Red/EX7_011.cs` exists — the card is **not** `unavailable`. No verdict is
stored yet (`qa/qa-reports/exam-verdicts/EX7-011.json` does not exist): all
three are `unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-011#effect#0` | `[Digivolve] Lv.4 w/[Three Musketeers] in text: Cost 3` | `EX7-011-effect0.yaml` | lowers, asserts pass |
| `EX7-011#effect#1` | `[On Play] [When Digivolving] By placing 1 Option card with the [Three Musketeers] trait from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's Digimon with 6000 DP or less.` | `EX7-011-effect1.yaml` ([On Play] arm) | lowers, asserts pass |
| `EX7-011#inherited#0` | `<Piercing>` (inherited) | `EX7-011-inherited0.yaml` | lowers, asserts pass |

Book: `qa/dcgo-exams/EX7/tm_ex7_011_pool.json` (decks `tm-megadramon`,
`tm-red-opponent-011`).

## Audit of the crashed agent's three files (resumed stage, 2026-09-18)

All three were on disk, uncommitted. Each was re-read against `EX7_011.cs`
(and, for `inherited#0`, `EX7_059.cs`) and re-lowered against the rebuilt
harness (`d5f23792f`). **No change was needed**:

- None digivolves into BeelStarmon BT25-085, and none resolves a
  `<De-Digivolve>`, so neither campaign heads-up applies.
- Every line keeps each seat on a ONE-Digimon board, so no `field.N` names a
  different Digimon on DCGO's centre-out frames.
- `effect#0`: BlackGatomon (BT25-082, purple/black Lv.4, text names [Three
  Musketeers]) opens the special line alone — the red circle fails on colour —
  so there is one distinct cost and no cost prompt. BlackGatomon's `[On Play]`
  (a [Three Musketeers]-text Tamer in hand) and `[All Turns]` grant (such a
  Tamer on the field) are both inert: P0 never holds a Tamer. Megadramon's
  `[When Digivolving]` is gated on a [Three Musketeers] trait Option in hand or
  trash (`CanActivateCondition`) — none.
- `effect#1`: `isOptional: true` → `OptionalSkill`, authored `dcgo_only`
  because our `select_union_zone … optional: true` pick is gate AND pick (kind
  `UnionZone`, outside the OptionalSkill+pick fold). Only the hand holds a
  candidate → `SetBool(canSelectHand)`, a direct set, no `generic_bool` row.
  `SelectHandEffect(canNoSelect: true)` then the mandatory
  `SelectPermanentEffect(Mode.Destroy, canNoSelect: false)`, which
  `EX7_011.cs` guards with `HasMatchConditionPermanent` (no ≤ 6000 DP opponent
  Digimon → no prompt; `EX7-059-effect3.yaml` relies on that branch).
- `inherited#0`: Megadramon becomes a digivolution card under BeelStarmon ACE
  (EX7-059). The ACE is the **T5 draw**, not an opening-hand card, because
  P1's Biyomon attacks P0's security on T4 with a Lv.5 [Three Musketeers]-text
  Digimon on P0's field: held in hand, the ACE would be a `<Blast Digivolve>`
  candidate there. (On DCGO. Our engine opens no counter timing on a
  player-target attack at all — the engine finding in `NOTES-EX7-059.md` — so
  the draft's caution also keeps this line clear of that gap.) The ACE's
  mandatory `[When Digivolving]` asks nothing (no Option in the trash, no
  [Three Musketeers] trait Option in hand) and its `[When Attacking]` is gated
  on an Option among its own digivolution cards — none.

## Observation recorded for triage (not exercised by these lines)

While probing `P-170#effect#5` with the DUAL card BeelStarmon BT25-085 in hand,
our engine opened Megadramon's `[On Play]` union pick with BT25-085 as the
candidate (a dual card passes `kind: option` + `trait_has: "Three
Musketeers"`). `EX7_011.cs` filters on `cardSource.IsOption &&
ContainsTraits(...)`; whether DCGO's `IsOption` holds for a dual card in hand
was not measured. No committed EX7-011 line holds a dual card, so none of them
can trip on it. See `../P/NOTES-P-170.md`.
