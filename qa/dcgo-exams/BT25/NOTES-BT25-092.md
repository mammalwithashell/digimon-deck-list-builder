# NOTES — Asuna Shiroki (BT25-092)

Denominator: **3 clauses** (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids BT25-092`).

| Clause | Scenario | Status |
|---|---|---|
| `BT25-092#effect#0` | `BT25-092-effect0.yaml` | authored, lowers sim-only |
| `BT25-092#effect#1` | — | **was unreachable; now authorable** (see below) |
| `BT25-092#effect#2` | `BT25-092-effect2.yaml` | authored, lowers sim-only |

## `BT25-092#effect#1` — unreachable (SUPERSEDED 2026-09-17)

> **Update 2026-09-17.** `G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST` is
> RESOLVED (`qa/resolved-gaps.md`): the clause is now authored in the YAML
> (`when: main_on_field`, one `select_union_zone{hand,material}` cost prompt,
> optional own-Digimon pick gated by `has_digivolve_candidate`, optional
> `select_union_zone{hand,trash}` result pick gated by `can_digivolve_onto`,
> `effect_initiated_digivolve` cost −1) and the Tamer's `FIELD_EFFECT` bit is
> live in the mask. The exam line sketched below is now lowerable; the
> `unreachable` verdict must be re-derived by authoring `BT25-092-effect1.yaml`
> (a later exam stage — not done here). Expect the sim side to fold each DCGO
> `generic_bool` zone menu + per-zone pick into ONE union prompt (the BT25-085
> convention) when mapping the DCGO selection rows.

The original measured reason (kept for the record):


Printed text: *"[Main] By suspending this Tamer and trashing 1 Option card from
your hand or your Digimon's digivolution cards, 1 of your Digimon may digivolve
into a Digimon card with [Three Musketeers] in its text or the [TS] trait in the
hand or trash with the cost reduced by 1."*

**Measured reason.** Our engine does not implement this clause at all. The
card's YAML spec (`code/digimon-engine/cards/bt25/BT25-092.yaml`) carries it
as a `BLOCKED (dsl)` comment, tracked as DSL gap
`G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST`: no lowering exists for
(a) a digivolve whose result card is chosen from the union of hand and trash,
nor (b) a cost trash drawn from the union of the hand and a Digimon's
digivolution cards. With no effect lowered, the engine offers no
`FIELD_EFFECT` action for the Tamer, so a `main: { on: field.N }` step has
nothing in the live mask to lower to — the scenario cannot be lowered, and
`--sim-only` would refuse it before any Unity time was spent. No legal line in
our engine reaches the clause.

DCGO's side IS authorable (BT25_092.cs `#region Main`: suspend cost ->
`generic_bool` hand/digivolution-cards menu (only when both hold an Option) ->
SelectHandEffect / SelectTrashDigivolutionCards -> SelectPermanentEffect
(canNoSelect: true) -> `generic_bool` hand/trash menu (only when both hold a
candidate) -> DigivolveIntoHandOrTrashCard with `reduceCostTuple: (1, null)`),
so once the DSL gap is closed the exam line is: play Asuna, have a Digimon on
the field, an Option in hand, and a same-colour Lv.4 with a single distinct
cost in hand (Scopemon BT21-071 over a Purple Lv.3 keeps both circles at 2 and
avoids the SelectCountEffect route).

Verdict to record: `unreachable` — "clause not implemented in our engine
(BLOCKED, G-DSL-DIGIVOLVE-FROM-UNION-WITH-SOURCE-TRASH-COST); no lowerable line".
