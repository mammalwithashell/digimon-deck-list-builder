# Deletion provenance and interrupted removal repair

Follow-up to PR #752, based on main `162afa8cc74dce9956feda5808aeddc84e758ab6`.
OpenSpec: `fix-deletion-event-identity`.

## Reproduction and rules basis

Chronomon: Holy Mode (BT26-016) attacks, then its When Attacking effect chooses
MarineBullmon (EX12-031) for deletion. MarineBullmon has Shellmon (EX12-026)
under it. The initial native regression fails because Decode is not offered;
the control that declines Chronomon's effect and defeats MarineBullmon in
actual battle passes. The pre-repair result is 1 passed / 1 failed, recorded
locally in `runs/deletion-event-identity-20261026/cause-red.log`.

After correcting cause inference alone, the expanded regression set records
4 passed / 7 failed in `window-red.log`. Both normal/security effect-versus-
battle controls pass. The remaining failures demonstrate premature subject
departure at Arrival's reward, Decode being offered twice, an incorrect
return-to-hand continuation, reused-incarnation suppression, and suppression
of a second independent nested deletion attempt. This separates the cause
repair from the window and guard repairs.

With Sanmyojin Arrival (EX12-070) present, the old hand-reward adapter pays
its self-trash, finishes the subject's leave by starting another deletion as
OwnEffect, then offers the reward. A slot-keyed replacement guard suppresses
the second Decode when MarineBullmon slides into Arrival's old slot, but not
when another permanent separates them. The prompt difference is an engine
defect, not a field-order rule.

The checked printed card faces and CR 4.3 sections 15-8-5 and 16-36 require
the distinction between effect and battle destruction. Immediate effects
interrupt before the cause and each can activate once until that cause
resolves. Arrival's DCGO source also pays its own departure and performs the
reward inside its immediate body; it does not delete the observed Digimon.
DCGO source inspection is supporting evidence, not a live oracle run.

## Repair contract

- Effect deletion retains the initiating controller while an attack or
  security procedure is pending. Actual normal/security battle supplies
  Battle explicitly.
- Hand/union Delay processing completes its payment and reward inside the
  original replacement window, then resumes its original cause, destination,
  subject, and remaining candidates.
- Replacement guards follow the battle-area incarnation through compaction.
  A replayed physical card is a new incarnation, and independent deletion
  attempts have independent activation guards.
- Choices remain exposed through masks. Tests clone at suspended decisions
  and compare continuation, while also asserting independent final-zone,
  cause, activation-count, and physical-card expectations.

## Validation

- Focused replacement regressions: 13 passed, including 24 real Arrival
  order/layout/choice branches and cloned continuations (38.82 seconds serial).
- Final engine unit suite: 511 passed (36.06 seconds).
- Final selection suite: 88 passed (67.33 seconds with four threads).
- Final replacement suite: 139 passed with four test threads (21.03 seconds).
- Focused union Delay DSL contracts: 3 passed (2.57 seconds).
- Affected card regressions: BT19-099 22 passed, EX12-036 13 passed, and
  ST20-14 18 passed.
- Repository Tier 0: 13 checks passed, including 183 verification-tool tests.
- OpenSpec strict validation and diff whitespace checks passed.

These final-source suites total 794 passing tests; the 13 focused regressions
are a subset of the replacement suite. An earlier broader build passed combat
(299, one pre-existing ignore), cost hooks (100), and deletion batching (13),
but found the ordinary-deletion lazy-identity regression. The repair preserves
the existing assertion, which passes in the final 511-test unit suite. That
older build was stopped during DSL execution so it is not a completed broad
suite result. Full Linux CI remains the broader integration gate.

Initial Linux CI passed the rules/core groups but failed one of 1,171 exam
scenarios: `EX12-070-effect4.yaml` deliberately asserted the old simulator bug
documented in its own caveat (the bounced Shishimamon went to trash). The native
harness reproduced exactly that hand/trash mismatch. The expectation now keeps
Shishimamon in hand, as printed MarineBullmon/Arrival text and DCGO source
require; the corrected scenario passes locally. All other asserted state and
scripted decisions are unchanged. This is a rules-backed simulator regression,
not a new oracle result: the historical `unreachable` prompt-shape verdict is
preserved. The complete exam corpus remains part of the follow-up CI gate.

Two concurrent focused runs were
stopped after several minutes without completion, while serial tests passed.
The repository already documents pathological Windows system-heap contention
for allocation-heavy tests and uses `mimalloc` in the card and DSL harnesses.
Applying that same test-binary allocator to the replacement harness let all
139 tests pass with four threads in 19.80 seconds, with unchanged assertions
and engine semantics. Temporary diagnostic output was removed. Production
allocation is unchanged.

The historical game-850 numeric prefix remains unchanged. Its original
continuation test is updated around a native real-card state because previous
mandatory-trigger corrections already changed that prefix's action sequence.
A separate explicit synthetic reaction to Arrival's payment retains genuine
nested-selection coverage without relying on the erroneous second deletion.

## Operational scope

The diagnostic training run, image, decks, and search settings are unchanged.
Existing data remains evidence from the engine revision that generated it.
No model-strength, full-pool rules-fidelity, or live DCGO certification follows
from these targeted engine regressions. Broader documented replacement
cross-timing/prevention limitations are outside this repair.
