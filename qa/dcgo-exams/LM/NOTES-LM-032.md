# NOTES — Purple Scramble (LM-032)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids LM-032`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `LM/Purple/LM_032.cs` exists — the card is **not** `unavailable`.
All three clauses have a scenario; none is `unreachable`.

Book: `qa/dcgo-exams/LM/tm_lm_032_pool.json` (decks `tm-purple-line`,
`tm-quiet-red`) — a per-card copy of the two decks the crashed agent had
put in the shared `tm_lm_pool.json` (which LM-056's lines also use and
which is not this card's to edit). Run every LM-032 line with `--decks`
pointing at the per-card book.

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `LM-032#effect#0` | `[Main] 1 of your purple Digimon may digivolve into a purple Digimon card in the hand with the digivolution cost reduced by 3. Then, place this card in the battle area.` | `LM-032-effect0.yaml` | lowers, asserts pass (repaired, below) |
| `LM-032#effect#1` | `[Start of Your Turn] If your opponent has a Digimon, <Delay>. ・Return 1 purple Digimon card from your trash to the top of the deck. Then, if you don't have a Digimon, you may play 1 purple Digimon card with 2000 DP or less from your trash without paying the cost.` | `LM-032-effect1.yaml` | lowers, asserts pass (repaired, below) |
| `LM-032#effect#2` | `[Security] You may play 1 purple Digimon card with 2000 DP or less from your trash without paying the cost. Then, add this card to the hand.` | `LM-032-effect2.yaml` | lowers, asserts pass (repaired, below) |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

All three were on disk, uncommitted. None passed sim-only as found:

1. **`effect0`** lowered but its hand assertion was one card short: the T3
   *breeding-area* digivolve draws a card like any digivolve, so the T5
   Youkomon digivolve draws `stack[13]` (ST1-05), not `stack[12]`. Assertion
   and the deck-accounting comment corrected; nothing in the line changed.
2. **`effect1`** did not lower at all — its `stack:` named ST1-04 three
   times against a deck holding two (`stacked card ST1-04 is not in deck`).
   The third copy sat at `stack[15]`, a position the line never draws; it was
   dropped. Then the second T7 attack was written as `field.1` after
   Youkomon's stack had already been deleted by the first — Gabumon is at
   slot 0 by then (`field.0`). Two assertion errors followed: the T9 draw is
   **Youkomon**, not `stack[14]` (see the ordering finding below), and the
   two Security Digimon that won P1's T7 battles (Phoenixmon, Birdramon) are
   in P1's trash — a checked security card always leaves the stack.
3. **`effect2`** lowered but asserted Birdramon at 5000 on P1's own turn;
   Agumon ST1-03 beneath it prints the inherited `[Your Turn] +1000 DP`, so
   both engines read 6000. Assertion corrected.

## `effect#1` — the Delay resolves BEFORE the draw (verified both sides)

`general_rule.pdf` 6-2-1-1 / 15-16-11: `[Start of Your Turn]` effects
activate before the unsuspend phase, and the draw phase (6-3) comes after.
DCGO agrees literally — `TurnStateMachine.ActivePhase()` runs
`StackSkillInfos(null, EffectTiming.OnStartTurn)` and drains it, then
unsuspends, and `DrawPhase()` is a separate later coroutine. Our engine
agrees (the lowered line's hand ends in ST6-07). Consequence for authors:
a card this <Delay> returns to the deck top **is the turn's draw**, so a
`stack[...]` slot after it is never reached that turn. The line's hand
assertion pins this.

## `effect#1` — prompt sequence re-derived from `LM_032.cs` (OnStartTurn region)

`SetUpActivateClass(null, ..., -1, TRUE, ...)` → OptionalSkill, gated by
`IsOwnerTurn && IsExistOnBattleArea(self) && Enemy.GetBattleAreaDigimons()
.Count >= 1 && CanDeclareOptionDelayEffect(card)` (not the placing turn).
Neither engine inspects the trash before asking, so the T7 gate opens with
an empty trash on both sides and is **declined** (a `select: { decline: true }`
row — our engine's cancel, DCGO's OptionalSkill "no"). On the T9 acceptance,
in order: `DeletePeremanentAndProcessAccordingToResult` (LM-032 trashed, no
prompt); `SelectCardEffect(Root.Trash, canNoSelect: false, maxCount 1)` over
purple Digimon cards — three candidates (Youkomon / DemiDevimon / Gabumon), a
real pick; `AddLibraryTopCards`; then `GetBattleAreaDigimons().Count == 0` →
`SelectCardEffect(Root.Trash, canNoSelect: true, maxCount 1)` over purple
Digimon with DP ≤ 2000 — Gabumon (2000) is the only candidate (DemiDevimon is
4000), and under the harness `SelectCardEffect.Activate()` always routes the
AI branch's `AutoSelect()` through `SetTargetCardAndIndicies` →
`InputDriver.TryAnswerStep`, so the row is consumed even with one candidate;
`PlayPermanentCards(payCost: false)`. Ours: the `kind: delay` gate, then
`select_trash optional: false` + `move_trash_card_to_deck_top`, then the
`if not any_permanent` branch's `select_trash optional: true` +
`play_from_trash_free`. 1:1 at every row.

## `effect#0` / `effect#2` — prompt sequences

- `effect#0` (`LM_032.cs` OptionSkill region): `SetUpActivateClass(null, ...,
  false, ...)` — no OptionalSkill. `HasMatchConditionPermanent` guard, then
  ONE `SelectPermanentEffect(Custom, maxCount 1, canNoSelect: true)` over own
  purple Digimon that some purple hand card can legally digivolve, then
  `DigivolveIntoHandOrTrashCard(isHand: true)` → `SelectHandEffect(canNoSelect:
  isOptional = true)`, then `PlayCardClass(payCost: true)` — with Youkomon's
  single circle (Purple Lv.3 / 2, −3 → 0) `CostList.Distinct()` has one entry
  and `CardController.cs` opens **no** `SelectCountEffect`. Then
  `PlaceDelayOptionCards`. Ours: `optional: true` + `select_own_permanent
  (optional)` → `select_hand` → `effect_initiated_digivolve` → auto-placement.
  Two picks on both sides.
- `effect#2` (SecuritySkill region): `HasMatchConditionOwnersCardInTrash`
  guard, then `PlayByEffect(Root.Trash, canNoSelect: true)` — ONE
  `SelectCardEffect` for the DEFENDER — then `AddThisCardToHand`. Ours: the
  inherited `on_security` effect's `optional: true` folds onto its declinable
  `select_trash`, one prompt. 1:1.

## Card-YAML finding (not a fix, not a verdict): `[Main]` hand pick with no digivolving Digimon

Measured at lowering on 2026-09-18 with a probe derived from
`BT3/BT3-096-effect0.yaml` (Mimi on the field, **no purple Digimon on the
field**, a purple Digimon — DemiDevimon ST6-02 — in hand, LM-032 used):
our engine parks a **non-optional Hand pick** after the empty permanent
pick (lowering reports `recorded cancel on a non-optional Hand prompt` when
the step is answered with a decline, at the step where DCGO asks only Mimi's
OptionalSkill). `code/digimon-engine/cards/lm/LM-032.yaml`
runs `select_hand` unconditionally after `select_own_permanent (optional:
true)`; with zero permanent candidates the binding is empty, the hand pick
is still parked, and `effect_initiated_digivolve` then has no target. DCGO
never asks: `LM_032.cs` wraps the whole digivolve in
`if (HasMatchConditionPermanent(CanSelectOwnPermanentCondition))`, and even
that condition requires a hand card that can legally digivolve the
permanent, so the hand prompt exists only when a target exists. The parked
prompt changes no reachable state, so under rule 17's *choice* test it is an
over-exposure of the same family as the `OrderedPermutation`-at-N=1 row in
`docs/DCGO_EXAM.md`. Suggested fix for the card owner: guard the hand pick
and the digivolve on the binding (an `if: { binding_exists: target }` /
equivalent), or gate the pick on `has_digivolve_candidate` (the leaf
`ceebcb6e3` added). **Not applied here** (exam stage: triage and report).
Consequence for authors today: any line that uses LM-032 with no legal
target must keep the user's hand free of purple Digimon cards, which is
exactly what `BT3-096-effect0.yaml` does.

## Slot hygiene pass (2026-09-18, second resume) — `effect#1` re-authored

The campaign's centre-out rule (`../BT25/NOTES-BT25-083.md`) was measured
after the audit above: `attack: field.N` reaches DCGO as a COMPACT index into
`GetFieldPermanents()` (frame-id order) and Digimon are seated at frames 4,
3, 5, …, so on a two-Digimon board our play-order slots are DCGO's reversed.
The audited `effect#1` fielded Youkomon + Gabumon on T7 and attacked
`field.0` twice. On DCGO the first attack would have been Gabumon (into
Phoenixmon) and the second Youkomon 6000 into Birdramon 5000 — which Youkomon
WINS, leaving P0 with a Digimon and the "if you don't have a Digimon" arm
closed. A scenario artefact, not a finding.

Re-authored so P0 never has two Digimon at once: Gabumon is played alone on
T3, attacks and dies alone on T5 (P1's `stack[9]` is now Birdramon, `stack[8]`
Phoenixmon), THEN DemiDevimon is played and LM-032 used (3 → 1 → −1; the
`[Main]` resolves and the −1 gauge hands the turn over, P1 opens T6 on 1 and
passes to 3). Youkomon is P0's only Digimon when it attacks on T7; LM-032 is
an Option permanent (back row on DCGO, sorted after every Digimon; second in
play order on ours), so `field.0` is the Digimon on both. Costs: the `[Main]`
target pick now has ONE candidate (DemiDevimon) — still a consumed row on both
sides (`SelectPermanentEffect`'s AI branch, `P/P-180-effect1.yaml`); the T7
gate is declined with Gabumon already in the trash (the decline is for the
"no Digimon" arm, not for an empty trash). T9's sequence and candidates are
unchanged (three-candidate return pick, Gabumon the lone ≤2000 play). 27
steps, witness at step 26; hand/trash/security asserts unchanged.

`effect#0` and `effect#2` address only one-Digimon boards (`effect#0`'s
`own.field.0` is a `select:` target, which rides the wire by identity) and
were re-lowered unchanged.
