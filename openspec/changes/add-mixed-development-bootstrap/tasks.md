## 1. Contracts and fixtures

- [ ] 1.1 Inventory native effective-cost/action-benefit metadata and event coverage for raising, battle, activated/effect and DNA evolution, draw, search, retrieval, filtering and payment; record unknown coverage explicitly.
- [ ] 1.2 Add failing tactical fixtures for useful evolution versus hard play, modest versus excessive memory handoff, immediate public combat priorities, normal evolution replacement, inactive draw text and DNA material tradeoffs.
- [ ] 1.3 Add failing pending-choice fixtures for optional draw/retrieval, evolution-line-preserving discard, page rebinding with duplicate IDs, invalidation, no-progress cycles and clean per-seat game reset.
- [ ] 1.4 Add hidden-zone permutation tests and cost-quote versus actual-payment parity checks without modifying tensor/action contracts.

## 2. Development teacher

- [ ] 2.1 Implement versioned read-only candidate features using engine legality and available native cost metadata; report unsupported benefit/cost cases rather than inventing results.
- [ ] 2.2 Implement `development_v1` with recorded feature scores, evolution/resource preferences, tactical priorities, memory/body penalties and deterministic tie breaks; retain greedy defaults.
- [ ] 2.3 Implement per-seat pending-choice strategy state, bounded permitted page inspection, physical candidate rebinding, progress detection and reset semantics.
- [ ] 2.4 Expose explicit teacher selection through PyO3 and the Rust Policy interface; verify actual decision-player routing and native/binding action parity.
- [ ] 2.5 Run tactical, information-boundary, legality, clone/state and existing greedy regressions; calibrate initial score scales against the pinned fixture expectations and freeze versioned weights.

## 3. Mixed bootstrap collection and fitting

- [ ] 3.1 Add reproducible greedy-only, development-only and 50/50 schedules with per-player assignments, global game IDs, independent assignment seeds, opening-seat balance and archetype/list coverage.
- [ ] 3.2 Implement the 416-game thirteen-archetype qualification schedule and verify exact 64-per-archetype/32-per-teacher appearances and stable worker-count-independent assignments.
- [ ] 3.3 Record teacher/seat/game provenance, action categories, route opportunities/choices, resource accounting, unknown metadata and failure recordings; add only telemetry absent from existing events.
- [ ] 3.4 Add stratified teacher/archetype/game row sampling with explicit realized weights and equal update budgets across controls; keep demonstrations out of search replay.
- [ ] 3.5 Enforce whole-game exclusions for both losses, stop qualification before fitting on caps/crashes, and preserve terminal-outcome targets plus game-level critic validation splits.
- [ ] 3.6 Wire opt-in mixture configuration into a separate fresh-seed preparation path with contract and artifact hash checks; prove default greedy behavior and current run inputs remain unchanged.

## 4. Qualification and comparison

- [ ] 4.1 Generate separate greedy-only, development-only and mixed qualification corpora on the same frozen schedule; report completion, opportunity-normalized behavior, coverage and collection throughput.
- [ ] 4.2 Fit all three fresh seeds with matched initialization and update budgets; verify checkpoint contracts and ONNX parity before evaluation.
- [ ] 4.3 Compare seed policies on the fixed 5,200-game cohort against common greedy/frozen-greedy-bootstrap anchors; retain paired seat-level results and separate meta-held-out/pet-same-list panels.
- [ ] 4.4 Run one matched 1,100-game 320/4/batch-four search update per seed in separate output directories, then repeat the common-anchor evaluation and tactical suite.
- [ ] 4.5 Report paired confidence intervals, the predeclared five-point aggregate noninferiority gate, per-archetype uncertainty, DNA-capable denominators, net hand resources, memory, attacks and caps; make a documented recommendation without automatic promotion.
- [ ] 4.6 Update the training runbook with teacher configuration/provenance, failure recovery and future image adoption instructions; preserve the currently running campaign as an independent baseline.
