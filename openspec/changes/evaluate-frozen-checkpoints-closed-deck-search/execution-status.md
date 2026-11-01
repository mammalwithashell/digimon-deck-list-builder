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

Both latest scored attempts are terminal failures. Linux CP12-versus-CP8 stopped
at `match-117-1`, second game / decision 44, with a `selection_filter` guard
failure; 237 receipts / 474 partial games remain preserved. Windows
CP16-versus-CP12 stopped at `match-083-1`, second game / decision 26, when the
Petrification token factory was treated as opaque; 167 receipts / 334 partial
games remain preserved. Neither panel is complete, and partial win rates are
not reported. Old controllers remain stopped.

Exact original inputs now reproduce both failures after 44 and 26 legal actions.
The earlier diagnostic prior rejection came from an empty embedded card pack:
the build script retained a previous compile-time mount path. It now resolves
the manifest directory at execution time, preserving the prior-binding checks.

Repairs bind the fixed Petrification constructor to its exact registry entry
and preserve compiled predicate data through count-capped/material picks,
security selection and choose-from-reveal. Opaque replacement factories and
callbacks still reject before their bodies. Actual unavailable event-card reads
still reject. Security recovery retains its existing mandatory-refresh and
optional-decline contracts.

Both original saved roots now complete 320 simulations / four worlds / batch
four with the original model, prior, request and search-seed bindings, on fresh
Windows and Linux diagnostic binaries. Each completes 81 evaluator calls / 324
rows and legally executes its selected root plan. Both platforms produce exact
non-timing search parity at these two roots. These are root diagnostics, not
complete games, pair replays, full qualification or scored panel results. Their
original legal prefixes retain identical inspected non-Debug state; queue/resume
Debug strings also reflect current-main source-identity metadata and unordered
binding-map display, so whole-history byte identity is not claimed.

The PR remains draft while full integrated qualification, profiles and the scored
study remain unfinished. Automation is paused; work proceeds directly. Fresh
attempt identities are required before scoring resumes.

## Validation and provenance

The PR is based on `origin/main` at
`751ebca2bd93909765c5e95718074a15d331baf9`. Current-main source-identity and
security-publicity changes are retained. The operational workspace is unchanged;
its unrelated training, frontend, dated launch/archive scripts, models and run
artifacts are excluded. Source line endings were normalized in this PR copy;
this tree is not byte-identical to any qualified historical runtime.

Fresh PR checks include 277 Python tests passed / 24 opt-in native tests skipped,
OpenSpec strict validation, and Linux compilation of the integrated Rust targets.
After CI reported failures, the CI repair commit passed all 73 determinization
tests, seven targeted unit regressions, Tier 0 (including facade placement),
and the regenerated lowering-coverage check. The source-identity fixture update
preserves its 28 assertion lines, and the relocated partition fixture is byte
identical.

Eight new runtime regression tests have passing evidence. The first batch was
35 Windows tests passed, including four new tests. The later batch initially
produced 11 Windows passes and one new-fixture failure: an optional invalidated
security menu correctly offered only decline, contrary to the fixture's assumed
commit. The fixture now exercises legal mandatory refresh and optional decline;
all its original assertions remain. The corrected test passed separately on Linux;
the other 11 tests were not repeated and no fresh 12-test aggregate is claimed.
The final repaired Windows runtime also passed all 73 determinization tests.
Production code was unchanged after the successful saved-root replays. Initial
failures, binaries, sources and logs are retained.

Windows native diagnostic builds and exact-root model inference have executed;
no fresh scored games, PyO3 qualification or full integrated qualification have
run. Machine-readable scope is in [validation.json](validation.json).

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
`runs/ex13-evaluation-pr-20261101/`. Repair and two-root parity evidence:
`runs/ex13-new-failure-repair-20261101/`.

These local paths are evidence indexes, not artifacts available in a fresh clone.
The 21/35 task count is retained as historical program progress; checked
qualification tasks refer to the recorded runtime, not automatic qualification of
future code changes. Full scored outcomes, fresh integrated Windows/Linux
qualification, final profiles and the next-experiment decision remain unfinished.
