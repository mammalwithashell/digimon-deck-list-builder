# Dynasmon simulator admission — 2026-10-29

Fresh runtime gate result: **8/8 explicitly EX13 lists admitted**, **9/13 repository lists admitted**. Primary fixture passes (54 physical cards, 22 unique IDs); the EX13 pool contains 42 unique IDs. All nine admitted repository lists loaded opening games through a fresh Python binding. The full card suite passed: **12,929 passed, zero failed, 35 ignored**. No admission rule was weakened.

These results qualify source based on `d0d0520262`, not compatibility with audited main `77f4702f9`. [Current-main overlap review](dynasmon-main-overlap.md) records 17 overlapping paths and the required belief-knowledge integration. No rebase or integration was performed.

## Scope and provenance

Branch `codex/dynasmon-admission`, fetched-main base `d0d0520262c6d8a4df0216ca6b5743052fe24789`, isolated worktree `.worktrees/dynasmon-admission`. No training, paid oracle run, deployment, merge, push or publication.

The primary is [Tesero, Eagle's Nest, October 11, 4–0](https://digimon.digilab.cards/decklist/120647). Exact 50-main/4-egg counts are in `dynasmon-tesero-120647.json`. Its counts equal repository list `dcg_nexus_14e373eaf549`; these remain distinct source records.

Eight records explicitly say EX13 (2026-10-04 through 2026-10-16). Four older records (2025-10-24, 2025-11-09, 2025-12-20, 2026-01-03) and one DigimonMeta record (raw date `10/3/2026`) lack a format label. Dates/labels establish repository scope only, not tournament legality. The full historical 57-ID union is outside the implementation scope.

## Current per-list generated gate results

| List | Repository format | Gate | Blocking IDs |
| --- | --- | --- | --- |
| digilab_a737387543bb | unspecified | reject | BT14-003, BT18-040, BT19-039, BT5-086, BT6-032, BT6-034 |
| digilab_af2df43d57eb | unspecified | reject | BT14-003, BT18-040, BT19-039, BT5-086, BT6-034, P-122 |
| digilab_f4ea789db4bf | unspecified | reject | BT14-003, BT18-040, BT19-039, BT6-034, BT9-109, ST19-02 |
| digilab_f31700eaad7b | unspecified | reject | BT14-003, BT18-040, BT19-039, BT6-032, BT6-034 |
| dcg_nexus_68400172033e | EX13 | pass | — |
| dcg_nexus_14e373eaf549 | EX13 | pass | — |
| dcg_nexus_1c630c677d77 | EX13 | pass | — |
| dcg_nexus_89c093e2fb71 | EX13 | pass | — |
| dcg_nexus_1a7d2e005f29 | EX13 | pass | — |
| dcg_nexus_3c85ce6ea42b | EX13 | pass | — |
| dcg_nexus_4400068f1b09 | EX13 | pass | — |
| dcg_nexus_7e05941203f3 | EX13 | pass | — |
| digimonmeta_6eb3bde8bd2b | unspecified | pass | — |

The JSON records exact counts, source URLs, dates, labels, failure reasons, constructs and retained verdicts. `runtime_verified` is true: the fresh binding matched the actual gauntlet's admitted deck IDs and loaded every admitted repository list, plus the primary fixture, with `standard_lite_v2` observations (9,434 floats) and 2,192-entry action masks. This is opening-game verification, not training or search qualification.

## Implemented cards and reviews

| Card | Work | Focused card checks |
| --- | --- | --- |
| BT18-030 | implementation | 8 |
| BT18-036 | implementation | 16 |
| BT18-098 | implementation | 12 |
| BT19-036 | implementation | 14 |
| BT19-042 | implementation | 10 |
| EX6-063 | implementation | 11 |
| EX13-025 | audit/repair | 23 |
| BT18-039 | implementation | 12 |
| BT19-029 | implementation | 21 |
| BT23-035 | audit/repair | 26 |
| BT6-089 | implementation | 9 |
| BT9-084 | implementation | 11 |
| EX7-026 | implementation | 11 |
| LM-045 | implementation | 8 |
| EX10-041 | implementation | 18 |
| LM-020 | audit/repair | 13 |

Queue for the 42-ID pool: 13 implementations, 3 audits/repairs, 26 retained implementations. All batches passed independent specification and quality reviews. EX10-041 was also checked against the actual image, official bundle, YAML and behavioral tests. The final focused admission runner passed 351/351 checks, and the full card suite passed. BT23-035's historical FAIL is retained in ledger history; its 26 current production-pack checks and reviews establish simulator repair, with oracle confirmation unmeasured.

Shared work covers physical security-trash receipts, canceled/redirected/parked payments, native Barrier, complete owned-hand paging, exact deck-trash self triggers and original-DP absence. Every decision uses existing masks/pending selections. No action, tensor, model or frontend contract changed.

Continuation qualification separates immutable effect attribution from a live source carrier. Attached card effects cannot borrow their former host after detachment; externally granted effects retain their recipient carrier. Successful DNA resolution explicitly pins the resulting carrier for follow-ups, including cloned and compacted continuations. These repairs passed the final focused and broad reruns. LM-045 uses ordinary blue `[Security]` timing; an incorrect pink-security scope qualifier was removed after the production icon lint caught it.

The initial global maximum-assignment reveal filter was incorrect under [official Q4704](https://world.digimoncard.com/rule/?card_no=BT3-093). It was removed completely. Five existing regressions remain unchanged; the corrected overlapping-bucket test preserves every legal physical choice even when one choice yields fewer total cards.

## Verification evidence and limits

| Check | Final result |
| --- | --- |
| Full card regression suite | **12,929 passed, 0 failed, 35 ignored**; 3,254.32 seconds |
| Focused Dynasmon admission | **351/351 passed**, none ignored |
| DSL | 1,635 passed, 1 ignored |
| Judge quiz | 44 passed, 1 ignored |
| Rules conformance | 361 passed, 1 ignored |
| Rules FAQ | 31 passed |
| Keyword semantics matrix | 3 passed |
| Determinism guard | 2 passed, 1 ignored; 41 unrelated cases filtered |
| Engine invariants | 5/5 passed |
| Mask and tensor contract | 182/182 passed |
| Security-payment receipt units | 6/6 passed; 506 unrelated unit cases filtered |
| Public-security network filtering | 18/18 passed with pure-module pytest (`--noconftest`) |
| Tier-zero artifact/tool checks | 13/13 passed, including 183 tooling tests |
| Replay goldens | 4/10 verified; 6 digest divergences exactly match the independently built clean base |

[Structured verification evidence](dynasmon-validation.json) records all required manifest entries, shared executions for duplicate full-card/DSL labels, source hashes for all 120 changed code paths, and the exact replay signatures. The final broad reruns used the optimized engine (`opt-level=2`) and test harnesses built with `-C opt-level=0` to reduce compilation cost. Complete target test sets were retained. Validation ran at BelowNormal priority, with one test thread and `RUST_MIN_STACK=268435456`.

The earlier full pass exposed a detached-link regression and two EX12-001 DNA continuation regressions. All three original tests now pass in the full suite; the 21 unchanged BT26-037, EX12-001 and granted-selection-body regressions also passed separately, including clone and compaction cases. Native selection origins passed 3/3, floating origins 4/4, own-carrier immunity 4/4, and granted immunity 3/3. The native replay fixture funds its real three-memory play cost and asserts memory 7/turn 0, preserving its unexpired-modifier assertions.

The fresh debug Python binding was installed only in the task-local environment. The actual gauntlet admitted the expected nine repository IDs; every admitted list and the primary fixture loaded opening games. Direct Python main-thread invocation hit the Windows stack limit. The successful check used a worker stack of 268,369,920 bytes (the largest 64 KiB-aligned value below Python's Windows 256 MiB limit). Default-stack and optimized-binding performance qualification are not claimed.

Replay rows 000, 001, 002, 005, 006 and 009 retain the same failing steps and recorded/replayed digests as an independently built clean `d0d0520` checkout. These remain aggregate failures; no goldens were blessed or regenerated.

Simulator admission is not oracle certification or search/training promotion qualification. Unsupported security-trash substitutions and other-destination outcomes remain explicit invariants; this work does not certify every replacement family. Exact self-card deck-trash qualification does not imply generic trash-scope OPT support; Wizardmon has no OPT. The existing cross-source immunity-grant suppression limitation remains documented in `docs/RUST_ENGINE_GAPS.md`; LM-020 is the only immunity provider in the scoped 42-ID pool, and grants its own immunity. No readiness gate or blocker flag was relaxed.

## Legacy production metadata dependency

Fresh source review found that the five legacy text discrepancies already have faithful canonical projections. Eight proposed overrides were rejected and removed. The final staged ledger is byte-identical to the prior 370 rows (SHA256 `b5bfc8a644790d3fe1df17bac867ae9dda52b8ac8d59856156d5e4b59f75e71a`). An independent second review approved this scoped evidence. No production metadata changed.

Complete public acquisition covered 4,474 official IDs and 4,535 secondary IDs with zero failed source locations. The necessary secondary adapter repair uses discovery and bounded exact-ID batches, refuses truncation, retains source digests and paces requests. Related adapter tests passed (136 related checks, including 13 new checks).

Supported runtime installation requires a complete candidate. Final-input official-condition preflight reproduces 84 unrelated printing-condition conflicts, none among these five cards. No validated minimal text/security apply route exists; retired raw-ingest branches are not an acceptable alternative. A source-only preview of 15 field differences is not a generated full candidate, runtime cutover, or publication approval. Exact diagnostics, hashes and scope limits are under `dynasmon-metadata-review/`.
