## Why

CP8, CP12, CP16 and CP20 have been compared using known opponent deck compositions. Main now includes fair-belief search with EX13 software qualification, allowing us to test whether checkpoint rankings survive realistic uncertainty without retraining the models. Existing results use an older engine, so information-regime effects require matched controls on one newly frozen runtime.

## What Changes

- Extend the frozen two-model evaluation harness with explicit per-actor open/closed information regimes and independent per-seat knowledge ledgers, reusing the production belief API.
- Freeze a training-list-only prior and keep source-held-out, supplemental and pet populations separate.
- Plan six closed-search checkpoint pairings, six matching open-search controls, and four within-checkpoint open-versus-closed panels at equal search budgets.
- Add a separate source-held-out closed-search round robin, with Light Hybrid coverage limitations stated explicitly.
- Require information-leakage, continuation, model compatibility, Windows/Linux parity, receipt and resource checks before scored execution.
- Report paired uncertainty, belief diagnostics, timing and distinct evolution routes; preserve all failed attempts and retain the existing campaign unchanged.
- Add a separately reported [mulligan diagnostic](mulligan-evaluation.md) for CP8/12/16/20: compare policy-only decisions with 4-, 8- and 16-world search at a fixed 320-simulation budget on matched opening states, then investigate selected keep/redraw disagreements through controlled continuations.

## Capabilities

### New Capabilities
- `frozen-checkpoint-information-regime-evaluation`: Auditable frozen-model comparisons across search information regimes and deck-prior support.

### Modified Capabilities
None. This experiment consumes the existing fair-belief search contracts without changing training, observation or action semantics.

## Impact

Planned implementation touches `code/digimon-engine/examples/checkpoint_search_eval.rs`, evaluation orchestration and tests under `code/tools/cloud_search/` and `code/tests/tools/`, and immutable experiment reports. Builds use isolated targets and private bindings. Local and Hetzner execution must respect other active jobs. This proposal does not launch jobs, replace frozen artifacts, train weights, restart controllers or promote a model.
