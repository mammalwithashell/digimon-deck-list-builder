# NOTES — Cyclonemon (BT24-011)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT24-011`):
**4 clauses** — `effect#0` (`[Digivolve] Lv.3 w/[TS] trait: Cost 2`),
`effect#1` (`<Rush>`), `effect#2` (`<Raid>`), `inherited#0` (inherited `<Raid>`).
DCGO script: `BT24/Red/BT24_011.cs` exists — the card is **not** `unavailable`.
**Status: 4 clauses, 4 scenarios, 0 unreachable.** Book:
`qa/dcgo-exams/BT24/tm_bt24_011_pool.json` (decks `tm-toy-black`, `tm-quiet-red`
— the ToyAgumon BT25-064 shell, see `BT25/NOTES-BT25-064.md`).

| Clause | Scenario | Sim-only |
|---|---|---|
| `BT24-011#effect#0` | `BT24-011-effect0.yaml` | lowers, asserts pass |
| `BT24-011#effect#1` | `BT24-011-effect1.yaml` | lowers, asserts pass |
| `BT24-011#effect#2` | `BT24-011-effect2.yaml` | lowers, asserts pass |
| `BT24-011#inherited#0` | `BT24-011-inherited0.yaml` | lowers, asserts pass |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

All four files were on disk, uncommitted. All four re-lowered unchanged on the
current harness binary and every prompt sequence holds against `BT24_011.cs`;
no repairs were needed. None digivolves into BT25-085 and none crosses a
`<De-Digivolve>`, so neither campaign-wide re-lowering finding applies.

- `effect#0`: `AddSelfDigivolutionRequirementStaticEffect(IsLevel3 &&
  HasTSTraits, 2, ignoreDigivolutionRequirement: false)`. The base is ToyAgumon
  BT25-064 — BLACK Lv.3 [TS] — so the printed `Red Lv.3 / 2` circle fails on
  colour and the clause is the only route (one cost, no `SelectCountEffect`).
  The T3 breeding digivolve correctly expects `main_phase` (a Lv.2 in an
  occupied breeding area opens no breeding decision on either engine — the
  `P-180-effect1.yaml` convention).
- `effect#1` / `effect#2`: Cyclonemon is hard-played off a 7-memory donation
  (P1 hard-plays Phoenixmon ST1-10, 3 → −7) and attacks the same turn.
  Phoenixmon is unsuspended and the highest-DP opposing Digimon, so the printed
  `<Raid>` window opens on the declaration (`RaidSelfEffect(isInheritedEffect:
  false)`): DCGO asks `OptionalSkill` then a `SelectPermanentEffect`
  (`canNoSelect: true`), ours ONE declinable pick — the documented `<Raid>`
  fold (`expect: OptionalSkill` on the row). `effect#1` DECLINES (the attack
  lands on the player: the `<Rush>` witness); `effect#2` TAKES the switch (5000
  into 12000: Cyclonemon dies with the opposing security untouched — only the
  switch produces that).
- `inherited#0`: Cyclonemon sits under a vanilla Vermilimon BT4-014, so only
  the inherited `RaidSelfEffect(isInheritedEffect: true)` is live; one trigger
  on the declaration, no `MultipleSkills`. The Tsunomon BT24-007 egg's
  inherited trigger is a hand-trash event that never occurs.

Slot hygiene: each seat holds at most ONE Digimon in every file, so no
`field.N` address can land differently on DCGO's centre-out frames.
