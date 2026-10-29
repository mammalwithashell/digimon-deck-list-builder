# BT25-089 App Fuse OPT followup

This followup addresses the sole verdict hold left by the original top-10
construct campaign: BT25-089 (Kazuki & Itsuki) in Rebootmon. The original
[completion report](completion-report.md) records the historical 9/10 result.

The regenerated per-card artifact now admits **10/10 lists and 167/167
unique IDs**, across 540 card entries. Rebootmon has no remaining blocked
card in this frozen pool. BT25-089 is `AUDITED-OK` with 16 card tests.

## Behavior and authority

BT25-089's End of Your Turn effect offers a whole-effect optional App Fuse.
Declining either selection or finding no eligible pair preserves its
once-per-turn use; committing the fusion consumes it. The printed card image,
official Q1818, and Comprehensive Rules 4.3 section 15-14-1-4 were checked.
DCGO's BT25_089.cs provides corroborating implementation behavior.

The reusable DSL step now accepts an optional `bind_as` Card receipt. It is
cleared before each attempt and populated only after a successful commit.
Receipt-bearing attempts continue the enclosing process after PASS, allowing
BT25-089 to explicitly call `refund_opt` when its receipt is absent. Unbound
App Fuse steps retain their previous PASS behavior. This is not a generic
refund for declining a later step after an effect has already done work.

Both prompts remain player-visible. Resolution checks the original host's
identity and current result/material legality. Receipt clearing propagates
through nested continuations and cloning; rebinding must supersede older
receipt clears. The public EffectContext App Fuse API remains compatible.

## Regression evidence

The original host-decline test failed when corrected to require another
same-turn opportunity. A nested receipt-reuse regression also failed before
the continuation correction (memory 6 instead of 9).

Two later rebinding regressions initially failed because their fixtures used
`from: self` (a Card) with a permanent-property operation. Correcting them to
`from: source` made the focused run pass on unchanged production code. Those
initial failures are fixture evidence, not confirmed engine defects. The
corrected tests retain same-clause, cross-clause, and unbound-success coverage.

Coverage includes both PASS prompts, no eligible pair, empty result zone,
invalidated materials/result/restrictions, host removal and replay,
field compaction, cloning at both prompts, legacy unbound behavior, and
preserving OPT consumption when earlier processing already occurred.
Both fresh spec and quality reviews approved the final scoped continuation
design. Runtime validation is recorded below.

## Final-source validation

| Check | Result |
|---|---|
| Integrated App Fuse consumers (BT21-084, BT23-079, BT24-087, BT25-089, P-241) | 81 passed |
| Integrated DSL App Fuse and folded-may refund cases | 28 passed |
| Selection / clone fuzz / timing dispatch | 79 / 1 / 52 passed |
| Replay goldens | 10/10 verified, unchanged |
| Tier 0 | 13/13 checks, including 174 tool tests |
| DSL vocabulary structural check | In sync |
| Per-card admission, frozen pool | 10/10 lists; 167/167 unique IDs |

The optional `implementation_inventory` binary check fails with 27 existing
exhaustiveness errors. A package copy from pre-followup commit `cf9ecbcdc`
reproduces the same 27 error messages; the App Fuse field additions introduce
no additional error. This is recorded separately from the passing engine
checks and is not represented as a successful inventory build.

Local logs are under `.superpowers/sdd/2026-10-22-app-fuse-opt/`, including
`final-integrated-corrected.log`, `final-safety-reviewed.log`, `replay.log`,
and the current/baseline inventory checks. These are ignored local evidence;
committed tests and the frozen deck list provide reproducible inputs.

## Scope limits

This closes the result/OPT residual. Separate App Fuse procedure issues
(draw, cost, and link retention), effect-link timing, and the other campaign
engine gaps remain documented. Per-card admission is not a claim of full
rules fidelity or DCGO confirmation. The gate and action/tensor contracts
are unchanged. Historical aggregate failures in the original completion
report remain recorded; focused followup results do not replace those runs.
