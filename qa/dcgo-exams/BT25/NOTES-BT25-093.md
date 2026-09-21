# NOTES — Ignition Flare (BT25-093)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-093`):
**6 clauses** — `effect#0` … `effect#5`.
DCGO script: `BT25/Red/BT25_093.cs` exists — the card is **not** `unavailable`.
**5 of 6 clauses have a scenario; `effect#4` is `unreachable`** (measured reason
below). No verdict is stored yet: the five authored clauses are `unmeasured`
until an oracle run (this stage emitted no jobs).

Book: `tm_bt25_093_pool.json` (decks `tm-red-attacker`, `tm-blue-defender`).

| Clause | Printed | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `effect#0` | `<Use Req. ([TS] Trait)>` | `BT25-093-effect0.yaml` | **new** — lowers, asserts pass |
| `effect#1` | `[Security] Activate this card's [Main] effects.` | `BT25-093-effect1.yaml` | header refreshed — lowers, asserts pass |
| `effect#2` | `[Main]` Delete all … lowest DP. If this effect didn't delete, trash 1 … Option. Then, you may link … | `BT25-093-effect2.yaml` | header refreshed — lowers, asserts pass |
| `effect#3` | Link DP `DP+2000` | `BT25-093-effect3.yaml` | header refreshed — lowers, asserts pass |
| `effect#4` | Link Condition `<Link> [TS] trait: Cost 3` | — | **`unreachable`** (below) |
| `effect#5` | Link Effect `[When Attacking] [Once Per Turn] Delete 1 … with as much DP as this Digimon or less.` | `BT25-093-effect5.yaml` | lowers, asserts pass (audited, unchanged) |

## Prompt shapes read off `BT25_093.cs`
- `[Main]`: `SetUpActivateClass(null, …, -1, FALSE, …)` — no OptionalSkill. The
  lowest-DP delete is PROMPT-FREE (`FindAll(IsMinDP)` →
  `DeletePeremanentAndProcessAccordingToResult`). The Option-trash pick
  (`canNoSelect: false`) runs only in `DeletionFailureProcess`. Then ONE link
  pick (`canNoSelect: TRUE`), guarded by `HasMatchConditionPermanent`.
  No `<De-Digivolve>`, so none of the count-row problem BT25-100 had.
- Link effect: `SetUpActivateClass(CanActivate, …, 1, FALSE, …)` — mandatory,
  one `SelectPermanentEffect` (`canNoSelect: false`) over opposing Digimon with
  DP ≤ this Digimon's. `effect5` measures the boundary (3000 vs 3000).
On a security flip the card's owner is the DEFENDER, so "your opponent's
Digimon with the lowest DP" is the attacking stack — it is deleted with no
prompt and the single select after the attack is the link pick. That matches
the one `Select` each of the four older lines lowers to.

Slot safety: each seat addresses at most one permanent in every line.

## What the pre-Unity review changed
- `effect1` / `effect2` / `effect3` headers carried a "PREDICTED DIVERGENCE AT
  THE LINK ROW" that is resolved (engine 35958972b,
  G-ENGINE-SECURITY-OPTION-LINK-TO-OWN-DIGIMON; `effect5` already said so).
  Replaced with the resolved wording; steps unchanged.
- `effect0` is new: the from-hand `[Main]` use of a RED Option off a BLUE [TS]
  Digimon (p0 runs `tm-blue-defender`, no red permanent anywhere), legal only
  through the clause. Our `[Main]`/`[Link]` mode-select is answered
  `choice: "[Main]"`, `sim_only` (DCGO's Play bit IS the `[Main]` use); the
  link pick is declined so the line stays clear of the Link boxes.

## Not measured by any line (stated, not skipped)
The failure branch of `effect#2` — "If this effect didn't delete, trash 1 of
your opponent's Option cards in the battle area" — needs an opponent with NO
Digimon (or an undeletable lowest-DP one) AND an Option in their battle area
(a `<Delay>`-placed Option). The quiet pools hold no such Option; a per-card
book with one would reach it. The clause id is covered by `effect2`'s success
branch, so this is a sub-branch gap inside a covered clause, not a missing
clause.

## `effect#4` — `unreachable`: same reason as `BT25-100#effect#5`
The Link Condition's cost (3) is only paid by a HAND declaration (the `[Main]`
links "without paying the cost"; an Option has no battle-area origin). Measured
on the sister card 2026-09-18 (`NOTES-BT25-100.md`): `link: { from: hand }`
finds **no legal action** for a Plug-In Option — our engine puts the Option
plug-in on the PLAY bit behind the mode-select, while DCGO's is a separate
`ActivateCardAction` that the harness emits only from a `HAND_EFFECT` bit. One
wire integer, two meanings: no line can mean "hand plug-in" on both engines.
Our mode-select here reads `[0: "Play as a [Main] Option" | 1: "Plug in via
Link Requirements (Cost 3)"]` (printed by the `effect0` lowering), so the
sim-side path exists; only the shared wire action does not.

