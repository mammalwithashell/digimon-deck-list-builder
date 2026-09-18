# NOTES — ToyAgumon (EX7-008)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-008`):
**3 clauses** — `effect#0`, `effect#1`, `inherited#0`. DCGO script:
`EX7/Red/EX7_008.cs` exists — the card is **not** `unavailable`. All three
clauses have a scenario; none is `unreachable`. No verdict is stored yet
(`qa/qa-reports/exam-verdicts/EX7-008.json` does not exist): all three are
`unmeasured` until the oracle pass.

| Clause | Scenario | Book | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-008#effect#0` — `[Digivolve] Lv.2 w/[Three Musketeers] in text: Cost 0` | `EX7-008-effect0.yaml` | `tm_ex7_008_pool.json` (`tm-kapurimon-egg`) | lowers, asserts pass |
| `EX7-008#effect#1` — `[On Play]` reveal 3, add 1 [Three Musketeers]-text card and 1 cost-6 Option, rest to the bottom | `EX7-008-effect1.yaml` | `three_musketeers_pool.json` | lowers, asserts pass |
| `EX7-008#inherited#0` — `[Your Turn] This Digimon gets +2000 DP.` | `EX7-008-inherited0.yaml` | `tm_ex7_008_pool.json` | lowers, asserts pass |

## Why a per-card book

Same reason as `NOTES-EX7-051.md`: the alt circle costs what the printed red
circle costs (0), so it is measurable only as legality, on a base the printed
circle cannot reach. The only Lv.2 cards printing "[Three Musketeers]" are
black (Kapurimon EX7-005, Pagumon BT25-005); `tm-kapurimon-egg` is the shared
50 with Kapurimon ×4 in the egg box. `EX7_008.cs` passes no `cardColor` to
`AddSelfDigivolutionRequirementStaticEffect`, so DCGO applies no colour check
on that circle; our `alt_paths` entry carries no `color_is` either. Kapurimon
is implemented on both sides (`61fc7c7d9`) and its trigger does not fire on a
digivolution in either engine.

## Audit of the files the crashed stage left (2026-09-18)

All three were complete and lowered; the only edits were stale header text
("Kapurimon has no YAML spec in our engine yet" — it has had one since
`61fc7c7d9`). Prompt sequences re-derived from the C#:

- `effect#0`: one open circle at cost 0, ToyAgumon has no `[When Digivolving]`
  → no prompt after the digivolve on either side.
- `effect#1`: `SimplifiedRevealDeckTopCardsAndSelect` with two conditions and
  no `canNoAction` → no "Will you select cards?" `generic_bool`; one
  `SelectCardEffect` per condition whose candidate count over the REMAINING
  revealed cards is ≥ 1 (`RevealLibrary.cs`: `AfterSelectCardCoroutine`
  removes the pick from `revealedCards` before the next condition is
  evaluated), so picking Sparrowmon for bucket 1 is what leaves Hurricane
  Screw Shot for bucket 2. The single remaining card goes to the bottom with
  no prompt (`ReturnRevealedCardsToLibraryBottom`, the `Count == 1` arm). Our
  engine parks a one-choice `OrderedPermutation` there — the documented
  "OrderedPermutation at N=1 — ours" row of `docs/DCGO_EXAM.md`; answered
  `sim_only`.
- `inherited#0`: `ChangeSelfDPStaticEffect(2000, isInheritedEffect: true)` is
  gated on `IsOwnerTurn`; the card face and the official bundle print
  `[Your Turn]` (the clause extractor keeps the timing tag out of `text`). The
  line reads both sides of the gate off the effective DP in the projection:
  6000 on the P0 turn, 4000 on the P1 turn. The Scopemon BT21-071 digivolve
  that puts ToyAgumon underneath carries the vacuous `[When Digivolving]`
  OptionalSkill of Scopemon as a `dcgo_only` decline (the
  `BT21-071-effect0.yaml` shape).
