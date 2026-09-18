# NOTES — Loweemon (BT7-071)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT7-071`):
**1 clause** — `effect#0`.
DCGO script: `BT7/Purple/BT7_071.cs` exists — the card is **not** `unavailable`.
Book: `qa/dcgo-exams/BT7/tm_bt7_071_pool.json` (decks `tm-mimi-purple`, `quiet-opponent`).

| Clause | Text | Scenario | Sim-only |
|---|---|---|---|
| `BT7-071#effect#0` | `You may digivolve this card from your hand onto one of your purple Tamers as if the Tamer is a level 3 purple Digimon.` | `BT7-071-effect0.yaml` | lowers, asserts pass |

## Audit of the crashed agent's file (resumed campaign, 2026-09-18)

Lowered and asserted as found. `BT7_071.cs`:
`AddSelfDigivolutionRequirementStaticEffect(TopCard purple && IsTamer,
digivolutionCost: 2, ignoreDigivolutionRequirement: false, condition: card
in owner's hand)`. The base is Mimi Tachikawa BT3-096 (purple Tamer, no
level), so the printed Purple Lv.3 / 2 circle cannot apply and the clause
is the only route: one cost, no `SelectCountEffect` / `EffectChoice`.
Loweemon has no `[When Digivolving]`; Mimi as a source has no inherited
text. The digivolve is the only action after the T1 Tamer play; the witness
is Loweemon standing over Mimi with 2 paid (memory 3 → 1, then the pass).

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