## 2026-09-20 — the Option hand plug-in is now its OWN main-phase action

`G-TOOLING-EXAM-OPTION-HAND-LINK` is **closed**, and it was closed on the
ENGINE side — the option these notes preferred — so no DCGO change was needed
(close-out job `three-musketeers-2`).

**The rules reading.** `general_rule.pdf` (Ver.3.6) §6-5-1 lists the main-phase
actions, and **§6-5-1-3 "Use an Option Card From the Hand"** and **§6-5-1-4
"Linking a Card in the Hand or Battle Area"** are two SEPARATE entries; §10-1-1
says a card is linked from the hand "by paying the cost **as part of the main
phase actions**". So plugging a Plug-In Option in from hand is its own
declaration, not a mode of using the card. DCGO already drew that line:
`CardEffectFactory.LinkEffect` (`Link.cs:19-24`) accepts ANY `CardSource` with
a `linkCondition` that `IsExistOnHand`, so the declaration sits in the card's
`CanDeclareSkillList` and `InputDriver.FindHandDeclarableSkillIndex` reaches it
as an `ActivateCardAction` — never a `PlayCardAction`.

**What changed in the engine** (`G-ENGINE-OPTION-LINK-FROM-HAND`,
`docs/RUST_ENGINE_GAPS.md`): `hand_option_link_condition_targets` /
`hand_option_link_available` put the Plug-In Option's from-hand link on the
`HAND_EFFECT` bit, exactly where the Link DIGIMON's already lived
(`G-ENGINE-DIGIMON-LINK-FROM-HAND`, `cf8e2519f`); `activate_hand_link` routes
an Option through `play_option_core` in `OptionPlayMode::Link`; and
`option_legal_play_modes` drops the Link mode for a HAND source so the PLAY bit
means only §6-5-1-3 and the declaration is not exposed twice. A
`[Security]`-flipped Option keeps its Link mode — it has no main-phase
declaration to make.

**For this file**: the clause is reachable now. Author it as

```yaml
- actor: 0
  do: { link: { card: <THIS CARD>, from: hand } }
- actor: 0
  do: { select: { targets: [own.field.N] } }
  expect: { prompt: SelectPermanentEffect, count: 1 }
```

— the same two-step shape every other `link:` line uses, on the same wire
integer on both engines. It needs no new DCGO build (`scripted-v15` already
dispatches a `HAND_EFFECT` bit to the hand card's link declaration); the
scenario itself has not been written yet, so the clause is `unmeasured`, not
`unreachable`.

### Scenario authored 2026-09-20 (close-out job `three-musketeers-2`)

`BT25-093-effect4.yaml` is on disk and lowers sim-only clean (10 steps,
`Action(30)` = the `HAND_EFFECT` bit, 14 assertions green). It is deliberately
the SAME line as the sister clause `../BT25/BT25-100-effect5.yaml`, so the only
reading that differs between the two files is the printed price:

  T1 P0 hard-plays Kamemon BT24-019, the pool's [TS] Digimon (3: 0 -> -3);
  T2 P1 passes out; T3 P0 declares the from-hand `<Link>` for **3**
  (3 -> 0, still P0's turn), host = Kamemon, then passes.

Witness: memory 0 (the printed cost 3 paid), `p0.hand` without BT25-093,
`p0.trash: []` and the host at 3000 DP (1000 + the #effect#3 Link DP box).

Wire rows (re-derived from the C#): ONE `main_phase` row for the declaration
(`CanDeclareSkillList` holds only `LinkEffect` — the `[Main]` is an
`EffectTiming.OptionSkill` effect reached by `PlayCardAction`); a `dcgo_only`
`OptionalSkill` accept (`Link.cs:29` `SetUpActivateClass(..., isOptional:
true, ...)`); the SHARED `SelectPermanentEffect` host pick (`Link.cs:70-88`,
maxCount 1, `canNoSelect: false`). BT25-093 has NO `[When Linking]` effect —
its Link ESS is `[When Attacking]` — so the line ends at the host pick.

No colour question: Ignition Flare is RED and P0's board is BLUE, but the
Option use-requirement / colour gate is general_rule.pdf §9-1 "Using Cards"
and a link declaration is not a use (`Link.cs:49-62` checks only origin zone
and host set). The card's own `<Use Req. ([TS] trait)>` (#effect#0) is
measured from hand by `BT25-093-effect0.yaml` and is NOT what makes this line
legal.

Slot hygiene: one Digimon on P0's board, none on P1's, for the whole line.

**`effect#4` moves `unreachable` -> `unmeasured`**: it needs an oracle pass
against `D:/dcgo-build/scripted-v16`. Denominator now **6 clauses: 5 + 1
authored-unmeasured, 0 unreachable.**
