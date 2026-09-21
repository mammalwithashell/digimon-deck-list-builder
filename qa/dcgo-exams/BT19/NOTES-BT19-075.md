# NOTES — MoonMillenniummon (BT19-075)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT19-075`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `effect#3`.
DCGO script: `BT19/Purple/BT19_075.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. All four clauses have a scenario; none is
`unreachable`. Book: `tm_bt19_075_pool.json` (decks `tm-moonmillenniummon`,
`quiet-opponent-mm`) — the shared Three Musketeers book has no Millenniummon
line, no `[Composite]` Digimon and no Tamer for the opponent.

| Clause | Text | Scenario | Sim-only (2026-09-18 re-lower) |
|---|---|---|---|
| `BT19-075#effect#0` | `[Digivolve] [Millenniummon]: Cost 2` | `BT19-075-effect0.yaml` | lowers, asserts pass |
| `BT19-075#effect#1` | `[On Play] [When Digivolving] Your opponent trashes cards in their hand until they have 5 left. For every 2 this effect trashed, delete 1 of your opponent's Tamers.` | `BT19-075-effect1.yaml` ([When Digivolving] arm, both sentences live) | lowers, asserts pass |
| `BT19-075#effect#2` | `[All Turns] When this Digimon would leave the battle area, by deleting 1 of your [Composite] trait Digimon, it doesn't leave.` | `BT19-075-effect2.yaml` | lowers, asserts pass — **predicted divergence on the follow-up**, below |
| `BT19-075#effect#3` | `[All Turns] [Once Per Turn] When other Digimon or Tamers are deleted, trash your opponent's top security card.` | `BT19-075-effect3.yaml` | lowers, asserts pass |

None of the four lines digivolves into BT25-085 and none resolves a
`<De-Digivolve>`, so the campaign-wide re-lowering findings (BeelStarmon cost
row, `IDegeneration` count row) do not apply; all four were re-lowered unchanged
against the current harness.

## Adversarial pre-Unity review — prompt sequences re-derived from `BT19_075.cs`

- **Millenniummon BT18-019 hard play (every line).** `BT18_019.cs` registers a
  DigiXros condition (Kimeramon x Machinedramon). `CardController` only calls
  `SelectDigiXrosClass.Select` for `card.HasDigiXros`, and `Select` opens a
  zone menu per element **only** when hand / field / trash holds a candidate
  (`canSelectHand || canSelectField || canSelectTrash`). No line holds a
  Kimeramon or Machinedramon anywhere, so nothing is asked. Its shared
  `[On Play]` is `optional: false` -> one `SelectPermanentEffect`
  (`canNoSelect: false`), asked with the single candidate Phoenixmon.
- **`effect#1`.** Both `[On Play]` and `[When Digivolving]` are registered under
  `EffectTiming.OnEnterFieldAnyone` as two `ActivateClass`es, but
  `CanTriggerOnPlay` / `CanTriggerWhenDigivolving` are mutually exclusive, so a
  digivolve stacks ONE trigger -> no `MultipleSkills`. `isOptional: false` -> no
  `OptionalSkill`. The hand trash is ONE `SelectHandEffect` owned by **p1**
  (`selectPlayer: card.Owner.Enemy`, `maxCount = hand - 5`, `canNoSelect: false`,
  `canEndNotMax: false`) — the scenario row is `actor: 1` with both ids in one
  answer. The Tamer delete is p0's `SelectPermanentEffect` (`Mode.Destroy`,
  `maxCount = min(floor(n/2), tamers)`), asked with the single Tamer.
  Sim note (expected): our `CountCappedMultiSelect` has no unambiguous DCGO
  prompt mapping, so step 20's `SelectHandEffect` is asserted by DCGO only.
