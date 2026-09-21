# NOTES — Image Training (LM-056)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids LM-056`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `effect#3`.
DCGO script: `LM/Purple/LM_056.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 4 clauses, 4 scenarios, 0 unreachable.** The four files the crashed
stage left (2026-09-15) were audited against `LM_056.cs` and re-lowered
unchanged on 2026-09-18 with `tm_lm_056_pool.json` (decks
`tm-lm056-yellow-line`, `tm-lm056-quiet-red`). None digivolves into BT25-085
and none resolves a `<De-Digivolve>`.

| Clause | Text | Scenario |
|---|---|---|
| `LM-056#effect#0` | `While you don't have [Image Training] in the battle area, you can ignore this card's color requirements.` | `LM-056-effect0.yaml` — the play is legal ONLY through the clause: p0 owns nothing but a YELLOW Tsunomon BT24-003 in breeding |
| `LM-056#effect#1` | `[Main] Reveal the top 2 … Add 1 purple or blue card … place this card in the battle area.` | `LM-056-effect1.yaml` |
| `LM-056#effect#2` | `[Main] <Delay> - 1 of your Digimon may digivolve into a purple or blue Digimon card in the hand with the digivolution cost reduced by 2.` | `LM-056-effect2.yaml` |
| `LM-056#effect#3` | `[Security] Reveal the top 2 … place this card in the battle area.` | `LM-056-effect3.yaml` |

## Adversarial pre-Unity review (from `LM_056.cs`)

- `[Main]` / `[Security]`: `isOptional: false` → no `OptionalSkill`. ONE
  reveal condition (`HasCardColor(Purple) || HasCardColor(Blue)`) → one
  `SelectCardEffect`, `Mode.AddHand`; the revealed pair is Lekismon BT25-024
  (blue) + Patamon BT25-031 (yellow) → exactly one eligible card. The single
  leftover is bottomed with no DCGO prompt; our one-choice
  `OrderedPermutation` is answered `sim_only` (the documented row). One bucket
  → none of the multi-bucket add-timing quirk. `[Security]` runs through
  `AddActivateMainOptionSecurityEffect`, i.e. the same `[Main]` body, answered
  by the DEFENDER (`actor: 0` on p1's turn).
- `<Delay>`: `isOptional: false`; Option deleted first (no prompt), then
  `SelectPermanentEffect` (`canNoSelect: true`, single candidate Patamon still
  asked), then `DigivolveIntoHandOrTrashCard(isHand: true)` →
  `SelectHandEffect` (`canNoSelect: true`). Lekismon onto the YELLOW Patamon is
  legal only through "[Digivolve] Lv.3 w/[TS] trait: Cost 2" → 0 after the
  reduction: one cost, no cost-choice prompt.
- Lekismon's own `[When Digivolving] <Draw 1>` is `optional: false` (no
  prompt). Its `[Your Turn] When your Digimon are played or digivolve, if any
  of them are red …` trigger has `CanUse` true for its own digivolution but
  `CanActivateCondition` needs a [Crescemon] in the trash — false, so it is
  not offered and no `MultipleSkills` opens. Patamon BT25-031's `[On Play]`
  never fires (it is breeding-digivolved, then moved); Tsunomon BT24-003's
  inherited needs a security removal on p0's own turn.

Related measured asymmetry (not exercised here): our engine does not count a
bare Digi-Egg for an Option's colour requirement while DCGO does — see
`../P/NOTES-P-108.md`. `effect#0` is unaffected: its egg is the WRONG colour on
both sides, which is the point of the line.

## Oracle pass (2026-09-19, drain of 2026-09-18) -- breeding-label fix

The failed jobs aborted with `expected prompt 'breeding_action' but DCGO asked
'main_phase'` on a breeding-area digivolve. That is a scenario label defect, not
an engine divergence: with a Lv.2 in the breeding area (no hatch, no move) BOTH
engines open the turn on the main phase (the campaign convention, e.g.
BT21-054-effect0.yaml, oracle-CLEAN; the sim side does not assert phase-prompt
labels, which is why sim-only was green). The `expect.prompt` on those
`digivolve: { from: breeding }` steps is now `main_phase`; sim-only re-lowered
green. Those clauses stay `unmeasured` until re-drained.

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#2` re-authored SLOT-SAFE; `effect#3` re-audited unchanged

