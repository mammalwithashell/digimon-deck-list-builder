# NOTES — Chaos Triangular (EX7-066)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-066`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2` (the `[Security]`). DCGO script
`EX7/Red/EX7_066.cs` exists — the card is **not** `unavailable`. No verdict is stored
yet: all three are `unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-066#effect#0` | `When effects trash this card from digivolution cards, 1 of your Digimon gets +3000 DP until the end of your opponent's turn. While you have a [Three Musketeers] trait Digimon, you may ignore this card's color requirements.` | `EX7-066-effect0.yaml` (new) | lowers, 16/16 asserts pass |
| `EX7-066#effect#1` | `[Main] Delete 1 … 9000 DP or less. For each of your [Three Musketeers] trait Digimon with different names, add 3000 … Then, place this card as the bottom digivolution card …` | `EX7-066-effect1.yaml` (REPAIRED) | lowers, 12/12 |
| `EX7-066#effect#2` | `[Security] Delete 1 of your opponent's Digimon with 12000 DP or less.` | `EX7-066-effect2.yaml` (REPAIRED) | lowers, 12/12 |

Book: `qa/dcgo-exams/EX7/tm_ex7_066_pool.json` (deck `tm-chaos-triangular`, the
crashed agent's, unchanged: EX7-066 ×4 and Deputymon EX7-010 ×4 in the shared list).

## Both crashed drafts were broken; what was repaired
- **`effect1` no longer lowered** (`no legal action matches Play BT25-085`). It hosted
  the line on BeelStarmon BT25-085 played from hand; BT25-085 is a DUAL card whose
  Digimon face cannot be played (`20df249e6`, `../BT25/NOTES-BT25-083.md`). Re-hosted
  on **BeelStarmon (X Antibody) EX7-073** (12 cost, no `[On Play]`, carries the trait):
  P1 now has 9 memory on T4 (Phoenixmon, 10 → -1), P0 uses the Option at 1 → -5, and
  the stale P0 pass after the Option was dropped (-5 ends the turn). The draft's
  header note about a BT25-085 dual-card union-pick prompt is moot on this host and
  was removed with it.
- **`effect2` lowered but was not slot-safe.** It attacked with `field.1` on a
  TWO-Digimon board — exactly the artefact measured on the sibling
  `EX7-070-effect2.yaml` r1 (DCGO seats Digimon centre-out, so `field.1` is the OTHER
  Digimon there). Rewritten to that sibling's repaired three-Digimon line: the
  third-entered Digimon attacks (`field.2` on both sides). The clause now has THREE
  legal targets instead of two.

## `effect#0` — the line
Deputymon EX7-010 alone on the board (slot-safe). T5: Chaos Triangular's `[Main]`
finds no delete target (empty P1 board; both engines skip the pick) and tucks itself
under Deputymon, which holds the `[Three Musketeers]` TRAIT on P0's turn
(`EX7_010.cs` `ChangeTraitsClass`). T7: Deputymon attacks; its `[When Attacking]`
trashes the Option; **the clause** gives Deputymon +3000 (6000 → 9000) before the
security check against Vermilimon BT4-014 (8000). Three witnesses: the pick row
itself (Option in trash, DP still 6000), survival of the security battle (9000 >
8000; without the clause Deputymon dies), and 9000 still reading on P1's T8 row
("until the end of your opponent's turn").

Trasher choice: Deputymon's effect asks nothing after the trash, so the open
trigger-timing finding (ours resolves a source-trash trigger mid-effect, DCGO stacks
it — `../P/NOTES-P-180.md`) cannot show at any row. MagnaKidmon EX7-013's
End-of-Turn trash was rejected for exactly that reason (it still has an attacker
pick and an attack-target pick to ask).

The **ignore-colour sentence** is not exercised by `effect0` (Deputymon is red, like
the Option). `effect1` exercises it implicitly: the red Option is used with only the
PURPLE BeelStarmon (X) on the board, legal solely through it.

## `effect#1` — what it is built to catch
Phoenixmon (12000) is above the unscaled 9000 cap and deletable only through the
"+3000 per differently-named [Three Musketeers] Digimon" rider (one such Digimon).
An engine that ignores the rider offers no target and desynchronises at the delete
row. Not exercised: two SAME-named trait Digimon (the "different names" half) — it
needs two copies of one trait Digimon fielded, a much longer line; not claimed.
