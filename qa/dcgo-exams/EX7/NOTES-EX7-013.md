# NOTES — MagnaKidmon (EX7-013)

Denominator (`PYTHONPATH=code python -m tools.clause_coverage.extract --card-ids EX7-013`):
**3 clauses** — `effect#0`, `effect#1`, `effect#2`. DCGO script
`EX7/Red/EX7_013.cs` exists — the card is **not** `unavailable`. No verdict is
stored yet (`qa/qa-reports/exam-verdicts/EX7-013.json` does not exist): all three
are `unmeasured` until the oracle pass. None is `unreachable`.

| Clause | Text | Scenario | Sim-only (2026-09-18) |
|---|---|---|---|
| `EX7-013#effect#0` | `[Digivolve] Lv.5 w/[Three Musketeers] in text: Cost 4` | `EX7-013-effect0.yaml` (deck `tm-magnakidmon`) | lowers, 6/6 asserts pass (crashed agent's file, audited, unchanged) |
| `EX7-013#effect#1` | `[On Play] [When Digivolving] You may use 1 [Three Musketeers] trait Option card from your hand without paying the cost. Then, draw … until you have 6 in your hand.` | `EX7-013-effect1.yaml` ([When Digivolving] arm, deck `tm-magnakidmon-red`) | lowers, 12/12 |
| `EX7-013#effect#2` | `[End of Your Turn] [Once Per Turn] By trashing 1 Option card from this Digimon's digivolution cards, 1 of your [Three Musketeers] trait Digimon gains <Security A. +1> for the turn, and attacks.` | `EX7-013-effect2.yaml` (deck `tm-magnakidmon-red`) | lowers, 12/12 |

Book: `qa/dcgo-exams/EX7/tm_ex7_013_pool.json`. `tm-magnakidmon` is the crashed
agent's deck (the shared 50 with EX7-013 ×4 for Gazimon ×4); `tm-magnakidmon-red`
is the same list with EX7-070 ×2 swapped for the vanilla red Lv.5 **Vermilimon
BT4-014** ×2. The extractor's `effect#1` text swallows the next clause's
`[End of Your Turn]` marker; the clause under `effect#2` is the End-of-Turn one.

## `effect#0` — audited, kept

Base = LadyDevimon BT25-083 (purple Lv.5 that prints the trait name), so only the
clause opens the route; her `[On Play]` is inert because MagnaKidmon — a
`[Three Musketeers]`-TRAIT card — is drawn only on T5. No digivolution into
BT25-085 and no `<De-Digivolve>` on the line, so neither campaign-wide repair
applies. Re-lowered against the current engine: unchanged.

## `effect#1` / `effect#2` — one line, parted at the End-of-Turn gate

Vermilimon (T3) → MagnaKidmon (T5, 3 → -1) → `[When Digivolving]` uses Bind Red
Trigger P-180 for free (P1 security 5 → 4, P-180 tucked under MagnaKidmon, hand
drawn back to exactly 6) → memory -1 ends the turn → `[End of Your Turn]` is live
because P-180 was just tucked. `effect1` declines the gate and asserts the clause
on a quiet board; `effect2` accepts it: trash P-180, MagnaKidmon gains
`<Security A. +1>` and attacks the player, checking **two** security cards
(4 → 2). P1's board is empty on purpose: P-180's own "when effects trash this
card" trigger then has no target on either engine, so the known trigger-timing
finding (`../P/NOTES-P-180.md`) cannot reorder MagnaKidmon's later prompts.

Rows that are split rather than shared, and why:

- **The Option to trash** — ours is a one-candidate `UnionZone{material}` pick
  that `cards:` cannot address (no digivolution-card identity in the resolver):
  `yes:` sim-only + `cards: [P-180]` dcgo-only (the `P-180-effect0.yaml` idiom).
- **The attack target** — DCGO's `SelectAttackEffect` harness hook takes
  "attack the player" only as `select_value: -1` (a bool answer aborts the job),
  and our `Target` prompt cannot map a bare `-1` ("count payload -1 has no
  candidate set"): `yes:` sim-only + `value: -1` dcgo-only.

## Not separable on this line (recorded, not claimed)

DCGO's attacker filter is **any** own battle-area Digimon
(`IsPermanentExistsOnOwnerBattleAreaDigimon`); the printed text and our YAML say
"1 of your **[Three Musketeers] trait** Digimon". Telling them apart needs a second,
non-trait own Digimon, i.e. a two-Digimon board — which is not slot-safe under the
harness (`../BT25/NOTES-BT25-083.md`) — and a `candidates:` expectation. Printed
text outranks DCGO for what the card says, so ours is the faithful side; it would
read as a DCGO quirk, not a finding against our engine.

## ENGINE FINDING met while authoring (logged, NOT fixed)

`F-ENGINE-FREE-OPTION-USE-IS-AFFORDABILITY-GATED` (`docs/RUST_ENGINE_GAPS.md`).
The first draft used the **[On Play]** arm: hard-play MagnaKidmon at memory 3
(12: 3 → -9), pick P-180. Measured sim-side: P-180 stayed in hand, P1's security
stayed 5, **and the mandatory "draw until 6" did not run** (declining the pick on
the same board draws to 6 correctly). Cause, read from the source:
`Game::option_legal_play_modes` (`game_actions/mod.rs`) filters every mode by
`memory - use_cost >= memory_min` regardless of `OptionCostPolicy::Free`, so at -9
a 6-cost Option has no legal mode → `play_option_core` returns `Invalid` →
`run_use_option_from_hand_step` returns early, dropping the tail and the outer
continuations. DCGO's `PlayOptionCards(payCost: false)` has no memory test, and a
"without paying the cost" use pays nothing (nothing to be unable to afford). The
committed line sidesteps it through the `[When Digivolving]` arm (-1 − 6 = -7 is
inside the window); the [On Play] arm at deep-negative memory is what a fix must
re-test.
