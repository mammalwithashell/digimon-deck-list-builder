# NOTES — Hurricane Screw Shot (EX7-071)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-071`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`. DCGO script:
`EX7/Purple/EX7_071.cs` exists — the card is **not** `unavailable`. All three
clauses have a scenario; none is `unreachable`. No verdict is stored yet
(`qa/qa-reports/exam-verdicts/EX7-071.json` does not exist): all three are
`unmeasured` until the oracle pass.

| Clause | Scenario | Sim-only (2026-09-18) |
|---|---|---|
| `EX7-071#effect#0` — "When effects trash this card from digivolution cards, gain 1 memory. While you have a [Three Musketeers] trait Digimon, you may ignore this card's color requirements." | `EX7-071-effect0.yaml` | lowers, asserts pass |
| `EX7-071#effect#1` — `[Main]` delete 1 Lv.3, 1 Lv.4, 1 Lv.5; then place under a [Three Musketeers] trait Digimon | `EX7-071-effect1.yaml` | lowers, asserts pass — **oracle state divergence predicted** (below) |
| `EX7-071#effect#2` — `[Security]` delete 1 Lv.3, 1 Lv.4, 1 Lv.5 | `EX7-071-effect2.yaml` | lowers, asserts pass — **same prediction** |

Book for all three: `qa/dcgo-exams/EX7/three_musketeers_pool.json`
(`three-musketeers` / `quiet-opponent`).

## `effect#0` — re-authored 2026-09-18: the trasher is BeelStarmon, not LadyDevimon

The first draft (uncommitted, crashed stage) trashed the Option with LadyDevimon
BT25-083's `[When Attacking]` and claimed the trigger-timing finding of
`../P/NOTES-P-180.md` "has no observable on this line" because the clause body
is prompt-free. **Measured, it does:** LadyDevimon's effect still asks its
"you may use 1 … Option card from your trash" `SelectCardEffect` after the
trash, and at that row our engine already reads `p0.memory: 4` (our trigger ran
mid-effect) where DCGO — which stacks the trigger until LadyDevimon's effect
finishes — would read 3 (sim probe: `ASSERT FAILED: at 13: p0.memory expected 3
but our engine has 4`). The differ compares memory at every row, so that line
could only ever report the ENGINE's trigger timing as a `diverged` on this
card.

The committed line instead uses BeelStarmon BT25-085's `[When Digivolving]`
"By trashing 1 Option card from any of your Digimon's digivolution cards or
link cards, this Digimon unsuspends": it asks nothing after the trash and its
remaining body is a no-op (BeelStarmon has just digivolved, already
unsuspended), so both trigger orders produce the same prompt sequence and the
same state at every row. The Option is seated under **LadyDevimon** by her own
`[On Play]` so that BeelStarmon's *other* `[When Digivolving]` (use an Option
from hand or from THIS Digimon's digivolution cards) stays inactive on both
sides — one trigger, no `MultipleSkills` / `TriggerOrder`. Full derivation in
the file header. The witness is the turn not passing: the grant digivolve
leaves memory at −1 and the clause brings it back to 0.

The ignore-colour sentence is not exercised by any EX7-071 line: `effect#1`
pays the colour requirement with a purple Tamer, and no line USES the Option
while the only colour source is a [Three Musketeers] trait Digimon. P-180's and
EX7-070's `effect#1` lines do exercise the identical sentence (a red / black
Option used in a purple deck under BeelStarmon); for this PURPLE Option in a
purple deck the sentence cannot be isolated with the campaign books — it would
need a per-card book whose only purple permanent is a [Three Musketeers] trait
Digimon, which the pool's Lv.6s cannot be without purple bodies beneath them.
Recorded as a coverage limit of `effect#0`, not as `unreachable`: the clause's
trigger sentence is reached.

## `effect#1` / `effect#2` — predicted finding: sequential vs simultaneous deletion (card YAML)

`EX7_071.cs` runs the three level picks first, accumulating
`selectedPermanents`, and deletes them **together** in one
`DestroyPermanentsClass(...).Destroy()` after the third pick. Our
`code/digimon-engine/cards/ex7/EX7-071.yaml` authors each arm as
`select_opponent_permanent` → `delete_permanent`, so each target is deleted
before the next pick is asked. Measured on `EX7-071-effect1.yaml`:

