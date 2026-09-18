# NOTES — Deputymon (EX7-010)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-010`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `inherited#0`. DCGO script
`EX7/Red/EX7_010.cs` exists — the card is **not** `unavailable`. No verdict is
stored yet (`qa/qa-reports/exam-verdicts/EX7-010.json` does not exist): all four
are `unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-010#effect#0` | `[Digivolve] Lv.3 w/[Three Musketeers] in text: Cost 2` | `EX7-010-effect0.yaml` | lowers, asserts pass |
| `EX7-010#effect#1` | `[When Digivolving] [When Attacking] [Once Per Turn] You may trash any 1 Option card from 1 Digimon's digivolution cards.` | `EX7-010-effect1.yaml` ([When Digivolving] arm) | lowers, asserts pass |
| `EX7-010#effect#2` | `[Your Turn] This Digimon gains the [Three Musketeers] trait.` | `EX7-010-effect2.yaml` | lowers, asserts pass |
| `EX7-010#inherited#0` | `[Your Turn] This Digimon gets +2000 DP.` | `EX7-010-inherited0.yaml` | lowers, asserts pass |

Book: `qa/dcgo-exams/EX7/tm_ex7_010_pool.json` (decks `tm-deputymon`,
`tm-red-opponent-010`; the book the crashed agent left, unchanged). Every line
keeps each seat on a ONE-Digimon board (`../BT25/NOTES-BT25-083.md`).

## Authoring decisions worth knowing before the oracle run

### `effect#0` — base choice
The printed circle (Red Lv.3 / 2) and the special line cost the same 2, so a
red Lv.3 cannot separate them. **Sparrowmon EX7-051** (purple Lv.3, text names
[Three Musketeers]) opens the clause ALONE: the digivolve is legal only through
it (`AddSelfDigivolutionRequirementStaticEffect` carries no `cardColor` →
`GetEvoCost` applies no colour check). The eggs in this book are red Koromon,
so Sparrowmon is hard-played on T1 rather than grown in the breeding area.

### The vacuous DCGO-only `OptionalSkill` rows (two of them)
- **Sparrowmon's `[Start of Your Main Phase]`** — the confirmed
  `EX7-051-effect0.yaml` shape: `isOptional: true` with a `CanActivate` of
  merely "hand or trash non-empty and a Digimon on the battle area".
- **Deputymon's own `[When Digivolving]` / `[When Attacking]`** —
  `CanActivateConditionShared` is only `IsExistOnBattleAreaDigimon(card)`, so
  DCGO asks the `OptionalSkill` EVERY time Deputymon digivolves or attacks,
  even when no Digimon on either side carries an Option
  (`SelectTrashDigivolutionCards` then breaks out at once:
  `maxDigivolutionDiscardCount == 0`). Our `EX7-010.yaml` gates the trigger on
  `any_permanent … source_count { kind: option } >= 1` and parks nothing.

Both are the documented vacuous-gate class (`docs/DCGO_EXAM.md` "Known gaps":
outcome-void, rules-supported on DCGO's side by §15-7-4, deliberately not
surfaced on ours) and are answered with `select: { decline: true, dcgo_only:
true }`. **Any other campaign line that digivolves into or attacks with
Deputymon needs the same row** (`../BT21/BT21-074-effect0.yaml` only PLAYS
Deputymon and never attacks with it, so it is unaffected).

### `effect#1` — host filter difference (not exercised, recorded)
DCGO's host `SelectPermanentEffect` offers EVERY battle-area Digimon of either
player (`permanentCondition: IsPermanentExistsOnBattleAreaDigimon`), including
ones with no Option underneath (picking one simply trashes nothing, and
`isFromOnly1Permanent` ends the loop). Ours offers only Digimon that hold an
Option (`select_any_permanent … source_count >= 1`). On the committed line the
board is a single Digimon, so the candidate sets coincide. A multi-Digimon line
with `expect: { candidates: … }` would read this as a difference. Not
adjudicated here (no rule section was checked for it); it is outcome-affecting
only if a player deliberately picks a Digimon with no Option and whiffs.

The Option is EX7-071 on purpose: its source-trash trigger (+1 memory) is
prompt-free and nothing in Deputymon's effect follows the trash, so the open
trigger-timing finding (`../P/NOTES-P-180.md`) cannot show at any row.

### `effect#2` — measuring a trait
Traits are not projected. The grant is read through Hurricane Screw Shot
(EX7-071, purple): with only the RED Deputymon on P0's field, using it is legal
solely through its "While you have a [Three Musketeers] trait Digimon, you may
ignore this card's color requirements", and its `[Main]` tuck ("… bottom
digivolution card of 1 of your [Three Musketeers] trait Digimon",
`EqualsTraits`) finds a target solely through the same grant. Without the grant
the card ends in the trash (the confirmed `EX7-071-effect1.yaml` outcome); with
it, `sources: [EX7-071]`. The not-your-turn half of "[Your Turn]" is not
observable through any card in the pool and is not claimed.

### `inherited#0`
Read directly off the projected effective DP: Megadramon over Deputymon is 9000
on P0's row and 7000 on P1's next row. Megadramon's two routes onto Deputymon
cost the same 3 → no cost prompt.
