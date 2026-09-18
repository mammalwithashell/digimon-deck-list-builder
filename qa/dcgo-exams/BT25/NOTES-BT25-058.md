# NOTES — Callismon (BT25-058)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-058`):
**6 clauses** — `effect#0` … `effect#5`.
DCGO script: `BT25/Green/BT25_058.cs` exists — the card is **not** `unavailable`.
All six clauses have a scenario; none is `unreachable`. No verdict is stored
yet: every clause is `unmeasured` until an oracle run (this stage emitted no jobs).

Book: `tm_bt25_058_pool.json` (decks `tm-callismon`, `quiet-opponent-tm`).

| Clause | Scenario | Sim-only (2026-09-18) |
|---|---|---|
| `BT25-058#effect#0` `[Digivolve] Lv.5 w/[TS] trait: Cost 4` | `BT25-058-effect0.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-058#effect#1` `<Reboot>` | `BT25-058-effect1.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-058#effect#2` `<Blocker>` | `BT25-058-effect2.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-058#effect#3` `<Fortitude>` | `BT25-058-effect3.yaml` | **one row added** — lowers, asserts pass |
| `BT25-058#effect#4` `[On Play] [When Digivolving] [When Attacking] [Once Per Turn]` suspend + lock | `BT25-058-effect4.yaml` | lowers, asserts pass (audited, unchanged) |
| `BT25-058#effect#5` `[All Turns] [Once Per Turn] When effects play or digivolve any Digimon …` | `BT25-058-effect5.yaml` | **re-authored** — lowers, asserts pass |

## Prompt shapes read off `BT25_058.cs`
- `effect#4`: `ActivateClassesForSharedEffects(…, optional: false, …)` — NO
  OptionalSkill. Two `SelectPermanentEffect`s, both behind
  `HasMatchConditionPermanent` (silent on an empty opposing board): the suspend
  pick is `canNoSelect: true`, the can't-unsuspend pick `canNoSelect: false`.
- `effect#5`: `SetUpActivateClass(…, 1, false, …)` — mandatory. De-Digivolve
  pick is `Mode.Degenerate`, `canNoSelect: false`; the battle pick is
  `canNoSelect: true`. Gated on `IsByEffect`: a hand-paid digivolve never fires
  it, which is why `effect#0`–`#4` (all hand digivolves) see ONE trigger and no
  MultipleSkills.

## `effect#3` — the predicted MultipleSkills row (added in pre-Unity review)
`<Fortitude>` replays Callismon BY AN EFFECT. The fresh permanent's `[On Play]`
(`effect#4`) and its own `[All Turns]` (`effect#5`: `CanTriggerOnPermanentPlay`
+ `IsByEffect`; `CanActivate` is only `IsExistOnBattleAreaActivate`) are then
both active in one window ⇒ DCGO asks `MultipleSkills` (`MultipleSkills.cs`:
`IsSkippable` only makes the panel declinable). Both bodies are silent (p1's
board is empty; each guards its picks), so the order cannot change state. Our
engine parks nothing there. Authored `dcgo_only`, `ordinal: 0`,
`candidates: [BT25-058, BT25-058]`. **Unverified until an oracle run:** if DCGO
stacks only one skill the run aborts on this row and says so — drop the row.

## `effect#5` — re-authored for slot safety
The draft bred DemiDevimon P-198, hard-played Aegiochusmon, moved DemiDevimon
out, then ran `digivolve: { from: field.0, using: BT25-058 }` on a TWO-Digimon
board. `field.N` reaches DCGO as a compact index in frame order and Digimon
seat centre-out (frames 4, 3, 5, …), so on DCGO `field.0` was DemiDevimon, not
the [TS] Lv.5 (`NOTES-BT25-083.md`). Now p0 never addresses a slot: DemiDevimon
(3) and Callismon (13; 7 → −6 is legal, the opposing gauge reads 6) are both
hard-played, DemiDevimon's trigger digivolves itself, and every Callismon pick
is an opposing permanent carried by identity. p1 overshoots with Phoenixmon on
T2 so p0 opens T3 on 7 and DemiDevimon's "4 or less memory" trigger
(`P_198.cs` `CanActivate`: `MemoryForPlayer <= 4`; ours `memory_lte: 4`) stays
silent until Callismon exists — confirmed sim-side (the line lowers through T3
with no prompt).

*Witness weakness, stated:* the `<De-Digivolve 1>` target is a bare Lv.3, so
nothing is trashed; the battle (Biyomon deleted) is what the state shows. The
bare target is deliberate — against a real stack DCGO's `IDegeneration`
consumes a `SelectCountEffect` row (`NOTES-BT21-074.md`); with no digivolution
cards `MaxCount` is 0 and `Activate()` skips the prompt. A stronger line would
De-Digivolve a two-card stack and carry a `dcgo_only` `value: 1` row.

*Finding reported by the first draft, NOT re-measured here:* with a Lv.3
DemiDevimon in the BREEDING area and ≤ 4 memory, our engine was observed to
park DemiDevimon's start-of-main digivolve pick before the breeding step;
`P_198.cs` gates it on `IsExistOnBattleAreaDigimon` (breeding-area effects do
not activate, 3-4-6). The re-authored line no longer passes through that
position, so it neither confirms nor refutes it. Worth a DebugRunner test.
