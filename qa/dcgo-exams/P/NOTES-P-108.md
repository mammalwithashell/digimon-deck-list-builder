# NOTES — Wisdom Training (P-108)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids P-108`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `P/Purple/P_108.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 3 clauses, 3 scenarios, 0 unreachable.** All three lower sim-only
with `tm_p_108_pool.json` (decks `tm-wisdom`, `quiet-opponent`).

| Clause | Text | Scenario |
|---|---|---|
| `P-108#effect#0` | `[Main] Reveal the top 2 cards of your deck. Add 1 purple card among them to the hand. Return the rest to the bottom of the deck. Then, place this card in the battle area.` | `P-108-effect0.yaml` |
| `P-108#effect#1` | `[Main] <Delay> … 1 of your Digimon may digivolve into a purple Digimon card in the hand with the digivolution cost reduced by 2.` | `P-108-effect1.yaml` |
| `P-108#effect#2` | `[Security] Place this card in the battle area.` | `P-108-effect2.yaml` |

## Adversarial pre-Unity review (from `P_108.cs`)

- `[Main]`: `SetUpActivateClass(null, …, -1, false, …)` → no `OptionalSkill`.
  `SimplifiedRevealDeckTopCardsAndSelect` with ONE condition → one
  `SelectCardEffect`; the single leftover is bottomed with no ordering prompt
  (our one-choice `OrderedPermutation` is answered `sim_only`, the documented
  `docs/DCGO_EXAM.md` row). One bucket → none of the multi-bucket add-timing
  quirk.
- `<Delay>`: `isOptional: false`; the Option is deleted first with no prompt,
  then `SelectPermanentEffect` (`canNoSelect: true`, single candidate still
  asked), then `DigivolveIntoHandOrTrashCard(isHand: true)` →
  `SelectHandEffect` (`canNoSelect: true`). Youkomon ST6-07's only circle is
  Purple Lv.3 / 2 → 0 after the reduction: one cost, no cost-choice prompt.
  Same shape as `../LM/LM-056-effect2.yaml`.
- `[Security]`: `PlaceSelfDelayOptionSecurityEffect` — mandatory, no prompt,
  and (unlike LM-056) it does **not** run the `[Main]`: no reveal.
- ST6-05 Elecmon / ST6-07 Youkomon are vanilla (no DCGO script); Pagumon
  ST6-01's inherited is `[On Deletion]` only.

## FINDING (engine, measured while authoring): a bare Digi-Egg does not satisfy an Option's colour requirement in our engine

First draft: hatch the purple Pagumon ST6-01 on T1 and play P-108 the same
turn. Lowering refused it — `step 1: no legal action matches Play { card:
"P-108" }` — with only the purple egg in the breeding area.

- DCGO allows it: `CardSource.MatchColorRequirement` (`CardSource.cs` ~305)
  walks `Owner.GetFieldPermanents()` and accepts any permanent whose
  `TopCard.IsPermanent`, and `CEntity_Base.IsPermanent` is
  `Digimon || Tamer || DigiEgg`. A hatched egg counts.
- Ours: `action/mask.rs::option_color_match_available` adds the breeding
  permanent's colours only `if breeding.is_digimon(card_data)`, and
  `Permanent::is_digimon` is `CardKind::Digimon | CardKind::Dual` — a
  `digi_egg` top card is excluded. (The pinning test
  `a_breeding_area_digimon_satisfies_the_color_requirement` uses a Lv.3.)

A Lv.2 in the breeding area is a Digimon for the rules (it is in play as an
In-Training Digimon), so DCGO's reading looks like the right one; not fixed
here. The committed lines breeding-digivolve into Elecmon first, which both
engines accept. The same asymmetry is what `../LM/LM-056-effect0.yaml` leans on
from the other side (its egg is the WRONG colour, so it is unaffected).

## SIDE FINDING (card YAML, not this card): ST1-02 / ST1-04 digivolve onto any Lv.2

The same failed lowering listed `Digivolve hand 2 onto breeding area` (Biyomon
ST1-02, red) and hand 3 (Dracomon ST1-04) as LEGAL over the PURPLE Pagumon:
`code/digimon-engine/cards/st1/ST1-02.yaml` authors its circle as
`from: { level_eq: 2 }` with no `color_is: red`. The printed circle is red.
Not exercised by any line here; recorded for the starter-deck owners.

## Oracle pass (2026-09-19, drain of 2026-09-18) -- breeding-label fix

