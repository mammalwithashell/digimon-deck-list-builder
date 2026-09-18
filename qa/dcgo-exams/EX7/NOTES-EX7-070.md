# NOTES — Der Blitz (EX7-070)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-070`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`. All three have a scenario;
none is `unreachable` or `unavailable` (`EX7/Black/EX7_070.cs` exists in DCGO).
No verdict is stored yet (`qa/qa-reports/exam-verdicts/EX7-070.json` does not
exist): all three are `unmeasured` until the oracle pass.

| Clause | Scenario | Sim-only (2026-09-18) |
|---|---|---|
| `EX7-070#effect#0` | `EX7-070-effect0.yaml` | lowers, asserts pass (re-authored 2026-09-18, below) |
| `EX7-070#effect#1` | `EX7-070-effect1.yaml` | lowers, asserts pass (prefix re-authored: BeelStarmon can no longer be `play:`ed) |
| `EX7-070#effect#2` | `EX7-070-effect2.yaml` | lowers, asserts pass (unchanged since `c9c3ccf49`) |

Book: `qa/dcgo-exams/EX7/three_musketeers_pool.json`.

## `effect#0` — re-authored around the engine's trigger-timing finding

The engine finding — a "when effects trash this card from digivolution cards"
trigger resolves in the MIDDLE of the trashing effect, where DCGO stacks it
until that effect finishes (`general_rule.pdf` 15-8-3-2) — is written up once,
with its two sim-side measurements, in `../P/NOTES-P-180.md`. With LadyDevimon
BT25-083's `[When Attacking]` as the trasher (the previous revision of this
line) an oracle run could only abort on prompt sequence and measure that
finding instead of this clause.

The committed line seats EX7-070 under LadyDevimon with her `[On Play]` and
trashes it with BeelStarmon BT25-085's `[When Digivolving]`
trash-an-Option-to-unsuspend effect, which asks nothing after the trash and
whose remaining body is a no-op on a just-digivolved Digimon: both trigger
orders give the same prompt sequence and the same state at every row. The
De-Digivolve target is a vanilla Kokatorimon-over-Agumon stack P1 builds on
T2/T4.

Two rows on it are one-sided, and both are stated in the file:

1. **DCGO-only `SelectCountEffect`, `value: 1`.** `IDegeneration(permanent, 1,
   activateClass)` is constructed without a `DegenerationCountRuling`, so
   `Degeneration()` opens "How many cards do you trash?" over the single
   candidate `{1}`; its auto-pick goes through `SetCount`, whose harness
   intercept consumes a scripted row. Our engine parks nothing for
   `<De-Digivolve 1>` (keyword-semantics 16-11: mandatory, no choice at x = 1).
   Finding first recorded in `../BT21/NOTES-BT21-074.md`; the previous revision
   of this line lacked the row.
2. **Sim-only gate — a card-YAML finding (below).**

## Card-YAML finding: spurious `optional: true` on the trigger clause

`code/digimon-engine/cards/ex7/EX7-070.yaml` marks the trigger clause
`optional: true`, citing DCGO's `SetIsOptionEffect(true)`. That call tags the
effect as belonging to an Option card; optionality is the `isOptional` argument
of `SetUpActivateClass(..., -1, false, ...)`, which is **false** here. The
printed text ("When effects trash this card from digivolution cards,
＜De-Digivolve 1＞ 1 of your opponent's Digimon.") has no "may", and
`<De-Digivolve>` is Mandatory (`docs/digimon-rules/keyword-semantics.md`,
16-11). Our engine therefore parks a `Replacement` yes/no gate DCGO never asks
— a rule-17 over-exposure (an illegal decline in the action space). Still
present on 2026-09-18 (the gate is live at lowering; the scenario answers it
`{ yes: true, sim_only: true }`). Fix: drop `optional: true`; once fixed, that
row must be removed or the line stops lowering. P-180's sibling clause is
authored mandatory and parks no gate. NOT applied from this stage.

## `effect#1` — what changed since `c9c3ccf49`

Same prefix change as `../P/NOTES-P-180.md` ("`effect#1` — what changed"):
BeelStarmon BT25-085 is reached through BlackGatomon's `[All Turns]` grant,
because `play: BT25-085` now lowers as the dual card's Option face. P1 fields
Agumon (cost 3) and Monodramon (cost 2) so "lowest play cost" is a real
selection. Both `SelectPermanentEffect` picks (`canNoSelect: false`) consume a
wire row under the harness's AI branch even with one candidate.
