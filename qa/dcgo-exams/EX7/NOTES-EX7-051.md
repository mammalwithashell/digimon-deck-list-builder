# NOTES — Sparrowmon (EX7-051)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-051`):
**3 clauses** — `effect#0`, `effect#1`, `inherited#0`. DCGO script:
`EX7/Purple/EX7_051.cs` exists — the card is **not** `unavailable`. All three
clauses have a scenario; none is `unreachable`. No verdict is stored yet
(`qa/qa-reports/exam-verdicts/EX7-051.json` does not exist): all three are
`unmeasured` until the oracle pass.

| Clause | Scenario | Book | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-051#effect#0` — `[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0` | `EX7-051-effect0.yaml` | `tm_ex7_051_pool.json` (`tm-kapurimon-egg`) | lowers, asserts pass |
| `EX7-051#effect#1` — `[Start of Your Main Phase]` place a [Three Musketeers] Option → `<Draw 1>` | `EX7-051-effect1.yaml` | `three_musketeers_pool.json` | lowers, asserts pass |
| `EX7-051#inherited#0` — `<Retaliation>` | `EX7-051-inherited0.yaml` | `tm_ex7_051_pool.json` | lowers after the 2026-09-18 repair; **oracle divergence predicted** (below) |

## Why a per-card book

`effect#0` is measured as LEGALITY: the printed alt circle costs exactly what
the printed purple circle costs (0), so on a purple Lv.2 the two are
indistinguishable. Only two Lv.2 cards in the pool print "[Three Musketeers]"
— Kapurimon EX7-005 and Pagumon BT25-005 — and both are BLACK, so a purple
Sparrowmon reaches them only through the clause. `tm-kapurimon-egg` is the
shared 50 with the egg box swapped to Kapurimon EX7-005 ×4. Kapurimon is
implemented on both sides since `61fc7c7d9`; its trigger (an effect places a
[Three Musketeers] Option under it) does not fire on a digivolution in either
engine, and the one line that DOES place an Option under the stack
(`effect#1`) deliberately uses the shared book and its Pagumon ST6-01 egg.

## `inherited#0` — repaired line, and the engine finding it runs into

The file as left by the crashed stage no longer lowered: `step 21: our engine
asks a selection here … (pending kind: TriggerOrder, … 2 pending)`.

**Measured (2026-09-18).** When BlackGatomon (top of Kapurimon / Sparrowmon /
BlackGatomon) loses its battle to Vermilimon BT4-014 and is deleted, our
engine parks a two-candidate `TriggerOrder` whose candidates are
`[EX7-051 <Retaliation>, EX7-051]` — the keyword body and a second,
keyword-less entry sourced from the same card. DCGO has exactly ONE trigger
there (`RetaliationSelfEffect(isInheritedEffect: true)` under
`OnDestroyedAnyone`, `isOptional` false), so it opens no `MultipleSkills`
prompt. The repaired line answers our prompt
`{ cards: [EX7-051], trigger: Retaliation, sim_only: true }` (zero wire rows).

What the second entry IS was not identified from this stage.
`../BT25/BT25-078-inherited0.yaml` hit the same shape on Gazimon and attributed
it to the `[When Moving]` clause on that card being queued by a board-wide
`on_move` observer scan. Sparrowmon has no `[When Moving]`; its only other
clause is the `[Start of Your Main Phase]` one. The factor the two cards share
is an inherited `kind: grant_keyword` `Retaliation` clause, so that explanation
is at best incomplete — whoever picks this up should start from the queue
entry's `effect_slot`, not from `on_move`.

**The consequence is the real finding.** With `<Retaliation>` ordered FIRST,
our engine still leaves Vermilimon on the opposing field (8000 DP, suspended):

```
ASSERT FAILED: p1.field expected [] but our engine has BT4-014 dp 8000 suspended
ASSERT FAILED: p1.trash expected [BT4-014] but our engine has []
```

This is `G-ONDELETION-PARK-CLEARS-BATTLE-STATE` (`docs/RUST_ENGINE_GAPS.md`):
the park unwinds `delete_permanents_batch`, the attack finishes, and when the
keyword body finally resolves `pending_attack` is gone, so
`EffectContext::battle_opponent_of` returns `None` and the mandatory keyword
(§16-12) silently no-ops. That entry says ordering `<Retaliation>` first makes
it fire; on THIS board it does not — the park itself is enough — which is a
second data point for that entry. The spurious second trigger is what creates
the park here, so fixing either one would let the keyword through.

Consistent with `BT25-078-inherited0.yaml`, the witness of the clause itself
(`p1.field: []`, `p1.trash: [BT4-014]`) is deliberately NOT asserted: it would
only fail the sim-only gate for a known reason. The oracle run is expected to
report `diverged` on the P1 side at the trailing pass — DCGO deletes
Vermilimon, we do not. Triage: **our bug** (rules §16-12, DCGO
`Retaliation.cs`), already tracked; no card-YAML change is indicated
(`kind: grant_keyword` with `scope: inherited` is the pack-wide idiom for a
printed inherited keyword).

## `effect#1` — prompt shape re-derived from `EX7_051.cs` (2026-09-18)

`SetUpActivateClass(..., -1, true, ...)` → OptionalSkill; then
`SetBool(canSelectHand)` (a direct set, no RPC row) because only the hand holds
a candidate; `SelectHandEffect(canNoSelect: true)`; `SelectPermanentEffect`
(`canNoSelect: true`, one candidate, still a wire row under the AI branch the
harness forces); `AddDigivolutionCardsBottom` + `DrawClass(1)` inside the
permanent coroutine. Our clause is `optional: true` over a declinable union
pick, so our ONE prompt is gate and pick at once and the DCGO gate is a
`dcgo_only` row. The four-row sequence in the file matches.

`effect#0`, `inherited#0`, and every other line in this campaign that promotes
a Sparrowmon carry the mirror-image row: with no [Three Musketeers] Option in
hand or trash, DCGO still asks the OptionalSkill (`CanActivateCondition` is
merely "hand or trash non-empty and an own non-token Digimon"), while the
`condition:` on our clause gates the trigger on a payable Option and parks
nothing — the documented vacuous-gate class (`docs/DCGO_EXAM.md`, Known gaps),
answered with a `dcgo_only` decline.
