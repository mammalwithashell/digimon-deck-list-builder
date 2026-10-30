# Paradise / Callismon bounded repair — 2026-10-30

## Scope and provenance

- Worktree: `C:\Users\james\.codex\worktrees\0a17\digimon-deck-list-builder-1`.
- Branch: `codex/paradise-callismon-admission`.
- Clean starting revision and freshly fetched `origin/main`: `77f4702f9c786f95aabb4745593e358c6face855`.
- One repaired card: BT25-058 Callismon. No shared-engine changes, training, publication, merge, deployment, or shared-player operations.
- Exact dated lists: [DigiLab 120649, October 11](https://digimon.digilab.cards/decklist/120649) and [DigiLab 130284, October 26](https://digimon.digilab.cards/decklist/130284). Both public pages were fetched and their full ID/count maps compared with the requested list on October 30. Each has 50 main cards, 4 eggs, and 21 unique IDs; composition is identical. See `exact-lists.json` for both entries and captured-page hashes. Page captures remain in `target-callismon/source-pages/`.
- No pairing API was queried and no historical match result was reverified.

## Repair and regressions

The optional suspension selection omitted the existing `continue_on_decline` contract. Its default drops the remaining process, which incorrectly skipped the mandatory independent “Then” lock. The production change adds `continue_on_decline: true` and an explanatory comment to that selection only.

The legal, mask-checked On Play reproducer failed before the change at “mandatory Then lock after decline.” The baseline broader filter ran 14 passing and 6 failing tests: four exposed decline-tail behavior and two exposed fixture setup mistakes, subsequently corrected without weakening the assertions.

Final focused result: **20 passed, 0 failed, 0 ignored** (18 card tests and 2 printed-form checks). Seven added regressions cover decline and mandatory masks; distinct selected suspension/lock targets; an already-suspended target; independent clone continuation and opponent-turn expiry; legal digivolve; legal attack; and Fortitude replay with explicit trigger-order choice. Existing coverage includes the empty opponent board. Independent source and test review approved the final diff with no findings.

Commands used isolated `CARGO_TARGET_DIR=<worktree>/target-callismon`, `CARGO_BUILD_JOBS=2`, `RUST_MIN_STACK=268435456`, and serial tests:

```powershell
cargo test -p digimon-engine --test cards_behavioral bt25_058_declined_suspend_still_requires_lock -- --test-threads=1 --nocapture
cargo test -p digimon-engine --test cards_behavioral bt25_058 -- --test-threads=1 --nocapture
cargo build -p dcgo-harness -p digimon-engine-py --lib --bin dcgo-harness
```

Logs: `red-decline.log`, `red-all-callismon.log`, and `green-callismon.log` in this directory. Full build logs also remain in `target-callismon/`.

## Saved DCGO comparisons

No live DCGO run was performed. The branch-built harness replayed the existing scenarios against the original October 10 recordings:

| Interaction | Result |
| --- | --- |
| `probe:BT25-058#effect#4:optional_decline` | CLEAN, 14/14 ours and 14/14 DCGO steps |
| `probe:BT25-058#effect#3:leave_play` | CLEAN, 33/33 ours and 33/33 DCGO steps |

The same comparisons were recorded using the harness's `--verdicts`, with freshly generated card-scoped `clause-text.json` and `interaction-denominator.json`. Only those two interaction verdicts were replaced with confirmed results. The harness migrated this card file to schema 3 and preserved the older rows as `legacy_unbound`. Original verdicts, including both `ours_wrong` findings, remain in `baseline-verdicts.json`; recording/sidecar hashes are in `saved-recording-hashes.json`. The separate historical Q6345 stale-triaged row was not adjudicated here.

Reproduce each comparison with `target-callismon/debug/dcgo-harness.exe exam --scenario <original scenario> --sidecar <hashed .state.jsonl> --cards-json data/cards.json --decks qa/dcgo-exams/BT25/tm_bt25_058_pool.json`. Recording additionally uses `--verdicts --clause-text-json qa/qa-reports/paradise-callismon-2026-10-30/clause-text.json --interaction-denominator qa/qa-reports/paradise-callismon-2026-10-30/interaction-denominator.json`. See `rediff-*.log` and `record-*.log`.

Scoped source-book commands (`PYTHONPATH=code`):

```powershell
python -m tools.clause_coverage.extract --card-ids BT25-058 --out qa/qa-reports/paradise-callismon-2026-10-30/clause-text.json --quiet
python -m tools.card_loop.interactions build --cards-from qa/qa-reports/paradise-callismon-2026-10-30/clause-text.json --out qa/qa-reports/paradise-callismon-2026-10-30/interaction-denominator.json
```

These are two measured interactions, not full card/archetype oracle certification. The fresh scoped denominator contains 6 clauses and 13 interactions (11 gating); the harness reports 0/6 currently bound confirmed clauses (all six historical clause rows are legacy-unbound/unmeasured). Only the two re-diffed interactions have current semantic binding. The standard global oracle-readiness report remains separate from training admission.

## Exact-list admission and runtime

Baseline: 20/21 IDs admissible, 0/2 exact lists loadable. Both were rejected solely for Callismon `diverged_no_quirk`; see `baseline-admission.json` and `baseline-loader.json`.

After the measured repair, regeneration reports **21/21 IDs admissible; 2/2 lists admitted**. `final-admission.json` retains each card's complete gate row. Global mechanics is ready with zero flagged blocking obligations; 315 open nonblocking gaps remain visible, not certified neutral.

Regeneration commands (with `PYTHONPATH=code`):

```powershell
python code/tools/impact_index.py --cards-dir code/digimon-engine/cards --out qa/impact/dsl_impact_index.json
python -m tools.rules.mechanics_readiness
python -m tools.lowering_coverage
python -m digimon_gym.agents.card_admissibility
python -m tools.clause_coverage.readiness
```

The freshly built PyO3 DLL was copied into `target-callismon/python/digimon_engine.pyd` and imported using an isolated `PYTHONPATH`; the installed wheel was not replaced. Production `MetaGauntlet.load()` admitted both entries using the committed gates and the fresh registry. Each list then completed 20 legal-action `DigimonEnv` smoke steps, seeds 1 and 2, using `standard_lite_v2` (9434 observation values; 2192 action-mask entries). See `runtime-smoke.json` for binding path/hash and results. The local reproducible smoke command is `python target-callismon/runtime_smoke.py` with `PYTHONPATH=<worktree>/target-callismon/python;<worktree>/code`. This ran on a 268369920-byte Python worker-thread stack; it is not default-stack, full-game, policy-strength, or training qualification.

## Aggregate verification

The final serial ladder is invoked with:

```powershell
python scripts/verify --tier 2 --path code/digimon-engine/cards/bt25/BT25-058.yaml --path code/digimon-engine/tests/cards_behavioral/bt25/bt25_058.rs --continue-on-failure
```

Final ladder exit: **1; 24 suites passed, 1 failed**. No required suite was skipped. Log: `verify-tier2.log`; machine-readable summary: `aggregate-results.json`.

| Check | Passed | Failed | Ignored |
| --- | ---: | ---: | ---: |
| Verification tooling (Python) | 183 | 0 | 0 |
| DSL (both ladder invocations) | 1596 each | 0 | 1 each |
| Judge quiz | 44 | 0 | 1 |
| Rules conformance | 361 | 0 | 1 |
| Rules FAQ | 31 | 0 | 0 |
| Keyword semantics matrix | 3 | 0 | 0 |
| Determinism guard | 2 | 0 | 1 |
| Focused Callismon filter | 20 | 0 | 0 |
| Full card behavior | 12749 | 0 | 35 |
| Invariant fuzz | 5 | 0 | 0 |
| Replay goldens | 4 recordings | 6 recordings | 0 |

All tier-0 artifact/gate checks passed. `git diff --check` passed.

**Remaining all-green blocker: pre-existing replay-golden divergence.** Six recordings diverge at steps 26, 18, 35, 16, 44, and 16. None contains Callismon. To establish the baseline empirically, only the changed production YAML was temporarily replaced with HEAD's original bytes, then `cargo build -p digimon-engine --bin replay_corpus` and the same `replay_corpus verify` command were run. The original-main card source reproduced all six identical filename/step/recorded-digest/replayed-digest tuples. The fixed YAML was restored in `finally`, and its exact tested hash was verified. Fixed and baseline replay executables are preserved under `target-callismon/`; the fixed executable was restored to `debug/replay_corpus.exe` afterward. No goldens were blessed, regenerated or weakened.

See `baseline-replay-check.json`, `baseline-replays.log`, and `baseline-replay-build.log`. This remaining baseline failure is outside the one-card repair scope; the overall ladder is not claimed green.

## Files and evidence

Production/test changes are limited to Callismon's YAML and card test file. Evidence updates are its DSL ledger row, exam-verdict file, existing gap entry, generated `data/card_admissibility.json`, and this report directory. Other regenerated artifacts were semantically unchanged. Two generator-only CRLF rewrites were restored to their original bytes after explicit JSON-equality checks; receipts are in `unchanged-artifact-restoration.json`. `final-input-hashes.json` records the tested source and gate inputs. The PR includes scoped source changes, structured evidence, saved-comparison logs, and `test-output.txt` extracts. Large compiler/test logs (`red-decline.log`, `red-all-callismon.log`, `green-callismon.log`, `baseline-replay-build.log`, `verify-tier2.log`), helper scripts, generated binaries, and source-page captures remain local and are not included in the PR. `aggregate-results.json` preserves every suite outcome and all replay divergence tuples; `test-output.txt` preserves the RED/GREEN and final ladder summaries.
