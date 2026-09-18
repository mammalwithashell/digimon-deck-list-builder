# NOTES — Bind Red Trigger (P-180)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids P-180`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`. All three have a scenario;
none is `unreachable` or `unavailable` (`P/Red/P_180.cs` exists in DCGO). No
verdict is stored yet (`qa/qa-reports/exam-verdicts/P-180.json` does not
exist): all three are `unmeasured` until the oracle pass.

| Clause | Scenario | Sim-only (2026-09-18) |
|---|---|---|
| `P-180#effect#0` | `P-180-effect0.yaml` | lowers, asserts pass (re-authored 2026-09-18, below) |
| `P-180#effect#1` | `P-180-effect1.yaml` | lowers, asserts pass (prefix re-authored: BeelStarmon can no longer be `play:`ed) |
| `P-180#effect#2` | `P-180-effect2.yaml` | lowers, asserts pass (unchanged since `c9c3ccf49`) |

Book: `qa/dcgo-exams/EX7/three_musketeers_pool.json`.

## ENGINE FINDING — a source-trash trigger resolves in the middle of the trashing effect

Still open, still unfixed, and independent of which scenario exhibits it.

- **DCGO** (`ITrashDigivolutionCards.TrashDigivolutionCards()` →
  `AutoProcessing.StackSkillInfos(hashtable, OnDigivolutionCardDiscarded)`):
  the trashed card's "when effects trash this card from digivolution cards"
  trigger is STACKED and resolves after the trashing effect finishes.
- **Our engine** (`game_actions/link.rs::trash_specific_source_card` →
  `enqueue_triggered` + `maybe_drain_effect_queue`): the trigger resolves
  immediately after the trash, BEFORE the rest of the trashing effect's body.

`general_rule.pdf` 15-8-3-2 (digest: `docs/digimon-rules/digest.md`) — a
trigger-type effect "can't activate during rule/effect processing; it waits as
pending activation" — makes DCGO's order the rules-correct one. This is a
**candidate engine bug** (effect queue drained mid-process), not a P-180 text
issue, and not a translation gap the exam should fold.

Two sim-side measurements of it (2026-09-18), both with LadyDevimon BT25-083's
`[When Attacking]` ("By trashing 1 Option card …, you may use 1 [Three
Musketeers] trait Option card from your trash with the cost reduced by 3") as
the trasher:

1. **Prompt order** (P-180 / EX7-070 under BeelStarmon, the line this file's
   `effect#0` held until 2026-09-18 — `git show c9c3ccf49:qa/dcgo-exams/P/P-180-effect0.yaml`
   for its shape): the step after the trash sees the TRIGGER's parked
   `OppField` pick; LadyDevimon's use-from-trash `SelectCardEffect` comes
   after it. DCGO asks them the other way round.
2. **State at a shared row** (EX7-071, whose trigger body is a prompt-free
   +1 memory): at LadyDevimon's use-from-trash row our engine already reads
   memory 4 where DCGO, with the trigger still stacked, reads 3
   (`ASSERT FAILED: at 13: p0.memory expected 3 but our engine has 4` on a
   probe copy of the old `EX7-071-effect0` line).

**Where it should be tracked:** `docs/RUST_ENGINE_GAPS.md` has no entry for it
(grep for `15-8-3-2`, `trash_specific_source_card`, "mid-effect" finds
none as of 2026-09-18). Proposed id
`G-ENGINE-SOURCE-TRASH-TRIGGER-RESOLVES-MID-EFFECT`. Not added from this stage
(scenario / NOTES / book files only).

## `effect#0` — why the scenario no longer uses LadyDevimon's `[When Attacking]`

