# NOTES — Psychemon (BT8-071)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT8-071`):
**1 clause** — `effect#0`.
DCGO script: `BT8/Purple/BT8_071.cs` exists — the card is **not** `unavailable`.
Book: `qa/dcgo-exams/BT8/tm_bt8_071_pool.json` (decks `tm-psyche-purple`, `tm-stingmon-blue`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT8-071#effect#0` | `[All Turns] Players can't reduce paid play costs.` | `BT8-071-effect0.yaml` | lowers, asserts pass |

## Audit of the crashed agent's file (resumed campaign, 2026-09-18)

Lowered and asserted as found. `BT8_071.cs` is a static
`CannotReduceCostClass` (every player; no target permanents, i.e. play
costs; `HasPlayCost`) — nothing is ever asked. The instrument is the
OPPONENT's Stingmon ST9-09, whose `ST9_09.cs` `ChangeCostClass` (`Cost -= 1`
when a blue Digimon is in play, `Root.Hand`) is likewise static and
prompt-free; with Gomamon ST2-02 (blue) on P1's field it would cost 3, and
with Psychemon on P0's field it must cost the printed 4. The witness is the
gauge (P1 pays 4 from 3 → −1, so P0 opens T5 on 1) AND the turn structure
(the turn hands over only because the full cost was paid). This is the same
printed text as Chikurimon ST13-08, which is `unavailable` in DCGO
(`../ST13/NOTES-ST13-08.md`); a `confirmed` here is evidence about the
shared engine substrate, not a verdict on ST13-08.

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
