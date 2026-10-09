# NOTES -- Sunarizamon (EX10-025)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX10-025`):
**2 clauses** -- `effect#0` ([On Play]), `inherited#0`. DCGO script exists
(`EX10/Black/EX10_025.cs`); card is not `unavailable`.

EX10-025: 2 clauses: 1 confirmed, 1 diverged, 0 unreachable, 0 unavailable, 0 unmeasured

Book: `rocks_pool.json` (deck `rocks-exam`, eggs ST5-01 so the Lv.2 base is NOT a
[Rock]/[Mineral] source -- Tumblemon would have made our SourceMulti pick ambiguous).

| Clause | Scenario | Oracle round-trips | Verdict |
|---|---|---|---|
| `EX10-025#effect#0` | `EX10-025-effect0.yaml` | 1 | confirmed (CLEAN, 10 of 17 ours / 14 dcgo rows compared) |
| `EX10-025#inherited#0` | `EX10-025-inherited0.yaml` | 2 | diverged (see below) |

## effect#0 -- line and prompt-order folds
Trash is filled by playing EX11-038 Sunarizamon twice (T1, T3), each trashing 1
Mineral/Rock hand card; EX10-025 is then played on T3 with exactly 2 in trash.
DCGO asks ONE `SelectCardEffect` (2 cards) then the host `SelectPermanentEffect`; our
clause asks the host first (its optional gate doubles as the pick) then 2 sequential
trash picks. Authored as `sim_only` / `dcgo_only` pairs; end state agrees:
EX10-025 holds sources [EX8-047, EX8-048], trash empty. The EX11-038 UnionZone cost is a
DCGO `generic_int` zone menu (0 = hand) folded as a `dcgo_only` `value: 0` row.
The 1-card-in-trash case ("place 2" with only 1 qualifying card) was NOT exercised.

## inherited#0 -- DIVERGED: resolution order (triage: our bug, rules-supported)
Line: ST5-01 (egg) -> EX10-025 -> EX10-028 Landramon; Landramon's [When Digivolving]
trashes EX10-025 from its sources, firing the inherited delete on p1's BT21-055.
- Round-trip 1 aborted on prompt order: DCGO "SelectPermanentEffect: wanted card 'BT21-055'
  ... is not among the offered candidates [EX10-028]".
- Round-trip 2 (rows split sim_only / dcgo_only): DIVERGED at step 16,
  `p0.field[0].dp: ours=4000 dcgo=8000`. DCGO state rows: step 18 Landramon DP 8000
  (4000 + 3000 + 1000 from ST5-01's inherited Blocker bonus) with BT21-055 still alive;
  step 20 BT21-055 in p1.trash. So DCGO finishes Landramon's own "1 of your Digimon
  gains Reboot/Blocker/+3000" step BEFORE the inherited trigger resolves; our engine
  runs the inherited trigger first and asks Landramon's own pick last.
- Rule: general_rule.pdf p.25 section 15-8-3-2 "Trigger-type effects can't activate
  during the processing for a rule or effect" -> the inherited trigger must wait for
  Landramon's effect to finish. DCGO's order is the rule-supported one; ours is not.
- Both engines delete BT21-055 in the end (DCGO sidecar: p1.field empty, p1.trash
  [BT21-055]); the divergence is ordering, not outcome. Not fixed here.
