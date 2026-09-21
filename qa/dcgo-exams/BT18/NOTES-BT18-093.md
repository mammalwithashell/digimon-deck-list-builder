# NOTES — Violet Inboots (BT18-093)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT18-093`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `BT18/Purple/BT18_093.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 3 clauses, 3 scenarios, 0 unreachable.** The three files the crashed
stage left were audited against `BT18_093.cs` and re-lowered unchanged on
2026-09-18 with `tm_bt18_093_pool.json` (decks `tm-inboots`,
`quiet-opponent`). No BT25-085 digivolution, no `<De-Digivolve>`.

| Clause | Text | Scenario |
|---|---|---|
| `BT18-093#effect#0` | `[Start of Your Turn] If you have 2 or less memory, set it to 3.` | `BT18-093-effect0.yaml` — p0 opens T5 on 1 by a double overshoot (Inboots 3 → -1, then p1's Biyomon 1 → -1) |
| `BT18-093#effect#1` | `[Start of Your Main Phase] By trashing 1 Option card or 1 card with the [Ghost] or [Three Musketeers] trait in your hand, <Draw 1>.` | `BT18-093-effect1.yaml` — fodder Shadow Wing ST1-13 (an Option) |
| `BT18-093#effect#2` | `[Security] Play this card without paying the cost.` | `BT18-093-effect2.yaml` |

## Adversarial pre-Unity review (from `BT18_093.cs`)

- `[Start of Your Turn]` is `SetMemoryTo3TamerEffect` — mandatory, no prompt,
  gated on `MemoryForPlayer <= 2`. There is no rules-level memory reset
  (`general_rule.pdf` §6-2-1-1 quotes this very text as a CARD effect), which is
  why the line has to hand p0 a turn that really starts on 1.
- `[Start of Your Main Phase]` is `SetUpActivateClass(…, -1, true, …)` →
  `OptionalSkill`, offered only when the hand holds an eligible card
  (`CanActivateCondition`: `HasMatchConditionOwnersHand`), then ONE
  `SelectHandEffect` (`maxCount` 1, `canNoSelect: true`, `Mode.Discard`) whose
  after-select draws 1 only if a card was picked. Ours is ONE declinable
  `select_hand` (cost) → the documented `optional_gate_fold`: the pick row
  carries `expect: OptionalSkill`, the emitter writes `OptionalSkill(yes)` +
  the pick. In `effect0` / `effect2` p0's hand holds no Option / [Ghost] /
  [Three Musketeers] card at any of its main phases with the Tamer in play, so
  the gate never opens on either side.
- `[Security]` is `PlaySelfTamerSecurityEffect` — mandatory, no prompt; the
  Tamer has no `[On Play]`, so nothing follows (contrast `../P/NOTES-P-212.md`).
