## 1. Shared evaluation

- [x] 1.1 Add optional anchored collector, scoped aggregation and durable failure reports.
- [x] 1.2 Add checkpoint/panel provenance and compatible dashboard filters.

## 2. Scheduling and publication

- [x] 2.1 Implement persistent bounded periodic checkpoint evaluator and service.
- [x] 2.2 Publish card reports through the monitor with retry and safe W&B steps.

## 3. Validation and rollout

- [x] 3.1 Test collector wiring, schedule/restart, failures and publication; run real-engine smoke.
- [x] 3.2 Document cadence and limits, deploy isolated evaluator, verify W&B report without restarting training.

Validation: 33 focused tests passed, one historical champion test skipped because its external fixture is absent. Four real-engine smoke games passed on the CI image (event version 1), then 96 baseline evaluation games completed (48 per panel). W&B confirmed both card-evaluation artifacts and `per_card_metrics_available=true`. Training retained its original container start time. Local native smoke was correctly rejected by the older local wheel; live engine validation used the matching CI image.
