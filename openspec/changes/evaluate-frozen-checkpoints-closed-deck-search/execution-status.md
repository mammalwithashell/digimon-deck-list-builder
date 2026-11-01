# Frozen checkpoint evaluation: review snapshot

Snapshot prepared for the draft PR on 2026-11-01 UTC. This is a review summary,
not the live host ledger. The full operational history and immutable binaries,
requests, partial receipts and archives remain under the original workspace's
`runs/` directory and are not included in Git.

The finite program compares frozen CP8, CP12, CP16 and CP20 before choosing the
next training experiment. It adds no training or model promotion. The scheduled
strength study contains 22 panels / 11,200 games: six closed-search panels
(3,120), six aligned known-list controls (3,120), four within-checkpoint mode
comparisons (2,080), and six held-out panels (2,880). Search uses 320 simulations,
four worlds, batch size four and no root noise. Mulligan diagnostics and the
opt-in Tamer-first baselines are separate protocols.

## Review order

1. `examples/search_quality/frozen_modes.rs` and `request_driver.rs` under
   `code/digimon-engine/`: actor modes, frozen requests, exact legal plans,
   telemetry and whole-pair failure propagation.
2. `code/tools/cloud_search/frozen_mode_{campaign,receipts,allocation,queue}.py`:
   immutable attempts, strict receipt validation, host claims and failure drain.
   Preparation, parity, profiling and reporting modules enforce the same inputs.
3. `code/digimon-engine/src/trigger_context/event_card.rs`,
   `src/determinize/{closed,context_guard}.rs`, registry and selection changes:
   unavailable event references discard hidden handles and reject reads;
   compiler-owned structured filters have a distinct path; opaque callbacks
   remain guarded. Factory/output identity is construction provenance, not a
   transitive safety certificate. The prototype context/world markers do not
   supply production permission.
4. Search-opportunity, mulligan and Tamer-first modules and their tests. The
   existing heuristic policies retain their behavior; new baselines are opt-in.

## Current blocker

The latest Linux closed CP12-versus-CP8 attempt failed at request `match-117-1`,
second game / decision 44, with
`closed-list projected context reached unproven callback: selection_filter`.
The native process exited 101; the container was not OOM-killed. Its 237 committed
receipts / 474 partial games were retained and individually validated. The panel
is failed, not complete; partial win rates are not reported. Its 15 successor
panels did not start. The independent Windows CP16-versus-CP12 queue remains a
separate attempt and was not modified while preparing this PR.

A subsequent diagnostic binary built successfully but rejected its prior binding
before reaching the failed decision (`unsupported prior schema or changed
compiled legality snapshot`). It did not produce a new root-cause trace. The
selection failure is unresolved, and this PR must remain draft. Any repair needs
new qualification and a new attempt identity; old controllers and partial
receipts must not be resumed or pooled.

## Validation and provenance

The PR is based on `origin/main` at
`751ebca2bd93909765c5e95718074a15d331baf9`. Current-main source-identity and
security-publicity changes are retained. The operational workspace is unchanged;
its unrelated training, frontend, dated launch/archive scripts, models and run
artifacts are excluded. Source line endings were normalized in this PR copy;
this tree is not byte-identical to any qualified historical runtime.

Fresh PR checks: 277 Python tests passed; 24 native subprocess tests were skipped
because they require explicitly supplied native binaries. An initial collection
failure identified the omitted `opening_hand_review` dependency; including that
existing helper resolved it without changing test assertions. OpenSpec strict
validation passed. Linux `cargo check --keep-going --locked --release -p
digimon-engine --features portable-policy-math --lib --tests --examples -j 3`
passed for the final integrated Rust sources (warnings remain). No Rust behavioral
tests, PyO3 rebuild, model inference or new scored games were executed for this
PR. Machine-readable commands and scope are in [validation.json](validation.json).

Integration checks exposed current-main event-reference conversions, two synthetic
queue fixtures missing the new source-identity field, a legacy diagnostic trait
method collision, existing Linux-sensitive card fixture paths, and an existing
source-index fixture type mismatch. The fixes preserve assertions and use the
checked read API. Earlier failed checks and the temporary bundle's missing-data
failure remain recorded in the local packaging evidence. The original workspace's
195 selected source files are byte-unchanged.

The pre-integration Assembly runtime had 112 software checks per OS, 134 reference
requests / 268 excluded games per OS, and exact parity across 268 games with no
non-timing differences. Those results do not qualify this PR's integrated tree.
Local qualification proof:
`runs/ex13-assembly-qualification-report-20261031/qualification-verified.json`,
SHA256 `b81c76226c21a783befbf35f5c8becb7b83e2b15b1814c66f2c40dd326c859ef`.
Preserved failed-panel evidence:
`runs/ex13-assembly-scored-failure-20261101/failure-preserved.json`.
PR packaging/check evidence:
`runs/ex13-evaluation-pr-20261101/`.

These local paths are evidence indexes, not artifacts available in a fresh clone.
The 21/35 task count is retained as historical program progress; checked
qualification tasks refer to the recorded runtime, not automatic qualification of
future code changes. Full scored outcomes, fresh integrated Windows/Linux
qualification, final profiles and the next-experiment decision remain unfinished.
