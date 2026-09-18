# NOTES — Chikurimon (ST13-08): `unavailable`

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids ST13-08`):
**1 clause** — `ST13-08#effect#0` ("[All Turns] Players can't reduce play costs.").

| Clause | Scenario | Verdict | Measured reason |
|---|---|---|---|
| `ST13-08#effect#0` | — | `unavailable` | DCGO has no script for this card (below). |

## Why `unavailable`, checked per card (not per set)

`unavailable` is decided by the presence of the card's own C# file, never by
the set's age. The set directory exists, the card's file does not:

```
$BASE_DCGO/Assets/Scripts/CardEffect/ST13/            -> Black/, Red/
$BASE_DCGO/Assets/Scripts/CardEffect/ST13/Black/      -> ST13_01, ST13_09, ST13_11, ST13_13, ST13_14, ST13_16
$BASE_DCGO/Assets/Scripts/CardEffect/ST13/Red/        -> ST13_02..06, ST13_15
find $BASE_DCGO/Assets/Scripts/CardEffect -name 'ST13_08*'   -> (nothing)
```

(`BASE_DCGO="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/DCGO"`,
rule 29 — the worktree placeholder is empty by design and was not consulted.)

(Re-verified 2026-09-18 on the resumed campaign; unchanged.)

With no `ST13_08.cs`, DCGO plays Chikurimon as a vanilla body: the printed
gate would never be exercised on the oracle side, so any scripted line would
measure DCGO's absence of the effect, not its behaviour. No scenario is
authored; the clause must read as **not verified**, never as passed.

## Where the same printed text IS examinable

Psychemon **BT8-071** prints the identical clause ("[All Turns] Players can't
reduce play costs."), is scripted in DCGO (`BT8/Purple/BT8_071.cs`,
`CannotReduceCostClass`), and shares the engine-side implementation
(`kind: flood_gate`, `modifier: CannotReducePlayCost`, `target_player: any`
in both YAML specs — `code/digimon-engine/cards/st13/ST13-08.yaml` cites
BT8-071 as the same idiom). Its exam line is
`qa/dcgo-exams/BT8/BT8-071-effect0.yaml`; a `confirmed` there is evidence
about the shared substrate, but it is **not** a verdict on ST13-08 and must
not be recorded as one.
