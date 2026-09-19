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
