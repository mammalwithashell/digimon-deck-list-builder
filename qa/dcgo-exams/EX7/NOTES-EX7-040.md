# NOTES — ToyAgumon (EX7-040)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-040`):
**3 clauses** — `effect#0`, `effect#1`, `inherited#0`. DCGO script
`EX7/Black/EX7_040.cs` exists — the card is **not** `unavailable`. No verdict is
stored yet: `effect#1` and `inherited#0` are `unmeasured` until the oracle pass;
`effect#0` is **`unreachable`** (below).

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-040#effect#0` | `[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0` | — | **unreachable** — no line can separate it from the printed circle |
| `EX7-040#effect#1` | `[On Play] By trashing 1 card with the [Three Musketeers] trait in your hand, <Draw 2>.` | `EX7-040-effect1.yaml` | lowers, 6/6 asserts pass |
| `EX7-040#inherited#0` | `<Reboot>` | `EX7-040-inherited0.yaml` | lowers, 8/8 |

Book: `qa/dcgo-exams/EX7/tm_ex7_040_pool.json` (deck `tm-toyagumon-black`, the
crashed agent's, unchanged: the shared list with EX7-040 ×4 and the vanilla black
Lv.4 Jazardmon ST5-07 ×4).

## `effect#0` — unreachable: every Lv.2 that prints the trait name is BLACK

The clause can only be measured on a base that the printed circle (**Black Lv.2 /
cost 0**) refuses — a NON-black Lv.2 with "[Three Musketeers]" in its text — the
construction the sibling lines use (`EX7-043-effect0.yaml`, `EX7-044-effect0.yaml`,
`EX7-008-effect0.yaml`). Measured over the whole pool
(`grep -l Musketeers data/card_bundles/*.md` ∩ `- Level: 2`): exactly two Lv.2
cards print it, **Kapurimon EX7-005** and **Pagumon BT25-005**, and both are
**Black**. On either, the printed circle and the clause open together at the same
cost 0: no prompt separates them, the end state is identical, and the line would
read `confirmed` with the clause deleted from the card. A line that cannot fail is
not a measurement, so none is authored.

Second, independent reason the oracle could not answer even then: `EX7_040.cs`
implements the condition as `TopCard.IsLevel2 && TopCard.ContainsTraits("Three
Musketeers")` — a **TRAIT** test — where the card prints "**in text**" (and where
its siblings `EX7_043.cs` / `EX7_044.cs` use `HasText`). No Lv.2 card carries the
`[Three Musketeers]` trait (Kapurimon: Lesser; Pagumon: Lesser/Iliad/TS), so on
DCGO this route is never open to any existing card. Printed card data outranks
DCGO (CLAUDE.md source priority); our `alt_paths` uses `in_text_contains`, the
faithful reading. Recorded as a DCGO quirk, not a finding against our engine.

Reachable again if a non-black Lv.2 printing "[Three Musketeers]" is ever released.

## `effect#1` — audited, kept

`SetUpActivateClass(..., -1, TRUE, ...)` with `CanActivate` = a trait card in hand
→ `OptionalSkill`, then `SelectHandEffect(Mode.Discard, canNoSelect: true)`. Ours is
one declinable hand pick, authored with `expect: { prompt: OptionalSkill }` — the
OptionalSkill+pick fold (the emitter writes OptionalSkill(yes) + the pick). P-180
is trashed from the HAND, so its "trashed from digivolution cards" trigger is not
in play. Hand 5 → 4 → 6 is the `<Draw 2>` witness.

## `inherited#0` — audited, kept

ToyAgumon (T3) → vanilla Jazardmon ST5-07 on top (T5, one route, no cost prompt) →
attack the player → P0 passes → at P1's first T6 decision the stack reads
`suspended: false`. `RebootSelfStaticEffect(isInheritedEffect: true)` is static:
nothing is asked. One Digimon per seat, no digivolution into BT25-085, no
`<De-Digivolve>`: neither campaign-wide repair applies.

## 2026-09-21 (close-out `three-musketeers-2`) — `effect#0` `unreachable` RE-VERIFIED; card otherwise closed

No job was emitted for `EX7-040` this pass (no scenario lacks a `confirmed` verdict
except the `unreachable` one, which has no scenario by construction). The standing
`unreachable` was re-checked rather than carried forward on trust, because a stale
reason is worse than no reason.

**The reason still holds, re-measured against `data/cards.json` on 2026-09-21.** The
clause is `[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0`, and a line can only
witness it if some Lv.2 card printing "[Three Musketeers]" digivolves into EX7-040 at a
cost the printed circles do NOT already offer. Sweeping every card at `level: 2` whose
effect / inherited / security text contains "Three Musketeers" returns exactly two:

| Card | Colors | Why it cannot separate the clause |
|---|---|---|
| Kapurimon EX7-005 | `card_colors: [5]` (Black) | the printed Black Lv.2 cost-0 circle opens at the same cost |
| Pagumon BT25-005 | `card_colors: [5]` (Black) | same |

So every candidate line would confirm with the clause DELETED — it measures the
printed circle, not the alt path. Independently, `EX7_040.cs` tests `ContainsTraits`
(a TRAIT) where the card prints "in text", and no Lv.2 carries the `[Three Musketeers]`
trait, so DCGO never opens this route either (DCGO quirk; printed data outranks it).

Reachable the moment a **non-Black** Lv.2 printing `[Three Musketeers]` is released —
re-run this sweep when a new set lands rather than re-deriving it.

**3 clauses: 2 confirmed, 0 diverged, 1 unreachable (reason re-verified 2026-09-21), 0 unavailable, 0 unmeasured.**
