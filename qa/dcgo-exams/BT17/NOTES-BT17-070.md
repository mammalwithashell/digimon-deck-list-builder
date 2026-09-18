# NOTES — Gulfmon (BT17-070)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT17-070`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`. DCGO script
`BT17/Purple/BT17_070.cs` exists — the card is **not** `unavailable`. No verdict is
stored yet: all three are `unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `BT17-070#effect#0` | `[Digivolve] Lv.5 w/[Dark Masters] in text: Cost 3` | `BT17-070-effect0.yaml` | lowers, 6/6 asserts pass (crashed agent's file, audited, unchanged) |
| `BT17-070#effect#1` | `[On Play] [When Digivolving] By placing 1 level 5 card with [Dark Masters] in its text from your hand or trash as this Digimon's bottom digivolution card, delete 1 of your opponent's level 5 or lower Digimon.` | `BT17-070-effect1.yaml` ([On Play] arm, cost from hand) | lowers, 12/12 |
| `BT17-070#effect#2` | `[When Attacking] By returning 7 cards from your opponent's trash to the bottom of the deck, unsuspend this Digimon.` | `BT17-070-effect2.yaml` | lowers, 21/21 |

Book: `qa/dcgo-exams/BT17/tm_bt17_070_pool.json` (deck `tm-gulfmon`, the crashed
agent's, unchanged: the shared list with BT17-070 ×4 and WaruMonzaemon P-216 ×4).

## The one Lv.5 "[Dark Masters]" card available, and what it costs
Every Lv.5 that prints "[Dark Masters]" (BT15-027/050/062/077, BT17-068, P-216) is
outside our authored pool — none has a YAML spec — so our engine treats
**WaruMonzaemon P-216** as a body with no scripted effect. All three lines keep it
where that cannot matter: `effect0` plays it with no `[Dark Masters]`-trait Digimon
in hand (`P_216.cs` `[On Play]` `CanActivate` false — nothing asked) and never lets
it be deleted; `effect1`/`effect2` only ever hold it in HAND and then under Gulfmon.

The one thing it carries into the stack is its **inherited `<Blocker>`**
(`P_216.cs` "Blocker - ESS"), which DCGO honours and our engine does not know. A
standing Gulfmon-over-P-216 would make DCGO open a block window on every P1 attack
that our engine never parks. `effect0` and `effect1` have no P1 attacks. `effect2`
has five — all on T6, the turn after Gulfmon attacked (T5), so Gulfmon is
**suspended** throughout and neither engine has a legal blocker. That ordering is
load-bearing; do not reorder T5/T6. It is a P-216 coverage gap (card not
implemented), not a Gulfmon finding.

## `effect#1`
`OptionalSkill` (isOptional TRUE; ours: `optional` + `outer_prompt` gate, 1:1) →
the hand-or-trash `generic_bool` is NOT asked (only one zone has a candidate →
`SetBool(canSelectHand)`, no RPC) → `SelectHandEffect` (ours: one declinable
`UnionZone{hand, trash}` pick; SHARED by identity) → `SelectPermanentEffect(Destroy)`
on Biyomon (SHARED). The assert on the delete row pins that the cost is already
paid (`sources: [P-216]`) while Biyomon still stands.

Not exercised: the TRASH origin and the both-zones `generic_bool` menu (needs a
Lv.5 "[Dark Masters]" card in P0's trash — only reachable by getting an unscripted
P-216 deleted or discarded, which no quiet line does), and the `[When Digivolving]`
arm (same body; `effect0` digivolves with the gate closed).

## `effect#2` — reaching seven cards in the opponent's trash legally
1 (Biyomon, Gulfmon's `[On Play]` delete) + 1 (the security card Gulfmon's T5 attack
checks) + 5 (four Monodramon BT1-009 and one Agumon BT3-007, vanilla, which attack
P0's security on T6 and lose to BlackGatomon ×4 / LadyDevimon) = 7 at the T7 attack
declaration. P1 affords five bodies on T4 because Gulfmon's 12-cost play hands it 9.

- **T5 is also a negative measurement**: with 1 card in the trash at declaration the
  clause is not live (`CanActivate`: `>= 7`; our `condition: count_gte 7`) and
  neither engine asks anything.
- **Slot safety with five Digimon.** DCGO seats Digimon centre-out (frames 4, 3, 5,
  2, 6), so `attack: field.0` names a different physical Monodramon on each engine —
  harmless because the four are identical vanilla cards that all die the same way,
  and Agumon, played LAST, is `field.0` only once it is alone (highest frame on
  both sides). Every projected row is the same multiset on both engines.
- **`hand.0` pins.** Four Monodramon in hand are ambiguous for `play:`; P1's hand in
  draw order is `[M, M, M, M, ST1-04, BT3-007]`, so slot 0 is a Monodramon for all
  four plays. This assumes DCGO's `HandCards` is append-on-draw (the confirmed
  `../BT11/BT11-089-effect0.yaml` relies on `hand.0` too, but only on the opening
  hand). **If the oracle run aborts or diverges at the T4 plays, suspect this pin
  first** — it would be a scenario artefact, not a clause finding.
- **The seven-card row.** DCGO: ONE `SelectCardEffect` (Root.Custom over the enemy
  trash, maxCount 7, canEndNotMax false). Ours: `CountCappedMultiSelect {7, 7}`,
  which the resolver answers from the same seven-id row → SHARED, one wire row.
- **Witness**: Gulfmon reads `suspended: false` after attacking, and P1's trash holds
  only the security card checked AFTER the return (`[ST1-04]`), security 4 → 3.
