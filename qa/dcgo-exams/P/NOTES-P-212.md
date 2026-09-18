# NOTES — Asuna Shiroki (P-212)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids P-212`):
**4 clauses** — `effect#0`, `effect#1`, `effect#2`, `effect#3`.
DCGO script: `P/Purple/P_212.cs` exists — the card is **not** `unavailable`.
No `SetIsBackgroundProcess(true)`. No verdict stored yet.

**Status: 4 clauses, 4 scenarios, 0 unreachable.** The four files the crashed
stage left were audited against `P_212.cs` and re-lowered unchanged on
2026-09-18 with `tm_p_212_pool.json` (decks `tm-asuna-p212`,
`quiet-opponent`). No BT25-085 digivolution, no `<De-Digivolve>`.

| Clause | Text | Scenario |
|---|---|---|
| `P-212#effect#0` | `[Start of Your Main Phase] If your opponent has a Digimon, gain 1 memory.` | `P-212-effect0.yaml` (T5 opens 3 → 4) |
| `P-212#effect#1` | `[On Play]` (the extractor's timing half; empty text) | `P-212-effect1.yaml` |
| `P-212#effect#2` | `<Draw 1> and trash 1 card in your hand. If this effect trashed a card with the [Three Musketeers] or [TS] trait, delete 1 of your opponent's level 3 Digimon.` | `P-212-effect2.yaml` (same line; the trashed card is the second Asuna, a [TS] Tamer → the rider deletes Biyomon) |
| `P-212#effect#3` | `[Security] Play this card without paying the cost.` | `P-212-effect3.yaml` |

`effect#1` / `effect#2` are ONE printed sentence the extractor splits at the
`<Draw 1>` keyword; the two files run the same line and differ only in
`clause:`.

## Adversarial pre-Unity review (from `P_212.cs`)

- `[Start of Your Main Phase]` is `Gain1MemoryTamerOpponentDigimonEffect`:
  `SetUpActivateClass(…, -1, false, …)` — mandatory, no prompt.
- `[On Play]` is `isOptional: false` → no `OptionalSkill`.
  `DrawAndDiscardCards` draws FIRST (`DrawClass`), then ONE `SelectHandEffect`
  (`maxCount` 1, `canNoSelect: false`). The rider's `SelectPermanentEffect`
  (`Mode.Destroy`, `canNoSelect: false`) opens only when the trashed card
  `HasThreeMusketeersTraits || HasTSTraits` AND the opponent has a level-3
  Digimon — asked with the single candidate Biyomon in `effect1`/`effect2`,
  never in `effect0`/`effect3` (Dracomon ST1-04 is trashed there).
- `[Security]` is `PlaySelfTamerSecurityEffect` (mandatory, no prompt) and it
  plays with `activateETB: true`, so **the security play fires her [On Play]**:
  the DEFENDER (`actor: 0`, on p1's turn) draws 1 and answers the mandatory
  `SelectHandEffect`. `effect3` carries that row; without it the line would
  desynchronize on the wire.
- Two Asunas sit in hand in `effect1`/`effect2`, so the play is pinned
  `from: hand.0`.
