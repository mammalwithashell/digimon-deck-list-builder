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
