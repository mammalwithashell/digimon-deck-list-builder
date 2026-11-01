# TS Plutomon: exact-list admission evidence

Verified on 2026-10-31 in local branch `codex/ts-plutomon-admission-20261031`,
based on freshly fetched main `751ebca2bd93909765c5e95718074a15d331baf9`.

## Result

[NewGuy's October 30 list](https://digimon.digilab.cards/decklist/131677)
passes the canonical training-admission gate and the real `MetaGauntlet.load`
path with a Python binding built from this checkout. The fixture preserves all
50 main-deck cards, four Digi-Eggs, and 19 unique IDs. See
`ts-plutomon-newguy-131677.json` and `ts-plutomon-admission.json` beside this report.

Fresh baseline generation reproduced four missing mechanics proofs:

| Card | Missing evidence on main |
|---|---|
| BT24-007 | `event_discarded_card_any`, `in_event_discarded_cards` |
| BT24-023 | `played_by_effect` |
| BT26-079 | `main_from_trash` |

Nine independent inline-YAML tests now execute these lowerings through the
engine. They assert matching and nonmatching discard batches, physical-copy
membership (including exclusion of an identical older trash card), both legal
choices between fresh copies, mixed-batch membership, effect versus paid hand
play, and trash Main activation/condition/turn/adjacent-timing boundaries.
Choices are checked against the real action mask and explicitly resolved.
Production engine code, card YAML, scanner rules, and gate policy are unchanged.

The reusable `played_by_effect` proof also admits BT24-074. Its existing
production-card suite was included in validation. Overall admissible-card count
changes from 1,091 to 1,095; the selected list changes from 16/19 to 19/19 IDs.

## Validation

| Check | Result |
|---|---|
| New mechanics tests | 9 passed (also included in full DSL result) |
| Full DSL | 1,644 passed, 1 ignored |
| BT24-007 / BT24-023 / BT26-079 / BT24-074 | 10 / 17 / 7 / 17 passed |
| Judge quiz | 44 passed, 1 ignored |
| Rules conformance | 361 passed, 1 ignored |
| Rules FAQ | 31 passed |
| Keyword semantics matrix | 3 passed |
| Determinism guard | 2 passed, 1 subprocess helper ignored |
| Tier 0 | All 13 stages passed; tooling suite 183 passed |
| Lowering/admission/mechanics Python tests | 104 passed, including the binding-dependent case |
| All three generated gate-artifact freshness checks | Passed |
| Matching Python wheel build and installation | Passed |
| Real gauntlet load | Exact source `digilab_131677` admitted; counts unchanged |
| Seed 131677 runtime smoke | 94 legal actions, game ended |
| Seed 131678 runtime smoke | 100 legal actions, bounded smoke limit reached |

Both runtime smokes checked the `standard_lite_v2` tensor size (9,434) and action
mask size (2,192). The capped smoke is not treated as a draw or a completed game.
The runner records the exact seed/action trace and propagates native failures.

Commands were run from the worktree, using its `.venv` and dedicated
`D:\cargo-target\plutomon` cache. Cargo compilation used at most two workers
(one for the binding); Rust tests used two threads and a 256 MiB stack.

```powershell
$env:PYTHONPATH = 'code'
.venv\Scripts\python.exe -m tools.lowering_coverage
.venv\Scripts\python.exe -m digimon_gym.agents.card_admissibility
.venv\Scripts\python.exe -m tools.rules.mechanics_readiness
# Each generator was also rerun with --check.
.venv\Scripts\python.exe scripts\verify --tier 0 --continue-on-failure
.venv\Scripts\python.exe -m pytest code/tests/tools/test_lowering_coverage.py code/tests/tools/test_rules_mechanics_readiness.py code/tests/rl/test_card_admissibility.py -q --override-ini addopts='' --basetemp ..\evidence\pytest-final --tb=short
# After installing the binding, the one previously skipped test was rerun:
.venv\Scripts\python.exe -m pytest code/tests/rl/test_card_admissibility.py::test_gauntlet_ledger_gate_matches_the_owned_constant -q --override-ini addopts='' --basetemp ..\evidence\pytest-binding --tb=short
cargo test -p digimon-engine --test dsl -- --test-threads=2
# Separate invocations for bt24_007, bt24_023, bt26_079, bt24_074:
cargo test -p digimon-engine --test cards_behavioral bt24_007 -- --test-threads=2
cargo test -p digimon-engine --test rules_conformance -- --test-threads=2
cargo test -p digimon-engine --test judge_quiz --test rules_faq --test keyword_semantics_matrix -- --test-threads=2
cargo test -p digimon-engine --test infra determinism_guard -- --test-threads=2
.venv\Scripts\maturin.exe build --manifest-path code\digimon-engine-py\Cargo.toml --interpreter .venv\Scripts\python.exe --out ..\evidence\wheels
uv pip install --python .venv\Scripts\python.exe ..\evidence\wheels\digimon_engine-0.1.0-cp311-abi3-win_amd64.whl --offline
.venv\Scripts\python.exe -m tools.plutomon_admission_report --runtime
```

Raw logs, baseline artifacts, the wheel, and test scratch evidence are preserved
in `C:\Users\james\Documents\Codex\2026-10-31\task\evidence`.
Python temporary directories were directed into this task's evidence directory.
Initial fixture-authoring failures (numeric binding syntax and using a
selection-only helper for ordinary plays) were corrected before the passing run;
no assertions or production behavior were weakened. An initial Python run had
temporary-directory setup errors and a deliberately stale generated-artifact
comparison; reruns with local temporary paths and regenerated artifacts passed.

## Changed files

- `code/digimon-engine/tests/dsl/plutomon_admission_constructs.rs`
- `code/digimon-engine/tests/dsl/main.rs` (one module registration)
- `code/tools/plutomon_admission_report.py`
- `data/card_admissibility.json`
- `qa/impact/lowering_coverage.json`
- `qa/archetype-qa/ts-plutomon-newguy-131677.json`
- `qa/archetype-qa/ts-plutomon-admission.json`
- `qa/archetype-qa/ts-plutomon-admission.md`

## Scope and limits

Read-only preflight and a later recheck of `D:\wt\p3-constructs` found no matching
lowering proofs beyond the existing parse-only test. Desktop session discovery
did not respond and that read request was terminated; active ownership was not
explicitly confirmed through the desktop API. The existing worktree and its
shared-engine changes were not modified.

No final validation failure or access blocker remains. Full-card-suite, search
sampler, replay-golden, and DCGO verification were not run. Previously reported
unrelated main failures are not claimed repaired or cleared by these results.
Admission is not DCGO certification. Validation did not run training, paid
verification, merging, or deployment.