```
at 21 (the Lv.4 pick row): p1.trash — ours [ST1-02]            DCGO (predicted) []
at 22 (the Lv.5 pick row): p1.trash — ours [BT1-014, ST1-02]   DCGO (predicted) []
```

The prompt SEQUENCE is identical (three `SelectPermanentEffect` rows, one
candidate each, each consumed by the harness's AI branch), so the oracle run
will not abort; it will report a state divergence at the second pick row. The
printed text is one sentence — "Delete 1 of your opponent's level 3, 1 of their
level 4, and 1 of their level 5 Digimon" — i.e. one simultaneous deletion
(`general_rule.pdf` 15-4-3 via `docs/digimon-rules/digest.md`: triggers
arising at the same timing are pending together and ordered by their
controller, which is only possible if the three deletions are one event). DCGO's reading is
the rules-correct one. With vanilla targets the END state is the same on both
sides, which is why the sim-only asserts pass; with `[On Deletion]` targets our
engine would resolve the first target's trigger before the second pick exists.

Fix shape (NOT applied — exam stage, triage and report): three selects binding
`lv3_target` / `lv4_target` / `lv5_target`, then one batched delete over the
bound set. Both the `[Main]` and the `[Security]` clause carry the same shape.
The three `opp.field.0` rows in both scenarios exist BECAUSE of the sequential
delete (each delete shifts our slots); once the YAML deletes simultaneously
they become `opp.field.0`, `opp.field.1`, `opp.field.2` — the wire rows do not
change (the wire carries top-card ids).

`effect#1` also rests on Asuna Shiroki BT24-088's prompt shapes (her `[On Play]`
fold and her `[Start of Your Turn]` OptionalSkill, re-read from `BT24_088.cs`
2026-09-18: both `SetUpActivateClass(..., -1, true, ...)`) and on the
"declined cost still draws 2" card finding that `BT24-088-effect0.yaml`'s
header records; that finding belongs to BT24-088's stage, and this line pays
the cost so it cannot surface here.

## Why `effect#1`'s colour source is a battle-area Tamer (observation, not a verdict)

`EX7-071-effect1.yaml` plays Asuna Shiroki BT24-088 on T1 purely as a PURPLE
permanent. A hatched purple Digi-Egg in the breeding area would have been
prompt-free, but it does not unlock the Option on our side:
`action/mask.rs::option_color_match_available` counts the breeding permanent
only when `Permanent::is_digimon` holds, and that predicate matches
`CardKind::Digimon | CardKind::Dual` — a Digi-Egg card is `CardKind::Egg`
(re-read 2026-09-18). The same function's doc block records that DCGO does not
count the breeding area at all and that the breeding branch is deliberate, so
a lone-egg line would have been a DCGO/ours disagreement about a rule that is
not this card's. Whether an In-Training card in the breeding area should
satisfy a colour requirement is a rules question for that helper's owner
(`general_rule.pdf` 3-4-5 is what the doc block cites); it is noted here only
because it shaped the line.

## `effect#0` triage (2026-09-18) — r1 abort was a slot-addressing artefact; r2 CONFIRMED

**r1** (recording `20260918T074045Z_9e7ecab2…`) aborted at step 25: expected
`SelectPermanentEffect`, DCGO asked `SelectCountEffect`, no state divergence in
the 21 rows compared. Cause: `digivolve: from: field.0` is a compact index over
DCGO's FRAME order (`InputDriver.FieldSlotToFrameId`; Digimon seat centre-out,
Tamers in the far row), so it named LadyDevimon BT25-083 on DCGO (two open
BeelStarmon costs -> cost `SelectCountEffect`) while ours named BlackGatomon
(entry order). The clause was never reached. Classification: **scenario
artefact (DCGO harness addressing quirk)** — not a card or engine finding.
Evidence order: printed text (bundle/image) = mandatory "gain 1 memory", no
"may"; `EX7_071.cs:19` `SetUpActivateClass(..., -1, false, ...)` + `AddMemory(1)`;
our YAML `scope: inherited` / `on_digivolution_card_trashed` / `gain_memory: 1`
agrees. No card/engine change.

**Re-author (r2)** — same board recipe as `EX7-070-effect0.yaml` r2: Asuna
played FROM HAND on T3 (first permanent), BlackGatomon digivolved in the
breeding area and promoted T7 -> ours `[Asuna, BlackGatomon, LadyDevimon]`,
BlackGatomon is `field.1` on both sides.

