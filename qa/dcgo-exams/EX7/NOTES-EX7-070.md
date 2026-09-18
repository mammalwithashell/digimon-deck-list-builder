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
authored mandatory and parks no gate. **FIXED 2026-09-18** (see the triage
section below).

## `effect#1` — what changed since `c9c3ccf49`

Same prefix change as `../P/NOTES-P-180.md` ("`effect#1` — what changed"):
BeelStarmon BT25-085 is reached through BlackGatomon's `[All Turns]` grant,
because `play: BT25-085` now lowers as the dual card's Option face. P1 fields
Agumon (cost 3) and Monodramon (cost 2) so "lowest play cost" is a real
selection. Both `SelectPermanentEffect` picks (`canNoSelect: false`) consume a
wire row under the harness's AI branch even with one candidate.

## `effect#0` triage (2026-09-18) — r1 abort was a slot-addressing artefact; card-YAML `optional: true` fixed; r2 CONFIRMED

**r1** (recording `20260918T073651Z_ed05eacf…`) aborted "step 26 expected
'SelectPermanentEffect' but DCGO asked 'SelectCountEffect'", 22 rows compared,
no state divergence. `--all-diffs` against the preserved sidecar shows nothing
more: the clause was never reached. Cause: `digivolve: from: field.0` is a
COMPACT index over `Player.FieldPermanents` in frame order on DCGO
(`InputDriver.FieldSlotToFrameId`), and `CardSource.PreferredFrame` seats
Digimon centre-out (4, 3, 5, …; Tamers in the far row). DCGO's order was
[LadyDevimon, BlackGatomon, Asuna]; ours (entry order) [BlackGatomon, Asuna,
LadyDevimon]. `field.0` named LadyDevimon on DCGO — a Lv.5 with two open
BeelStarmon costs — hence the cost `SelectCountEffect`. Scenario artefact;
neither engine wrong.

**Re-author (r2).** Asuna is played FROM HAND on T3 (first permanent in the
battle area) and BlackGatomon is grown in the BREEDING area (`from: breeding`,
slot-free) and promoted on T7: ours [Asuna, BlackGatomon, LadyDevimon], DCGO
[LadyDevimon, BlackGatomon, Asuna] — BlackGatomon is `field.1` on both sides.
Reusable rule (extends `../BT25/NOTES-BT25-083.md`): with exactly two own
Digimon, put ONE Tamer into the battle area before either Digimon and address
the FIRST-entered Digimon as `field.1`.

**Card-YAML fix (our bug, landed with the re-author).** `EX7-070.yaml` clause 1
carried `optional: true`. Citations: `EX7_070.cs:18`
`SetUpActivateClass(CanActivateCondition, ActivateCoroutine, -1, false, …)` —
isOptional = false (`SetIsOptionEffect(true)` on :20 only tags an Option
card's effect); printed text has no "may"; `<De-Digivolve>` is Mandatory
(general_rule 16-11). Tests `ex7_070_clause1_is_inherited_mandatory_source_trash`,
`ex7_070_source_trash_offers_dedigivolve_and_resolves`,
`ex7_070_source_trash_cannot_be_declined` failed before (parked
`Replacement`), pass after; `cargo test --test cards_behavioral -- ex7_070`
20/20. The scenario's sim-only gate row was dropped.

**r2** — oracle job `exam-EX7-070-effect0-r2`, recording
`20260918T093542Z_a6c8ca88…`: diff **CLEAN** (22 of 33 ours / 28 DCGO rows).
DCGO `effect_activation` EX7-070 "De-Digivolve 1" `is_optional: false`,
`executed: true`; final state Kokatorimon in P1's trash, Agumon remains,
EX7-070 in P0's trash, turn 10 at memory −1. Verdict **confirmed**.

EX7-070 now: 3 clauses — 2 confirmed, 1 diverged (`effect#2`, attacker
slot-addressing artefact, needs the same kind of re-author), 0 unmeasured.
