# NOTES — Iron Slash (BT25-100)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-100`):
**8 clauses** — `effect#0` … `effect#7`.
DCGO script: `BT25/Black/BT25_100.cs` exists — the card is **not** `unavailable`.
**7 of 8 clauses have a scenario; `effect#5` is `unreachable`** (measured reason
below). No verdict is stored yet: the seven authored clauses are `unmeasured`
until an oracle run (this stage emitted no jobs).

Book: `tm_bt25_100_pool.json` (decks `tm-red-attacker`, `tm-blue-defender`).

Clause map (positional split of the official text, `data/card_bundles/BT25-100.md`):

| Clause | Printed | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `effect#0` | `<Use Req. ([TS] trait)>` | `BT25-100-effect0.yaml` | **new** — lowers, asserts pass |
| `effect#1` | `[Security] Activate this card's [Main] effects.` | `BT25-100-effect1.yaml` | count row added — lowers, asserts pass |
| `effect#2` | `[Main]` (marker; empty clause text) | `BT25-100-effect2.yaml` | count row added — lowers, asserts pass |
| `effect#3` | `<De-Digivolve 2>` 1 of your opponent's Digimon. Then, you may link … | `BT25-100-effect3.yaml` | count row added — lowers, asserts pass |
| `effect#4` | Link DP `DP+2000` | `BT25-100-effect4.yaml` | count row added, DP witness pinned — lowers, asserts pass |
| `effect#5` | Link Condition `<Link> [TS] trait: Cost 2` | — | **`unreachable`** (below) |
| `effect#6` | Link Effect `<Collision>` | `BT25-100-effect6.yaml` | count row added — lowers, asserts pass |
| `effect#7` | Link Effect `<Piercing>` | `BT25-100-effect7.yaml` | count row added — lowers, asserts pass |

