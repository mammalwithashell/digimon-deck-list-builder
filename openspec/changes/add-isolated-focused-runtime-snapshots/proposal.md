## Why

Focused deck validation currently encounters unrelated global source-migration disagreements before it can exercise its own repaired cards. An explicit, independently validated runtime snapshot can make progress on the exact two-deck campaign without treating a blocked global candidate as publishable or silently narrowing its evidence denominator.

## What Changes

- Audit the complete executable and evidence closure of selected decks, opponents, scenario setup and named rest decks, dynamic references, and shared Q&A. Preserve every source-universe member and unresolved reference.
- Add a separate immutable snapshot producer that regenerates admitted records from complete original source inputs and reviewed corrections/selections, with exact consumer-field provenance, stable global indices, and a pinned DSL pack.
- Add explicit isolated consumer entry points that verify snapshot identity and reject missing or outside-closure cards before execution. Existing default loaders, publication paths, and pool selection retain their contracts.
- Bind source-book v2 clauses, complete gating interactions, scenarios and all five verdict classes to an append-only campaign generation. Historical evidence stays intact; changed bindings require new measurements.
- Stage adoption: offline audit and producer first, isolated harness next, then an explicitly bounded Pilot/Architect route with matching run/model metadata only after behavioral validation. No action or observation layout changes are proposed.

## Capabilities

### New Capabilities

- `isolated-focused-runtime-snapshots`: Complete-universe accounting, executable/evidence closure, source-derived sealed projections, strict isolated consumers, generation-bound validation and bounded training admission.

### Modified Capabilities

- `canonical-card-data-pipeline`: Distinct scoped artifact refusal at global boundaries and fresh producer/projection epoch requirements, preserving stronger committed-path and forward-projection contracts.
- `card-data-validation-gates`: Complete isolated binding tuple, loaded-code freshness, immutable read/activation checks and pack/release separation without weakening global gates.
- `legacy-free-training-runtime`: Explicit scoped Rust handles and final-deck/worker/cache refusal, preserving Rust-only execution and compact-profile retirement.

Implementation and validation occur only on the existing isolated repair branch, seeded at `1201f706bcc48f27aa813c2dfe301ca1d567beb8`, followed by reviewed integration into the current two focused branches. The base checkout is a read-only human-merge compatibility reference, not an implementation destination. Exact working-tree spec bytes and branch/function pins are in `integration-contract.json`; they are not assumed to be contained in base HEAD. The contract remains proposed pending independent review.

## Impact

Likely implementation surfaces are `code/tools/card_data/`, `code/tools/clause_coverage/`, `code/tools/card_loop/`, the Rust harness and runtime constructors, and later explicitly opted-in PyO3/training consumers. Source/correction history, complete inventories, stable registry ownership, engine packs, and current scenario/Q&A inputs become reproducibility dependencies. Documentation changes must cover CARD_DATA_PIPELINE, CARD_POOL_CONTRACTS, TRAINING_RUNBOOK and affected runtime APIs when the corresponding stage is implemented. Engine fixes remain on their separate review branches; no main merge, publication, or training launch is authorized by this proposal's existence.
