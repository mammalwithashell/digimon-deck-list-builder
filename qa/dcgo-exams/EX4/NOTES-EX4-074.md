# NOTES — ShineGreymon: Ruin Mode (EX4-074)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX4-074`):
**3 clauses** — `effect#0` (the "[ShineGreymon]: Cost 4" digivolution
condition), `effect#1` ([When Digivolving] [On Deletion] -5000 DP), `effect#2`
([End of Attack] delete / <Recovery +1 (Deck)> / hatch). `EX4/Purple/EX4_074.cs`
exists, so nothing is `unavailable`.

| Clause | Scenario | Sim-only |
|---|---|---|
| `EX4-074#effect#0` | `EX4-074-effect0.yaml` | lowers, asserts pass |
| `EX4-074#effect#1` | `EX4-074-effect1.yaml` | lowers, asserts pass |
| `EX4-074#effect#2` | `EX4-074-effect2.yaml` | lowers, asserts pass |

One line serves all three (see the shared header in each file); each file
asserts the observable that belongs to its clause.

## Audit of the crashed agent's files (resumed campaign)

The three files were on disk, uncommitted, and none of them lowered:

1. `stack:` named **ST1-04 Dracomon**, which is not in `tm-ruin-red`
   (`tm_ex4_074_pool.json`). The harness refuses to stack a card the named
   deck does not contain. Replaced by ST1-13 (4 copies in the deck) at both
   positions — one is a never-flipped security slot, the other a draw the
   line never reaches.
2. `assert.at` was `32` on a 31-step line (0-based, so the last step is
   `30`). Re-pointed at the trailing T14 breeding decision, step `30`.
3. The Tamer assertion entries omitted `dp`; the projection renders a Tamer
   with `dp: -1`, and the sequence comparison is a rendered-string multiset,
   so the entry must carry it.

Nothing in the prompt sequence needed changing on review against
`EX4_074.cs`: both DP arms are `isOptional: false` with no pick; the
[End of Attack] chain is one `SelectPermanentEffect(Destroy, canNoSelect:
false)` over two opposing Digimon (a real decision; note that under the
harness even a single-candidate pick consumes a scripted row — the forced
pick short-circuit lives in `SelectPermanentEffect.cs`'s human-UI branch
only, and both seats take the AI branch, as `P/P-180-effect1.yaml`
documents), then `IRecovery(1)` and `HatchDigiEggClass` with no prompt.

## Card-YAML observation (not a fix, not a verdict)

`code/digimon-engine/cards/ex4/EX4-074.yaml` marks the [End of Attack]
opposing-Digimon pick `optional: true` to survive an empty opposing board.
The printed clause has no "may", and DCGO's pick is `canNoSelect: false`
(mandatory whenever a candidate exists). On this line the pick is taken, so
the wire row is identical on both sides and the flag is invisible to the
oracle; but as an action-space matter it over-exposes a decline the rules do
not grant (rule 17 / the DSL optional-on-mandatory pitfall). A
`select_opponent_permanent` that is mandatory-when-possible and skips
cleanly on zero candidates would remove it. Logged here for the card owner;
outside this exam's remit.

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
