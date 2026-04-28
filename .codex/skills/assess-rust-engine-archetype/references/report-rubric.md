# Rust Engine Archetype Assessment Rubric

## Evidence Checklist

Collect only the evidence needed for the requested archetype:

- Card list: card IDs, names, role in archetype, and counts if decklist-based.
- Printed text: `effect_text`, `inherited_text`, `security_text`, plus alt digivolution or special play requirements.
- Existing implementation: YAML in `code/digimon-engine/cards/_examples/`, generated embedded DSL pack, raw Rust cards, or implemented card registry.
- DSL support: schema/spec type, predicate, step, timing, formula, binding, and lowering path.
- Engine support: `EffectContext` primitive, timing dispatch, pending-selection/action mask support, state field, or combat/security hook.
- Tests: closest existing DSL or behavioral regression test.
- Gap tracker routing: `docs/RUST_ENGINE_GAPS.md` for missing Rust engine primitives, `qa/dsl-vocab-gaps.md` for missing DSL vocabulary or lowering, and `docs/RUST_PYTHON_PARITY.md` for cross-engine divergences.

## Capability Table

Use this shape when reporting many cards:

| Card | Required behavior | Status | Evidence | Gap / next step |
|---|---|---|---|---|
| `CARD-ID` Name | Brief text-normalized behavior | ready / dsl-gap / engine-gap / rules-gap / test-gap / data-gap | File refs or local docs | Smallest missing capability |

## Gap Format

For each non-ready gap, include:

- **Gap:** one sentence naming the missing capability.
- **Type:** `dsl-gap`, `engine-gap`, `rules-gap`, `test-gap`, or `data-gap`.
- **Tracker:** `qa/dsl-vocab-gaps.md`, `docs/RUST_ENGINE_GAPS.md`, `docs/RUST_PYTHON_PARITY.md`, or `none` if the issue is local to the requested assessment.
- **Blocks:** card IDs/effects affected.
- **Why it matters:** the gameplay choice, timing, or mutation that cannot be faithfully represented.
- **Evidence:** local file references, tests, docs, or absence after targeted search.
- **First test:** the smallest failing Rust test that would prove the required behavior.
- **Implementation hint:** probable file area, such as `code/digimon-dsl/src/step.rs`, `code/digimon-engine/src/dsl_cards/step/`, `code/digimon-engine/src/effect_context/`, `code/digimon-engine/src/action/`, or `code/digimon-engine/tests/dsl/`.

## Readiness Verdicts

- `ready`: all core cards are expressible through current DSL and backed by engine behavior.
- `mostly ready`: only tech cards, coverage gaps, or low-risk DSL conveniences remain.
- `blocked`: at least one core card needs new engine/action/pending-selection behavior.
- `unknown`: local source data or rules evidence is insufficient.

## Assessment Pitfalls

- Do not infer DSL support from Rust engine support alone; verify schema and lowering.
- Do not infer engine support from parser support alone; verify lowered runtime behavior.
- Do not accept singleton auto-picks for choices. Even one-card choices often need `PendingSelection` under this repo's no-approximations policy.
- Do not treat DCGO as authority over printed text or local rules docs.
- Do not report "implemented" solely because a card ID appears in data; distinguish metadata from executable behavior.
- Do not file DSL vocabulary/lowering gaps in the Rust engine tracker, or engine primitive gaps in the DSL vocabulary tracker.
