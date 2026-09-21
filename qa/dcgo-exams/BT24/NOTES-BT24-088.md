# NOTES — Asuna Shiroki (BT24-088)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT24-088`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `BT24/Purple/BT24_088.cs` exists — the card is **not** `unavailable`.
All three clauses have a scenario; none is `unreachable`. Book:
`qa/dcgo-exams/EX7/three_musketeers_pool.json`.

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT24-088#effect#0` | `[Start of Your Turn] If you have 4 or less memory, by returning this Tamer to the bottom of the deck, you may play 1 [Asuna Shiroki] or 1 level 4 or lower Digimon card … from your trash without paying the cost.` | `BT24-088-effect0.yaml` (the [Asuna Shiroki] name branch) | lowers, asserts pass |
| `BT24-088#effect#1` | `[On Play] By trashing 1 card with [Three Musketeers] in its text or the [TS] trait from your hand, <Draw 2>.` | `BT24-088-effect1.yaml` (cost PAID) | lowers, asserts pass |
| `BT24-088#effect#2` | `[Security] Play this card without paying the cost.` | `BT24-088-effect2.yaml` (P1 defends; card pinned at P1 `stack[9]`) | lowers, asserts pass |

Audited on resume (2026-09-18) against `BT24_088.cs`; re-lowered on the current
harness binary; no repairs needed.

## Prompt shapes (re-derived from the C#)

- `[Start of Your Turn]`: `SetUpActivateClass(CanActivate: on battle area &&
  MemoryForPlayer <= 4, …, -1, TRUE)` → `OptionalSkill`; then
  `DeckBouncePeremanentAndProcessAccordingToResult` (no prompt); then a
  `SelectCardEffect(Root.Trash, maxCount 1, canNoSelect: true)`; then
  `PlayPermanentCards(activateETB: true)`. Ours: `optional: true` whose first
  body step is the mandatory `activation_cost` → a single-candidate declinable
  `TriggerOrder` that lines up 1:1 with the `OptionalSkill` (`yes: true`), then
  the declinable `select_trash`.
- `[On Play]`: `SetUpActivateClass(CanActivate: hand >= 1, …, TRUE)` →
  `OptionalSkill`, then `SelectHandEffect(Mode.Discard, canNoSelect: true)`.
  Ours is ONE declinable hand pick → the `optional_gate_fold`
  (`expect: { prompt: OptionalSkill }` on the pick row; the emitter splits it
  into OptionalSkill(yes) + the pick).
- `[Security]`: `PlaySelfTamerSecurityEffect` — mandatory, no prompt — then the
  [On Play] above, which is the DEFENDER's decision on the attacker's turn.

## Card finding on `effect#1` — `<Draw 2>` is not gated on the cost being paid

Re-measured 2026-09-18 (probe: `BT24-088-effect1.yaml` with the fold row
answered `decline: true`): our engine trashes nothing and **still draws 2**
(hand after the decline = opening four + two LadyDevimon). `BT24-088.yaml`'s
`on_play` process is `select_hand { optional: true }` → `trash_from_hand_by_index`
→ `draw 2` with no `binding_present` guard, so declining the pick skips the
trash and falls through to the draw. `BT24_088.cs` draws only `if (discarded)`,
and the printed "By trashing …, <Draw 2>" is a cost (no cost, no effect).
Fix is one guard in the YAML (the `BT25-085.yaml` `binding_absent` idiom, or
drop the inner `optional:` and let the clause-level gate be the decline — the
shape `BT25-092.yaml`'s [Start of Your Main Phase] already uses). **Not applied
here** (exam stage); it belongs to the card-fix gate with a failing-then-passing
`cards_behavioral` test.

All three committed lines route AROUND the declined branch on purpose (every
[On Play] they cross is PAID), so the finding does not surface on them as a
hand divergence attributed to the wrong clause. A dedicated declined-cost line
would read `diverged` today; author it together with the fix.
