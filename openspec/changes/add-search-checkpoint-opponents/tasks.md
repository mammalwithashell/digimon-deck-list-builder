## 1. Native historical opponents

- [x] 1.1 Add validated pool identities and deterministic paired assignments.
- [x] 1.2 Route real actors to separate evaluators, preserve search and filter historical training rows.
- [x] 1.3 Add native CLI loading, validation and per-game receipts; test default equivalence and chunk determinism.

## 2. Orchestration

- [x] 2.1 Freeze explicit pins and recent completed generations, rejecting empty or mutated pools.
- [x] 2.2 Bind all pool assets into dynamic scheduler identity and aggregate learner coverage.
- [x] 2.3 Forward optional cloud-plan configuration with hash validation and test generation/controller contracts.

## 3. Verification and publication

- [x] 3.1 Run native and Python regressions and a bounded local ONNX integration smoke. Native: 24 passed. Python: 100 passed, one skip. Real ONNX: one full game completed; second hit the 600-second dev deadline. Separate six-decision probe completed both opening orders with expected truncation; no probe data fitted.
- [x] 3.2 Update the runbook with configuration, interpretation, limitations and validation evidence.
- [x] 3.3 Validate OpenSpec, synchronize with main, commit, push and update PR #754; leave cloud training stopped.
