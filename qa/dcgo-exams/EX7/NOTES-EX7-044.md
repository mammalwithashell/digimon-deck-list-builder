# NOTES — Gigadramon (EX7-044)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-044`):
**3 clauses** — `effect#0`, `effect#1`, `inherited#0`. DCGO script
`EX7/Black/EX7_044.cs` exists — the card is **not** `unavailable`. No verdict is
stored yet: all three are `unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-044#effect#0` | `[Digivolve] Lv.4 w/[Three Musketeers] in text: Cost 3` | `EX7-044-effect0.yaml` (deck `tm-gigadramon-line`) | lowers, 6/6 asserts pass |
| `EX7-044#effect#1` | `[On Play] [When Digivolving] Reveal the top 4 … place 1 Option card with the [Three Musketeers] trait … If this effect placed, delete 1 … play cost of 3 or less.` | `EX7-044-effect1.yaml` ([On Play] arm, deck `tm-gigadramon`) | lowers, 8/8 (crashed agent's file, audited, unchanged) |
| `EX7-044#inherited#0` | `<Collision>` | `EX7-044-inherited0.yaml` (deck `tm-gigadramon-line`) | lowers, 11/11 |

Book: `qa/dcgo-exams/EX7/tm_ex7_044_pool.json`. `tm-gigadramon` is the crashed
agent's deck; `tm-gigadramon-line` (this stage) is the same list with Sparrowmon
EX7-051 ×4 and EX7-071 ×3 swapped for **Deputymon EX7-010** ×4 (the non-black Lv.4
that prints the trait name) and the vanilla black Lv.6 **HiAndromon BT2-064** ×3.

## The reveal cannot be avoided, so two lines keep it empty-handed
Gigadramon's `[On Play]` / `[When Digivolving]` is mandatory
(`SetUpActivateClass(..., -1, FALSE, ...)`), so every line that fields it pays the
reveal's rows. `effect0` and `inherited0` stack four NON-candidates on top:
`RevealDeckTopCardsAndSelect` computes `maxCount = min(1, 0) = 0` and opens no
`SelectCardEffect`, nothing is placed, the delete is skipped — but
`RemainingCardsPlace.DeckTopOrBottom` still asks a `generic_bool` ("Deck Top" =
true / "Deck Bottom" = false) and, with 4 ≥ 2 cards, one multi-pick ordering
`SelectCardEffect`. Ours asks an `EffectChoice` (by label) and one
`OrderedPermutation` pick per position. Those rows are split `sim_only` /
`dcgo_only`, the shape `EX7-044-effect1.yaml` already used (and the confirmed
`../BT25/BT25-078-effect1.yaml` uses for the same helper).

## `effect#0`
Printed circle is also "Black Lv.4 / 3" → base = Deputymon EX7-010 (RED). Legal only
through `TopCard.IsLevel4 && TopCard.HasText("Three Musketeers")` / our
`in_text_contains`; one route, no cost prompt. Deputymon is hard-PLAYED (no
`[On Play]`), never digivolved into and never attacks, so its vacuous DCGO-only
`OptionalSkill` (`NOTES-EX7-010.md`) does not come up. One Digimon on the board.
Asserted on P1's turn so Deputymon's inherited "[Your Turn] +2000 DP" is off.

## `effect#1` — audited
Single `[Three Musketeers]` Option (Der Blitz EX7-070) among the four → one SHARED
`SelectCardEffect`; remainder rows split as above; the "if placed" delete is a
SHARED `SelectPermanentEffect` on P1's only Digimon (Biyomon, play cost 2). The
`generic_bool` answer `decline: true` = `select_bool: false` = "Deck Bottom"
(`RevealLibrary.cs` `ReturnRevealedCardsToLibraryTopOrBottom`). No BT25-085, no
`<De-Digivolve>`: neither campaign-wide repair applies.

## `inherited#0`
Gigadramon under vanilla HiAndromon; attack the PLAYER while P1 holds one vanilla
Biyomon. DCGO `AttackProcess.cs` opens the block `SelectPermanentEffect` for the
defender with `canNoSelect: !AttackingPermanent.HasCollision` → mandatory; ours
parks a mandatory block window. Same row as `../BT25/BT25-100-effect7.yaml`.
Witness: `p1.security` stays 5 on an attack aimed at the player and a Blocker-less
Biyomon is in the trash. Keywords are not projected, so `<Blocker>` on Biyomon is
read through the forced block, not directly.