Both outstanding clauses were re-read against `LM/Purple/LM_056.cs` before any
Unity time.

### `effect#2` — the placed Option was at a DIFFERENT index on the two wires

Same latent defect as the sister line `../P/P-108-effect1.yaml`, and the same
repair. `main: { on: field.N }` is a MAIN-PHASE ACTION, so DCGO resolves the
slot itself: `ActivatePermanentAction(slot, …)` reads
`actor.GetFieldPermanents()[slot]` (`Script/Harness/InputDriver.cs:425-463`),
which walks `FieldPermanents[0..15]` in FRAME-ID order
(`Script/Player.cs:669`). `CardSource.PreferredFrame()`
(`Script/CardSource.cs:2290-2364`) seats Digimon on one row (y descending;
centre-out 4, 3, 5, 2, 6, …) and Tamers/Options on another (y ascending; 11,
10, 12, 9, …) — the enemy seat's `correctOrder`
`{4,3,5,2,6,1,7,0,8, 11,10,12,9,13,14,15}` spells both rows out — so **every
Digimon precedes every placed Option** in DCGO's compact list, while ours is
plain entry order. The old line played LM-056 on T1 and promoted Patamon on
T5, so `main: { on: field.0 }` named LM-056 for us and PATAMON for DCGO: the
oracle job would have aborted on "[Main] sub-slot on field slot 0 (BT25-031)
names no activatable [Main] effect". Never measured — the 2026-09-18 run
aborted earlier on the breeding label.

Repaired by promoting Patamon FIRST and playing LM-056 SECOND (both on T5), so
the Option is index 1 on both wires and the step is `main: { on: field.1 }`;
the `<Delay>` fires on T7. A pre-clause `assert:` now pins the ordering
(`p0.field: [BT25-031, LM-056]`). The line still plays LM-056 with only a
YELLOW Digimon on the board, so `effect#0`'s ignore-colour precondition is
unchanged. P1 no longer hatches (it declines breeding and passes every turn),
which removes its Agumon and keeps every seat at one permanent. The reusable
rule is in `../README.md` §"Slot safety".

### `effect#3` — the [Security] line was re-read and kept

`LM-056` sits at P0's `stack[9]` (the TOP security card) and P1's promoted
Agumon flips it on T6, so the clause is measured on the card actually being
checked, for the DEFENDER, on the attacker's turn. Re-derived from the C#:
`AddActivateMainOptionSecurityEffect` → `CardEffectFactory
.ActivateMainOptionSecurityEffect` (`Script/CardEffectFactory.cs:551-592`) is
`SetUpActivateClass(null, ActivateCoroutine, -1, FALSE, …)` and simply runs the
same `[Main]` `ActivateClass` — **no `OptionalSkill` on either the outer
security wrapper or the inner body**, so the whole clause is ONE
`SelectCardEffect` (the single blue card among the revealed pair) plus our
sim-only `OrderedPermutation` at N=1. Every `attack:` / `move:` step in the
file happens while P1 holds exactly one Digimon, and P0 holds none until
LM-056 is placed, so no slot is ambiguous. Unchanged; sim-only 8 checks / 0
failed.

Sim-only at close: all four LM-056 lines lower with `tm_lm_056_pool.json`
(`effect#0` 6/0, `effect#1` 6/0, `effect#2` 11/0, `effect#3` 8/0).
`effect#2` and `effect#3` stay **`unmeasured`** — only an oracle diff moves
them.

## 2026-09-21 (close-out `three-musketeers-2`) — ORACLE VERDICTS RECORDED: card CLOSED

`effect#2` and `effect#3` drained `completed` on oracle build `scripted-v16`
(DCGO `fc67f9ae6`, action-space digest `711d23bf12`) and both diffed **CLEAN**:

| Clause | Sidecar | Diff | Verdict |
|---|---|---|---|
| `LM-056#effect#2` | `20260921T043609Z_6794b105` | CLEAN, compared 19 of 20 ours / 19 dcgo (1 sim-only row) | **confirmed** |
| `LM-056#effect#3` | `20260921T043702Z_ece1cec6` | CLEAN, compared 14 of 15 ours / 14 dcgo (1 sim-only row) | **confirmed** |

The excluded row on each line is our own follow-on `select:` step, which answers a
prompt DCGO does not open; it is named in the denominator rather than netted out.

**4 clauses: 4 confirmed, 0 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**
