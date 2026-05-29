# Fan-quiz conformance corpus — implementability audit

The seed corpus is derived from a community Digimon TCG rules quiz (30
scenarios; mechanics paraphrased, not reproduced). This audit maps each
scenario to a **mechanic category** and a **readiness** verdict so the
corpus can be built out incrementally — `expected_pass` fixtures encoded
now, `blocked_on_card_impl` documented and added as the cards land.

## Readiness reality

Almost every quiz scenario asserts the *outcome of a card-effect
resolution* (timing windows, immunity, on-deletion ordering, DNA/DigiXros
interactions). Those depend on the specific cards being implemented in the
engine, so they are `blocked_on_card_impl` until then. What the substrate
verifies **today**, independent of card implementation, is **staging-level
correctness**: that an arbitrary board can be reproduced exactly (stacks,
zones, suspend state, memory, phase) and that the engine reports the right
scalar/stack/DP/mask state on it. The Q16 *board* (the user's reported
DNA-digivolve bug) is encoded as an `expected_pass` staging fixture; the
Q16 *rules outcome* is a parallel `blocked_on_card_impl` fixture.

## Category → scenario map

| Category | Quiz scenarios (approx) | Readiness | Notes |
|---|---|---|---|
| Staging proof (board reproduction) | Q16 board | **expected_pass** | `q16-paildramon-staging.json` — stack/DP/memory verified in both layers |
| Timing / interruption ordering | Q6, Q7, Q9, Q13, Q14, Q21, Q30 | blocked | needs would-play / [On Play] / [On Deletion] / Partition resolution |
| Immunity / "unaffected" | Q1, Q2, Q15, Q17, Q18, Q28 | blocked | needs Progress / X-Antibody / persistent-immunity effects |
| [On Deletion] / [when trashed] | Q19, Q20, Q21, Q23 | blocked | needs Eyesmon: Scatter Mode + inherited trash triggers |
| DNA digivolve | Q9, Q26, Q30 | blocked | needs Mastemon / Miraculous Mega Knight / inherited-DNA effects |
| DigiXros | Q25, Q26, Q29 | blocked | needs DarknessBagramon / Dorbickmon DigiXros + stack ordering |
| Cost / payability declaration | Q5, Q26, Q27 | blocked | needs [Assembly] / cost-becomes-unpayable handling |
| Tokens | Q12, Q13, Q22 | blocked | needs Petrification/Familiar token placement + egg-deck-bottom rule |
| Security-check counting | Q4 | blocked | needs Security Attack +1/-1 net resolution |
| Breeding-area rules | Q3 | blocked | needs breeding-area effect gating |
| De-Digivolve / Burst | Q8, Q15 | blocked | needs Burst end-of-turn trash + one-at-a-time De-Digivolve |
| Memory cascade (multi-step) | Q10, Q11 | blocked | needs Mental Training / Gravity Crush / Akihiro Kurata / MirageGaogamon |
| Alliance + suspend-DP timing | Q24 | blocked | needs Hudiemon Alliance + Rapidmon(X) suspended-DP aura |

## How the corpus runs

- **Rust layer** — `cargo test -p digimon-engine --test scenario_corpus`
  loads `data/cards.json`, stages each fixture, evaluates `engine`
  assertions. `expected_pass` must pass; `blocked_on_card_impl` prints
  PENDING.
- **UI/server layer** — `npx playwright test scenario-conformance` (with
  FastAPI up) stages each fixture via `/debug/games` and evaluates the same
  `engine` assertions server-side. Same fixtures, same verdicts.

## Adding a scenario

1. Author `qa/scenarios/<id>.json` per `README.md`.
2. Tag `readiness`. If it needs unimplemented cards, set
   `blocked_on_card_impl` with a `blocked_reason`.
3. Add the `id` to the `CORPUS` list in
   `code/frontend/e2e/scenario-conformance.spec.ts` (the Rust runner
   auto-discovers all `*.json`).
4. When a blocked fixture starts passing, flip its tag — that surfaces the
   newly-implemented behavior.