## What the pre-Unity review changed in the six older files
1. **The `<De-Digivolve 2>` COUNT row (would have aborted every line).**
   `SelectPermanentEffect.cs:1008` builds `new IDegeneration(target, 2, cardEffect)`
   with no `DegenerationCountRuling`, so `Degeneration()` opens `SelectCountEffect`
   "How many cards do you trash?" with `MaxCount = min(digivolution cards, 2)`,
   `CanNoSelect: false`. Against the four-card Vermilimon stack that is
   candidates `{1, 2}` — a REAL choice (general_rule 16-11 "trash up to x",
   16-11-6 "the declared number"). Our engine parks nothing and always takes
   the maximum — the tracked rule-17 gap **G-DEDIGIVOLVE-NO-DECLARATION**
   (`docs/RUST_ENGINE_GAPS.md`). Each line now carries a `dcgo_only`
   `value: 2` row after the target pick (actor = the effect's owner, p1), and
   every later assert index moved by one. Against a bare Lv.3 (`effect0`) there
   is no row: `MaxCount` 0 and `Activate()` skips the prompt.
2. **Stale header text.** (a) The "PREDICTED DIVERGENCE AT THE LINK ROW" is
   resolved — engine 35958972b (G-ENGINE-SECURITY-OPTION-LINK-TO-OWN-DIGIMON)
   lifts a security-flipped Option onto the ordinary LinkSelectHost path.
   Re-measured sim-side: the link row is a LIVE OwnField pick, Kamemon reads
   3000 afterwards and p1's trash is empty. (b) "the scenario vocabulary has no
   way to answer [the mode-select]" is outdated — the `choice:` select form
   (harness d5f23792f) answers it; `effect0` uses it.
3. `effect4` (Link DP) now pins the actual witness (`p1.field` Kamemon
   `dp: 3000`, `p1.trash: []`); its asserts had been thinned out because of the
   stale divergence prediction.

Slot safety: in every line each seat addresses at most one permanent (p0 one
stack, p1 Kamemon; a linked Option is not a separate permanent).

*Scope note:* `effect1`, `effect2` and `effect3` all reach the `[Main]` body
through `[Security]`. That is the right route for `effect#1`; for `effect#2`
(the `[Main]` marker) a from-hand use would be the more literal witness —
`effect0` is that line and could be pointed at by a sister file.

## `effect#5` — `unreachable`: no shared action means "plug this Option in from hand"
The Link Condition's COST (2) is only ever paid by a hand declaration — the
`[Main]`'s own link is "without paying the cost" — and an Option has no
battle-area origin to link from (it is trashed or linked on use; no `<Delay>`).
Measured 2026-09-18:
- `link: { card: BT25-100, from: hand }` → **`no legal action matches Link`**.
  The hand-link `HAND_EFFECT` bit exists for Link DIGIMON only
  (`Game::activate_hand_link`, G-ENGINE-DIGIMON-LINK-FROM-HAND).
- On OUR side the Option plug-in lives on the PLAY bit behind the mode-select:
  `play:` + `choice: "Plug in"` parks the OwnField host pick (scratch probe).
- On DCGO the same declaration is a separate `ActivateCardAction` over the
  `LinkEffect`, which the harness emits only from a `HAND_EFFECT` bit
  (`InputDriver.BuildMainPhaseAction`); a Play-bit wire row is `PlayCardAction`
  = the `[Main]` use.
So one wire integer means two different things and no line can mean "hand
plug-in" on both engines. To make it reachable either the engine exposes the
Option hand-link on the `HAND_EFFECT` bit (as it does for Link Digimon), or the
emitter translates Play + branch 1 into DCGO's `ActivateCardAction`. The HOST
GATE half of the condition ([TS] Digimon) is exercised indirectly by every
line that links to Kamemon. Same clause, same reason: `BT25-093#effect#4`.

See also `NOTES-BT25-091.md`, F-ENGINE-PLUGIN-MODE-SELECT-WITHOUT-HOST — our
mode-select was offered (and its "Plug in" branch executed, paying 2 and
trashing the card) with NO legal Digimon host. Found with this card, **FIXED
2026-09-20 (`f24b986d0`)**: `option_legal_play_modes` now drops the Link mode
when `link_host_candidates` is empty (general_rule.pdf §10-1-3-1; DCGO
`Link.cs:24`). That closes the OFFERING half only — this clause's
`unreachable` reason above (no shared wire action means "plug this Option in
from hand") is unchanged.

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

`BT25-100-effect5.yaml` is on disk and lowers sim-only clean (10 steps,
`Action(30)` = the `HAND_EFFECT` bit, 14 assertions green). The line is
deliberately minimal so the only thing it reads is the Link Condition box:

  T1 P0 hard-plays Kamemon BT24-019, the pool's [TS] Digimon (3: 0 -> -3);
  T2 P1 passes out; T3 P0 declares the from-hand `<Link>` for **2**
  (3 -> 1, so the turn stays P0's), host = Kamemon, then passes.

Witness: memory 1 (the printed cost 2 paid), `p0.hand` without BT25-100,
`p0.trash: []` (a linked Option is not trashed) and the host at 3000 DP
(1000 + the #effect#4 Link DP box, pinned as proof the link landed).

Wire rows, adversarially re-derived from the C# before commit:
1. ONE `main_phase` row. `CardSource.CanDeclareSkillList` is
   `EffectList(EffectTiming.OnDeclaration)` filtered to `ActivateICardEffect`
   — for Iron Slash that is ONLY `LinkEffect` (its `[Main]` is registered at
   `EffectTiming.OptionSkill` and is reached by `PlayCardAction`), so
   `InputDriver.FindHandDeclarableSkillIndex` returns the link and the bit
   becomes `ActivateCardAction(card, 0)`. Our side: no `[Hand] [Main]`, so
   `Game::hand_effect_slot_is_link` is true and the same bit is the link.
2. `dcgo_only` accept — `Link.cs:29` builds the declaration
   `SetUpActivateClass(null, ActivateCoroutine, -1, TRUE, ...)`, so DCGO asks
   an `OptionalSkill` yes/no BEFORE the host pick; our engine treats the
   declaration itself as that decision. Same row `../BT21/BT21-074-inherited0.yaml`
   needed after its first oracle run aborted on it.
3. SHARED `SelectPermanentEffect` "Select 1 Digimon to link." (`Link.cs:70-88`,
   maxCount 1, `canNoSelect: false`) — asked on both wires even at one
   candidate.
4. Nothing else: BT25-100 has NO `[When Linking]` effect (its regions are Link
   Condition / Link / Use Requirement / Link Inherit / Main / Security).

No colour question: Iron Slash is BLACK and P0's board is BLUE, but the Option
use-requirement / colour gate belongs to general_rule.pdf §9-1 "Using Cards"
and a link declaration is not a use — `LinkEffect.CanUseCondition`
(`Link.cs:49-62`) checks only the origin zone and the host set, and
`hand_option_link_condition_targets` documents the same reading on our side.

Slot hygiene: one Digimon on P0's board, none on P1's, for the whole line.

**`effect#5` moves `unreachable` -> `unmeasured`**: it needs an oracle pass
against `D:/dcgo-build/scripted-v16`, not a tooling change. Denominator now
**8 clauses: 7 + 1 authored-unmeasured, 0 unreachable.**

## Oracle run 2026-09-21 — `effect#5` MEASURED: `unreachable` retired, now `diverged`

The stored `unreachable` reason ("no shared wire action means 'plug this Option
in from hand'") **no longer holds** and has been overwritten in
`qa/qa-reports/exam-verdicts/BT25-100.json`. `BT25-100-effect5.yaml` ran
`completed` (sidecar
`20260921T042552Z_0bad16abd58e4f80bf7cb98eb9f1ae40.state.jsonl`).

`--all-diffs` reports ONE row, at the host pick — the same shape as the sister
clause `BT25-093#effect#4`, with the price the only difference:

```
DIVERGED at step 7 (9 of 10 ours / 10 dcgo steps)
  memory:  ours=1 dcgo=3
  p0.hand: ours=[BT1-028,BT1-028,BT2-024,BT3-020]
           dcgo=[BT1-028,BT1-028,BT2-024,BT25-100,BT3-020]
```

`ours=1` is the printed **Cost 2** paid (3 → 1) — the witness this file was
built to read — against BT25-093's `ours=0` for its Cost 3. Every later row
matches: host at 3000 DP, the Option in neither hand nor trash, P0 still holding
the turn.

**Triage:** not a cost-value divergence but a payment-ORDER one, and the PDF
backs DCGO — `general_rule.pdf` §10-1-3-1..3 (p.19) pays the link cost AFTER the
host is chosen, while our Plug-In Option branch pays at declaration. Logged as
`G-ENGINE-OPTION-HAND-LINK-COST-TIMING` in `docs/RUST_ENGINE_GAPS.md`.

**Denominator: 8 clauses — 7 confirmed, 1 diverged, 0 unreachable, 0
unavailable, 0 unmeasured.**