- **`effect#2`.** `WhenRemoveField` is `isOptional: true` -> `OptionalSkill`,
  then `SelectPermanentEffect` (`canNoSelect: true`, single candidate Deltamon:
  MoonMillenniummon's own traits are `[Wicked God]`, so it is not its own cost).
  Ours: `Replacement` gate + a plain own-permanent pick -> same two rows.
- **`effect#3`.** `isOptional: false`, no pick -> no prompt on either side.

## Finding (engine, predicted): the OPT observer misses a replacement-cost deletion

Measured sim-side on `BT19-075-effect2.yaml` (re-measured 2026-09-18, still
present): after the leave-replacement deletes Deltamon BT6-012 as its cost, our
engine leaves **p1's security at 4**. DCGO deletes the cost through
`CardEffectCommons.DeletePeremanentAndProcessAccordingToResult`, which raises
`OnDestroyedAnyone` like any other deletion, and `BT19_075.cs`'s own
`[All Turns] [Once Per Turn]` observer (`OtherPermanentDeleted`:
`permanent.IsDigimon || ...`) is live on the board — so DCGO is expected to
trash p1's new top security card (Dracomon ST1-04) and read **3**. The same
observer fires correctly for a battle deletion (`BT19-075-effect3.yaml`) and for
an effect deletion of a Tamer (`BT19-075-effect1.yaml`), so the suspect is the
deletion path taken by a `kind: replacement` cost `delete_permanent` (no
`on_any_deletion` dispatch), not the observer. Printed text agrees with DCGO:
Deltamon is an "other Digimon" and it was deleted. The `assert:` block
deliberately names only fields both engines should agree on; the oracle differ
compares `p1.security` on every row and will report it. **Not fixed here** —
recorded for triage after the oracle pass.

## Oracle pass (2026-09-19, drain of 2026-09-18) -- effect2 is NOT slot-safe

`effect0`, `effect1`, `effect3` diffed CLEAN -> `confirmed`. `effect2`'s job
failed (`SelectPermanentEffect: wanted card 'BT6-012' ... offered [BT18-019]`)
and stays **`unmeasured`** -- a scenario defect, not an engine finding. The
line has TWO p0 Digimon when it addresses `field.1` (T5 digivolve + attack).
DCGO seats Digimon centre-out (Deltamon frame 4, Millenniummon frame 3 ->
compact order [Millenniummon, Deltamon]); ours is play order [Deltamon,
Millenniummon]. So DCGO's action 401 put MoonMillenniummon on **Deltamon**
(the sidecar diff leads at step 12: `p0.field[1].sources` DCGO `[BT6-012]`),
and the replacement then only offered Millenniummon. Selections travel by card
identity; main-phase `field.N` actions do not. Re-author so Millenniummon is
addressed at an index both engines agree on (e.g. a third-played Digimon sits at
index 2 on both), then re-drain. Side observation for the DCGO harness: the
recording shows that digivolve RESOLVED (`action_detail` cost_paid 0,
`alt_path: cost_modifier`) -- a Lv.6 onto a Lv.4 non-[Millenniummon] for 0 --
which looks like InputDriver applying the [Millenniummon] alt path to the wrong
frame; worth checking before trusting any slot-mismatched line.
**4 clauses: 3 confirmed, 0 diverged, 0 unreachable, 1 unmeasured.**

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#2` re-authored against the measured slot defect

The r1 triage above ("`effect2` is NOT slot-safe") is now acted on. The line
held TWO p0 Digimon when it addressed `field.1` for the T5 digivolve and the
attack; DCGO seats Digimon CENTRE-OUT (`CardSource.PreferredFrame`: frames 4,
3, 5, …) and a main-phase `field.N` reaches it as a COMPACT frame-order index
(`InputDriver.FieldSlotToFrameId`), so at N=2 the two engines disagree about
every index and DCGO digivolved onto Deltamon.

**The repair.** p0 now fields a THIRD Digimon, and the one under exam is the
third-entered — the only index (2) the two orderings agree on, the
`../P/P-180-effect0.yaml` / `-effect2.yaml` r2 pattern that went oracle-CLEAN:

| entry | card | why |
|---|---|---|
| 0 | Monodramon BT1-009 (vanilla Lv.3, [Mini Dragon]) | slot ballast; NOT [Composite], so the replacement's cost pick stays single-candidate |
| 1 | Deltamon BT6-012 | the [Composite] the replacement pays |
| 2 | Millenniummon BT18-019 → MoonMillenniummon | every `field.N` in the file |

Only `digivolve: { from: field.2 }` and `attack: { attacker: field.2 }` carry
an index; the three `select:` rows travel by card identity. The turn structure
grew to T7 to fit the extra play without clamping memory (T1 Monodramon 2, T3
Deltamon 5, T4 p1 Phoenixmon 10, T5 Millenniummon 14, T6 p1 two plays, T7
digivolve + attack), and p1 makes two T6 plays so its hand is exactly 5 at the
digivolve and `#effect#1`'s hand trash stays silent. Millenniummon's [On Play]
still fires with p1 holding exactly one Digimon, so that pick is unambiguous
too.

**The predicted divergence was re-measured on the new line and still holds.**
A scratch copy asserting `p1.security: 4` at the witness row passes on our
engine: after the leave-replacement deletes Deltamon as its cost, our
`[All Turns] [Once Per Turn]` observer (`#effect#3`) does NOT fire, so p1's
security stays at 4 and ST1-04 is not in p1's trash. DCGO deletes the cost
through `CardEffectCommons.DeletePeremanentAndProcessAccordingToResult`, which
raises `OnDestroyedAnyone` like any deletion, so it is expected to read 3. The
committed `assert:` deliberately names only the fields both engines should
agree on and leaves `p1.security` to the oracle differ. Still **not fixed**
(exam stage) — recorded for triage after the oracle pass.

Sim-only after the repair: 10 checks / 0 failed. `effect#2` stays
**`unmeasured`**; `effect0` / `effect1` / `effect3` keep their `confirmed`
verdicts (untouched).
**4 clauses: 3 confirmed, 0 diverged, 0 unreachable, 1 unmeasured.**

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#2` MEASURED on the repaired line: **diverged**, and the predicted engine finding is CONFIRMED

The re-authored slot-safe line drained `completed` on oracle build `scripted-v16`
(DCGO `fc67f9ae6`). The `unmeasured` above is resolved.

| Clause | Sidecar | Diff | Verdict |
|---|---|---|---|
| `BT19-075#effect#2` | `20260921T041928Z_40f7887a` | DIVERGED at step 21, compared **22 of 22 ours / 22 dcgo** — nothing truncated, nothing excluded | **diverged** |

The slot repair worked: the digivolve landed on Millenniummon on both wires (no
`p0.field[..].sources` row in the diff at all), and the three `select:` rows travelled
by card identity as designed. The ONLY divergence is the one this file predicted
sim-side two resumes ago:

- `p1.security: ours=4 dcgo=3`
- `p1.trash: ours=[P-180, ST1-10] dcgo=[P-180, ST1-04, ST1-10]`

**ENGINE FINDING, ours is wrong.** The replacement pays its cost by deleting Deltamon
BT6-012. DCGO deletes that cost through
`CardEffectCommons.DeletePeremanentAndProcessAccordingToResult`, which raises
`OnDestroyedAnyone` like any other deletion, so this card's OWN `[All Turns]
[Once Per Turn]` observer (`#effect#3`) fires and trashes p1's top security (Dracomon
ST1-04). Ours does not fire. Printed text agrees with DCGO — Deltamon is an "other
Digimon" and it was deleted — and the same observer demonstrably DOES fire on this
card for a battle deletion (`#effect#3`'s own line) and for an effect deletion of a
Tamer inside an ordinary resolution (`#effect#1`'s line); both are `confirmed`. What
is special here is that the cost deletion happens inside ANOTHER deletion's
replacement window.

The card YAML is not at fault: `cards/bt19/BT19-075.yaml` authors the replacement as
`kind: replacement` → `select_own_permanent` + `delete_permanent` +
`cancel_replacement` and the observer via the production `on_any_deletion` timing,
both faithful to `BT19_075.cs`. The clause's own witness also holds on both wires:
MoonMillenniummon did not leave, Deltamon did.

**Not fixed here** (verdict-recording stage). Logged as
`G-ENGINE-REPLACEMENT-COST-DELETION-NO-OBSERVER` in `docs/RUST_ENGINE_GAPS.md`, with
the mechanism marked UNCONFIRMED — the symptom is measured, the cause (queued
`OnAnyDeletion` entries never drained once `cancel_replacement` ends the outer batch)
is a hypothesis to verify before fixing.

> **SUPERSEDED (2026-09-21, later the same day — see the section below).** The
> UNCONFIRMED mechanism above was verified and is WRONG. The dispatch happens, the
> entry IS enqueued, and the deferred scope DOES flush it. The real cause is a
> dangling-handle alias from battle-area compaction, it is not replacement-specific,
> and the clause is now `confirmed`. Fixed by `4c609b5e2`.

**4 clauses: 3 confirmed, 1 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#2` TRIAGED and FIXED: **confirmed**, and the cause was NOT what the gap predicted

The `diverged` verdict above is resolved. Re-diffing the line against the **preserved**
sidecar `20260921T041928Z_40f7887a` (zero Unity time) after the engine fix reports
**CLEAN — compared 22 of 22 ours / 22 dcgo steps**. Verdict re-recorded `confirmed`.

**Not a scenario artefact.** The slot-safety check was run first, as the triage brief
requires: the re-authored line's digivolve and attack both landed on Millenniummon on
both wires (no `p0.field[..].sources` row anywhere in the diff), and the three `select:`
rows travelled by card identity. The single divergence was real.

**The predicted MECHANISM was wrong; the symptom was right.** Two resumes of this file
blamed "a `kind: replacement` cost `delete_permanent` takes a path with no
`on_any_deletion` dispatch". Instrumenting the actual line disproves that at both ends:

- The cost deletion DOES reach `drain_batch_on_any_deletion`, and it DOES enqueue an
  `OnAnyDeletion` entry for BT19-075 slot 3. Nothing about the dispatch is missing.
- The entry was then **dropped by the queue's condition filter**
  (`effect_queue.rs::non_firing_queued_effect_indices_for`) because the clause's
  `event_permanent_is_source: false` gate evaluated FALSE.

**The real cause: a dangling `PermanentHandle` aliased by battle-area compaction.**
`PermanentHandle` is `(player, battle-area INDEX)` and the battle area COMPACTS on
removal. `drain_batch_on_any_deletion` enqueues the deleted permanent's **pre-trash**
handle, but by then the survivors have already slid down. In this line p0 fielded
`[0]=Monodramon, [1]=Deltamon, [2]=MoonMillenniummon`; deleting Deltamon at index 1 slid
MoonMillenniummon into index 1, so the trigger's `event_permanent` `{0,1}` and the
observer's `source_permanent` `{0,1}` compared EQUAL — the carrier looked like the thing
that had just been deleted, and its own "other Digimon" gate rejected it. The trace:

```
non_firing? card=BT19-075 slot=3 passes=false
  src_perm=Some(PermanentHandle { player: 0, index: 1 })
  subject=Some(Permanent(PermanentHandle { player: 0, index: 1 }))
```

**This is not replacement-specific at all** — the replacement was only the reason the
deleted Digimon happened to sit BELOW the carrier. The minimal reproducer needs no
replacement, no security check and no attack: place an ally at index 0, the carrier at
index 1, delete the ally, and the observer silently misses it
(`bt19_075_observer_fires_when_deleted_ally_sits_below_the_carrier`, RED before the fix).
That also explains why the sibling clauses stayed `confirmed`: `#effect#1`'s Tamer
deletion and `#effect#3`'s battle deletion both delete from a HIGHER index or from the
opponent's side, where no alias is possible. It is a **pool-wide** defect of every
`[All Turns] When other Digimon or Tamers are deleted` observer, not a BT19-075 one.

**The fix** (`code/digimon-engine/src/dsl_cards/predicate.rs`, `event_permanent_is_source`):
handle equality alone is not identity once the event permanent is gone. When the trigger
carries a `deleted_object` snapshot AND this effect's own carrier is still standing at
`source_permanent` (it still holds `rctx.source_card`), the equality is the compaction
alias and `is_source` is false. Every other case keeps the plain comparison — an observer
whose carrier IS the deleted permanent finds no live carrier at that slot and still reads
`is_source == true`, so `bt19_075_carrier_own_deletion_does_not_fire_observer` is unmoved.

**Citations.** `general_rule.pdf` (Ver.3.6) **15-8-3-1** — a trigger-type effect "will
always trigger as soon as its trigger conditions are met, then the effect will activate";
**15-8-3-2** — it merely "can't activate during the processing for a rule or effect";
**15-8-3-5** — it is then "pending activation"; **15-8-3-6** — "Once an effect triggers,
it will remain triggered". Deferring the activation is licensed; discarding it is not.
**15-8-5-4** documents this card's exact replacement shape ("[Main] Delete 1 of your
opponent's Digimon" causing "[All Turns] When this Digimon would be deleted, by deleting
1 other Digimon with [Sukamon] in its name, it isn't deleted") as ordinary immediate-type
processing that the cause resolves out of. DCGO agrees: every deletion runs through
`CardEffectCommons.DeletePeremanentAndProcessAccordingToResult`, which raises
`OnDestroyedAnyone` regardless of seating order.

**Gate.** Four new tests in `tests/cards_behavioral/bt19/bt19_075.rs` — the minimal
index-order reproducer plus three shapes of the exam line (top-level, nested inside an
opponent Option's `[Main]`, and the full attack → security-check → P-180 `[Security]`
line). Only the first was RED before the fix, which is itself the finding: the other
three pass either way because they seat the carrier at index 0. Full `cards_behavioral`
**8227 passed / 0 failed / 37 ignored**.

**4 clauses: 4 confirmed, 0 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**