With that trasher the oracle run can only abort on prompt sequence at the
trigger row: it measures the engine finding above, never the clause. The
previous revision of this file kept that line on purpose ("the abort is the
measurement"). It is replaced because the finding does not need a P-180
scenario to stay recorded (this file records it, with two measurements), while
the clause does need a line that can be measured.

The committed line seats P-180 under LadyDevimon with her `[On Play]`, then
trashes it with BeelStarmon BT25-085's `[When Digivolving]` "By trashing 1
Option card from any of your Digimon's digivolution cards or link cards, this
Digimon unsuspends": nothing is asked after the trash, and the remaining body
is a no-op on a Digimon that has just digivolved. Both trigger orders therefore
produce the same prompt sequence and the same state at every row. This is NOT a
`sim_only:` / `dcgo_only:` re-ordering — no row is folded to hide an order;
the two engines genuinely walk the same sequence. The only folded rows are the
documented existence gaps (our `optional: true` gate on BeelStarmon's effect,
the digivolution-card pick the resolver cannot name by identity, Asuna's gate,
DCGO's zone menu).

What this line does not exercise: the ignore-colour sentence (the Option is
never USED on it). `P-180-effect1.yaml` does — a red Option used in a purple
deck with BeelStarmon as the only [Three Musketeers] trait Digimon.

## `effect#1` — what changed since `c9c3ccf49`

`play: { card: BT25-085 }` now lowers as the dual card's OPTION face ("Fly
Bullet"); its Digimon face "can't play to the field". The prefix is therefore
the BlackGatomon-grant route (`../BT25/BT25-082-effect2.yaml`): hatch Pagumon,
Sparrowmon, BlackGatomon (alt circle 2, cost prompt), Asuna free-played,
BlackGatomon → BeelStarmon for 4 via the `[All Turns]` grant. BlackGatomon is a
Lv.4, so BeelStarmon's later-added "Lv.5 w/[Three Musketeers] in text or w/[TS]
trait: Cost 3" route (`9aa21ed0f`) does not open and no second cost prompt is
asked on either side (`../BT25/NOTES-BT25-083.md` is the record of the Lv.5
case).

Placing P-180 under BeelStarmon is an effect adding a digivolution card; the
stack beneath holds Pagumon ST6-01 / Sparrowmon / BlackGatomon, none of which
observes `OnAddDigivolutionCards` (the observers are BT25-005, EX7-005,
BT7-056), so nothing else triggers on either side.

## Correction to `../BT21/NOTES-BT21-074.md`'s heads-up

That note lists `P-180-effect0.yaml` beside `EX7-070-effect0.yaml` as a line
that resolves a `<De-Digivolve 1>` through `IDegeneration` and therefore needs
the DCGO-only `SelectCountEffect` row. P-180's trigger **deletes**
(`SelectPermanentEffect.Mode.Destroy`); there is no `IDegeneration` and no
count row on this card. `EX7-070-effect0.yaml` does need it and now carries it.

## Harness translation gap (not a rules finding)

A `select: { cards: [ID] }` answer cannot name a digivolution-card candidate:
`selection_resolve.rs`'s `Hand | UnionZone` arm scans only hand and trash, and
`EffectChoice` / `SourceMulti` have no identity arm at all. Lines here answer a
single-candidate source pick with `{ yes: true, sim_only: true }` (the
resolver's single-accept-id path) and put DCGO's card row on the wire as
`dcgo_only`. A two-candidate source pick is unanswerable — see
`../EX7/NOTES-EX7-073.md` (`EX7-073#effect#2`).

## `effect#0` triage (2026-09-18) — r1 abort was a slot-addressing artefact; r2 CONFIRMED

**r1** (recording `20260918T074731Z_7ea4b67c…`), `--all-diffs` against the
preserved sidecar: `TRUNCATED, no divergence found` over the 22 comparable
rows; the job aborted at the T9 digivolve ("expected 'SelectPermanentEffect'
but DCGO asked 'SelectCountEffect'"). The sidecar's `p0.field` at that row is
`[BT25-083, BT25-082, BT25-092]`: DCGO seats Digimon centre-out
(`CardSource.PreferredFrame`: frames 4, 3, 5, …) and `digivolve: from: field.N`
reaches it as a COMPACT frame-order index (`InputDriver.FieldSlotToFrameId`),
so `field.0` named LadyDevimon BT25-083 on DCGO. A Lv.5 has two open routes
into BeelStarmon BT25-085 (circle 4, alt 3) → the cost `SelectCountEffect`. The
clause was never reached. Class: scenario artefact — neither engine wrong, no
card or engine change.

**r2**: BeelStarmon (X Antibody) EX7-073 is played first as slot ballast (no
`[On Play]`; in the `three-musketeers` book), LadyDevimon second, and
Sparrowmon is promoted THIRD, so the digivolving stack is `field.2` / frame 5 on
both sides for both digivolves. Oracle job `exam-P-180-effect0-r2`, recording
`20260918T111304Z_c2360b77…`: diff **CLEAN** (24 of 31 ours / 27 DCGO rows).
DCGO `effect_activation` BT25-085 "By trashing 1 Option …, unsuspend"
`executed: true`, then P-180 "Delete 1 digimon with 7k DP or lower"
`is_optional: false`, `executed: true`; Agumon ST1-03 deleted on both sides.
Verdict **confirmed**. Citations for the clause body: printed text
(`data/card_bundles/P-180.md`), `P_180.cs` "When Trashed from digivolutions
cards" (mandatory, one `SelectPermanentEffect` Destroy over DP ≤ 7000).

The ENGINE FINDING above (source-trash trigger resolves mid-effect,
`general_rule.pdf` 15-8-3-2) is unaffected by this line and remains open.
