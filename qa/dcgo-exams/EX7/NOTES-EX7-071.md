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
