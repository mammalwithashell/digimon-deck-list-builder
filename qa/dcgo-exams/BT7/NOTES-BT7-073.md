# NOTES — KaiserLeomon (BT7-073)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT7-073`):
**2 clauses** — `effect#0`, `effect#1`.
DCGO script: `BT7/Purple/BT7_073.cs` exists — the card is **not** `unavailable`.
Both clauses have a scenario; neither is `unreachable`.
Book: `qa/dcgo-exams/BT7/tm_bt7_073_pool.json` (decks `tm-mimi-purple`, `quiet-opponent`; identical to `tm_bt7_071_pool.json`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT7-073#effect#0` | `You may digivolve this card from your hand onto one of your purple Tamers as if the Tamer is a level 3 purple Digimon for a memory cost of 2.` | `BT7-073-effect0.yaml` | lowers, asserts pass |
| `BT7-073#effect#1` | `[When Digivolving] If a card with [Hybrid] in its traits or [Koichi Kimura] is in this Digimon's digivolution cards, this Digimon gains <Retaliation> until the end of your opponent's next turn.` | `BT7-073-effect1.yaml` | lowers, asserts pass |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

Both lowered and asserted as found.

- `effect#0`: same shape as `BT7-071-effect0.yaml` — the Tamer route
  (`BT7_073.cs`: purple Tamer, cost 2) is the only legal route onto Mimi
  (printed circles Purple Lv.3 / 3 and Purple Lv.4 / 1 need a level), and
  the clause's own cost (2) differs from the Lv.3 circle's 3, so the gauge
  (3 → 1) is the clause. The `[When Digivolving]` does not activate (only
  Mimi under it; DCGO `CanActivateCondition` false, ours `condition:` false).
- `effect#1`: Loweemon BT7-071 (trait [Hybrid]) goes onto Mimi on T3;
  KaiserLeomon digivolves over it on T5 through its Purple Lv.4 / 1 circle
  (single route — the Tamer route needs a Tamer top card). DCGO
  `OnEnterFieldAnyone`, `SetUpActivateClass(..., -1, false, ...)`,
  `CanActivateCondition` = a digivolution card with the [Hybrid] trait or
  named Koichi Kimura → `GainRetaliation(self, UntilOpponentTurnEnd)`;
  mandatory, no prompt. The keyword is then FORCED to show itself: the
  permanent (Mimi's frame, on the field since T1) attacks the player on T5
  and is left suspended; on T6 Vermilimon BT4-014 (8000) attacks it, wins,
  and <Retaliation> — mandatory (keyword-semantics 16-12; DCGO
  `RetaliationEffect` is `isOptional: false`) — deletes Vermilimon. Witness:
  both battle areas empty, Vermilimon in P1's trash beside the flipped
  Biyomon.

## Second resume (2026-09-18): re-lowered; slot hygiene checked

Every file for this card re-lowered unchanged against the current harness
(no digivolve into BT25-085, no `<De-Digivolve>`, no `<Link>` on this card).
Slot hygiene (the campaign's centre-out rule, `../BT25/NOTES-BT25-083.md`:
`field.N` in `attack:` / `digivolve:` / `main:` is a COMPACT frame-order index
on DCGO, reversed against ours on a two-Digimon board): every slot-addressed
step here acts on a seat holding exactly ONE permanent (P0: Mimi's frame,
first as a Tamer and then as the Digimon stacked on it; P1: at most one
Vermilimon), so `field.0` names the same permanent on both engines. `select: { targets: }`
rows ride the wire by top-card identity and are unaffected.