The failed jobs aborted with `expected prompt 'breeding_action' but DCGO asked
'main_phase'` on a breeding-area digivolve. That is a scenario label defect, not
an engine divergence: with a Lv.2 in the breeding area (no hatch, no move) BOTH
engines open the turn on the main phase (the campaign convention, e.g.
BT21-054-effect0.yaml, oracle-CLEAN; the sim side does not assert phase-prompt
labels, which is why sim-only was green). The `expect.prompt` on those
`digivolve: { from: breeding }` steps is now `main_phase`; sim-only re-lowered
green. Those clauses stay `unmeasured` until re-drained.

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#1` re-authored SLOT-SAFE

Both outstanding clauses (`effect#0`, `effect#1`) were re-audited against
`P/Purple/P_108.cs` before any Unity time. `effect#0` needed no change (its
only multi-permanent moment carries no slot-addressed action). `effect#1` did,
and the defect was **latent, not measured** — the 2026-09-18 oracle run aborted
earlier, on the breeding label, so it never reached the step.

**What was wrong.** `main: { on: field.N }` is a MAIN-PHASE ACTION, so its
slot is NOT identity-resolved: DCGO builds `ActivatePermanentAction(slot, ...)`
and reads `actor.GetFieldPermanents()[slot]`
(`Script/Harness/InputDriver.cs:425-463`). `GetFieldPermanents()` walks
`FieldPermanents[0..15]` in FRAME-ID order (`Script/Player.cs:669`), and
`CardSource.PreferredFrame()` (`Script/CardSource.cs:2290-2364`) puts Digimon
and Tamers/Options on DIFFERENT rows — Digimon sorted by y descending (the
centre-out 4, 3, 5, 2, 6, ... row), Tamers/Options by y ascending (11, 10, 12,
9, ...), which the enemy seat's explicit `correctOrder`
`{4,3,5,2,6,1,7,0,8, 11,10,12,9,13,14,15}` spells out. **So in DCGO's compact
list every Digimon precedes every placed Option**, while ours is plain entry
order. The old line played P-108 on T3 and promoted Elecmon on T5, making the
Option our index 0 and DCGO's index 1; `main: { on: field.0 }` would have
named ELECMON on the oracle and aborted with "[Main] sub-slot on field slot 0
(ST6-05) names no activatable [Main] effect".

**The repair** (no clause content changed): Elecmon is promoted FIRST and
P-108 played SECOND, both on T5, so the Option is index 1 on both wires and
the step is `main: { on: field.1 }`; the `<Delay>` then fires on T7. A
pre-clause `assert:` block now pins the ordering itself
(`p0.field: [ST6-05, P-108]`) so a future re-order cannot go unnoticed. The
rule is written up for the next author in `../README.md` §"Slot safety".

Sim-only after the repair: `effect#0` 12 checks / 0 failed, `effect#1` 11
checks / 0 failed (book `tm_p_108_pool.json`). Both clauses remain
**`unmeasured`** — only an oracle diff can move them.

**Re-derived prompt shapes are unchanged** from the audit above: the `[Main]`
is `SetUpActivateClass(null, …, -1, false, …)` (no `OptionalSkill`), one
`SelectCardEffect` over the revealed pair, no ordering prompt at N=1; the
`<Delay>` deletes the Option with no prompt, then `SelectPermanentEffect`
(`canNoSelect: true`) then `SelectHandEffect` (`canNoSelect: true`, via
`CardEffectCommons.DigivolveIntoHandOrTrashCard` :1006-1031). Youkomon's single
Purple Lv.3 / 2 circle goes to 0 under the -2, so no `SelectCountEffect`.

## 2026-09-21 (close-out `three-musketeers-2`) — ORACLE VERDICTS RECORDED: card CLOSED

`effect#0` and `effect#1` drained `completed` on oracle build `scripted-v16`
(DCGO `fc67f9ae6`, action-space digest `711d23bf12`) and both diffed **CLEAN**:

| Clause | Sidecar | Diff | Verdict |
|---|---|---|---|
| `P-108#effect#0` | `20260921T043752Z_b3aa3e3d` | CLEAN, compared 8 of 9 ours / 8 dcgo (1 sim-only row) | **confirmed** |
| `P-108#effect#1` | `20260921T043827Z_e15fafbf` | CLEAN, compared 19 of 20 ours / 19 dcgo (1 sim-only row) | **confirmed** |

The single excluded row on each line is our own follow-on `select:` step, which
answers a prompt DCGO does not open — it is NAMED in the denominator, not netted
out, so "compared 8 of 9" is a complete accounting, not a shortfall.

**3 clauses: 3 confirmed, 0 diverged, 0 unreachable, 0 unavailable, 0 unmeasured.**
