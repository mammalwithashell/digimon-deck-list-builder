# NOTES — Digimon Emperor (BT8-094)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT8-094`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`.
DCGO script: `BT8/White/BT8_094.cs` exists — the card is **not** `unavailable`.
All three clauses have a scenario; none is `unreachable`.
Book: `qa/dcgo-exams/BT8/tm_bt8_094_pool.json` (decks `tm-emperor`, `quiet-opponent`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT8-094#effect#0` | `[All Turns] When any of your opponent's level 5 or lower Digimon are deleted, by suspending this Tamer, <Draw 1>.` | `BT8-094-effect0.yaml` | lowers, asserts pass |
| `BT8-094#effect#1` | `[Opponent's Turn] When any of your opponent's level 3 Digimon move from the breeding area to the battle area, gain 2 memory.` | `BT8-094-effect1.yaml` | lowers, asserts pass |
| `BT8-094#effect#2` | `[Security] Play this card without paying the cost.` | `BT8-094-effect2.yaml` | lowers, asserts pass |

## Audit of the crashed agent's files (resumed campaign, 2026-09-18)

All three lowered and asserted as found. Prompt sequences re-derived from
`BT8_094.cs`:

- `effect#0` (`OnDestroyedAnyone`, `SetUpActivateClass(..., -1, TRUE, ...)`):
  one OptionalSkill for P0 — asked on P1's turn, which is what `[All Turns]`
  buys — gated by `CanActivateSuspendCostEffect` (Tamer unsuspended); then
  `SuspendPermanentsClass` + `DrawClass(1)`, no pick. Ours: `optional: true`
  + `activation_cost: suspend_self` → one accept/decline park. The deletion
  is Biyomon ST1-02 (Lv.3) losing a security battle to Phoenixmon ST1-10
  (12000, vanilla) — no other trigger in play.
- `effect#1` (`OnMove`, `SetUpActivateClass(..., -1, false, ...)`):
  mandatory, `IsOpponentTurn` + `CanTriggerOnMove(Lv.3 opposing Digimon)`,
  `AddMemory(2)`; no prompt on either side. Witness is the gauge (P1 opened
  T6 on 3, reads 1 after promoting Biyomon).
- `effect#2`: `PlaySelfTamerSecurityEffect` — mandatory, no prompt; a Tamer
  has no DP so no battle follows. Standard security skeleton (`stack[9]`).

## Second resume (2026-09-18): re-lowered; slot hygiene checked

Every file for this card re-lowered unchanged against the current harness
(no digivolve into BT25-085, no `<De-Digivolve>`, no `<Link>` on this card).
Slot hygiene (the campaign's centre-out rule, `../BT25/NOTES-BT25-083.md`:
`field.N` in `attack:` / `digivolve:` / `main:` is a COMPACT frame-order index
on DCGO, reversed against ours on a two-Digimon board): every slot-addressed
step here acts on a seat holding exactly ONE Digimon, with any Tamer / Option
played AFTER it (back-row frames sort after every Digimon on DCGO), so
`field.0` names the same permanent on both engines. `select: { targets: }`
rows ride the wire by top-card identity and are unaffected.