**r2** — oracle job `exam-EX7-071-effect0-r2`, recording
`20260918T095110Z_57c03ad4…`: diff **CLEAN** (22 of 32 ours / 27 DCGO rows).
DCGO `effect_activation` EX7-071 "Memory +1" `is_optional: false`,
`executed: true`; memory -1 -> 0, turn 9 stays with P0. Verdict **confirmed**.

EX7-071 now: 3 clauses — 1 confirmed, 2 diverged (`effect#1` / `effect#2`,
sequential-vs-simultaneous deletion + attacker slot artefact), 0 unmeasured.

## `effect#1` triage (2026-09-18) — OUR BUG, fixed, re-measured CONFIRMED

Classified **our bug** (card YAML + missing DSL vocabulary). Citations: printed
text is one sentence with no "Then" (one deletion event); `general_rule.pdf`
15-4-3-2 (triggers arising before a single effect resolves are simultaneous);
`EX7_071.cs:170-252` (one `DestroyPermanentsClass(selectedPermanents).Destroy()`
after the third pick). Fix: new DSL step `delete_permanents: { targets: [...] }`
(G-DSL-DELETE-PERMANENTS-BATCH, one `Game::delete_permanents_batch`), applied to
BOTH the `[Main]` and `[Security]` clauses of `EX7-071.yaml`. Tests
`ex7_071_{main,security}_triple_delete_is_simultaneous_after_third_pick` failed
before and pass after. `EX7-071-effect1.yaml` pick rows now read
`opp.field.0/1/2`; re-diffed against the PRESERVED sidecar
`20260918T074213Z_414c3998…`: **CLEAN** (24 of 24 ours / 25 DCGO rows) ->
`effect#1` **confirmed**. `effect#2` is triaged separately (its lead is the
attacker slot artefact; the simultaneous-deletion half is fixed by this change).

## `effect#2` triage (2026-09-18) — OUR BUG (fixed by `baba1a0ea`) + slot artefact; r2 CONFIRMED

**r1** (recording `20260918T074334Z_72bebad9…`), `--all-diffs` against the
preserved sidecar — two stacked defects:

1. LEAD step 18: `p1.field[0].suspended ours=false dcgo=true`,
   `p1.field[2].suspended ours=true dcgo=false` — the attacker slot-addressing
   artefact. `attack: attacker: field.0` named Biyomon on ours (entry order)
   and Kokatorimon on DCGO (centre-out frame order). **Scenario artefact.**
2. Steps 19-20: ours `p1.trash=[ST1-02]` / `[BT1-014, ST1-02]`, DCGO `[]` with
   all three still fielded — sequential vs simultaneous deletion. **Our bug**
   (card YAML): printed `[Security]` text is one sentence, one deletion event;
   `general_rule.pdf` 15-4-3-2; `EX7_071.cs:312-394` (three picks accumulated
   into `selectedPermanents`, one `DestroyPermanentsClass(...).Destroy()` at
   392-394). Fixed for BOTH clauses by the `effect#1` triage commit
   `baba1a0ea` (new DSL step `delete_permanents`, ENGINE/DSL FIX; test
   `ex7_071_security_triple_delete_is_simultaneous_after_third_pick` fails
   before / passes after). This triage made no further card or engine change.

The preserved r1 sidecar can never diff clean (DCGO attacked with a different
Digimon), so the line was **re-authored (r2)**: Vermilimon BT4-014 — the
THIRD-entered Digimon, `field.2` on both sides — attacks; the three picks read
`opp.field.0/1/2` (no slot shift, batch delete); `at: 19` / `at: 20` asserts
pin `p1.trash: []` at the 2nd and 3rd pick rows.

**r2** — oracle job `exam-EX7-071-effect2-r2`, recording
`20260918T102852Z_4848662e…`: diff **CLEAN** (22 of 22 ours / 22 DCGO rows).
DCGO `effect_activation` EX7-071 "[Security] Delete 1 of your opponent's level
3 Digimon, level 4 Digimon, and level 5 Digimon." `is_optional: false`,
`executed: true`. Verdict **confirmed**.

EX7-071 now: 3 clauses — 3 confirmed, 0 diverged, 0 unreachable,
0 unavailable, 0 unmeasured.
