# NOTES — Der Blitz (EX7-070)

Denominator (`clause_coverage.extract --card-ids EX7-070`): **3 clauses** —
`effect#0`, `effect#1`, `effect#2`. All three have a scenario; none is
`unreachable` or `unavailable` (`EX7/Black/EX7_070.cs` exists in DCGO).

| Clause | Scenario | Sim-only |
|---|---|---|
| `EX7-070#effect#0` | `EX7-070-effect0.yaml` | lowers, asserts pass — **oracle abort predicted** (below) |
| `EX7-070#effect#1` | `EX7-070-effect1.yaml` | lowers, asserts pass |
| `EX7-070#effect#2` | `EX7-070-effect2.yaml` | lowers, asserts pass |

## Predicted findings on `EX7-070#effect#0`

Two, both visible at lowering time and both independent of DCGO's answer:

1. **Trigger timing (engine-side, shared with P-180#effect#0).** Our engine
   fires "when effects trash this card from digivolution cards" immediately
   after the trash, in the middle of LadyDevimon BT25-083's still-resolving
   [When Attacking] effect; DCGO stacks it (`StackSkillInfos`) and asks
   LadyDevimon's use-from-trash pick first. `general_rule.pdf` 15-8-3-2
   (trigger-type effects wait as pending activation during effect
   processing) puts DCGO in the right. Full write-up: `../P/NOTES-P-180.md`.

2. **Spurious optional gate (card-YAML-side).** `code/digimon-engine/cards/
   ex7/EX7-070.yaml` marks clause 1 `optional: true`, citing DCGO's
   `SetIsOptionEffect(true)`. That call tags the effect as belonging to an
   Option card; optionality is the `isOptional` argument of
   `SetUpActivateClass(..., -1, false, ...)`, which is **false** here. The
   printed text ("When effects trash this card from digivolution cards,
   ＜De-Digivolve 1＞ 1 of your opponent's Digimon.") has no "may", and
   `<De-Digivolve>` is Mandatory (docs/digimon-rules/keyword-semantics.md,
   16-11). Our engine therefore parks a `Replacement` yes/no gate DCGO never
   asks. The scenario answers it `sim_only`; the YAML should drop
   `optional: true` (a rule-17 over-exposure: an illegal decline in the
   action space). Same misreading is worth checking on P-180.yaml's sibling
   comment, although P-180's clause 0 is authored mandatory and parks no gate.

`EX7-070-effect0.yaml` is authored in OUR order so that it lowers; the oracle
run is expected to abort at the trigger-pick row. Treat that as `diverged`
and triage with the two items above; do not fold the rows with
`sim_only:`/`dcgo_only:` to force a comparison of end states.
