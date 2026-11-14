# EX13 Royal Knights training admission

Base: `6b1a4fce1` (main, restored and reconciled 2026-11-13).
Original reviewed patch base: `6e175a96d31e07dc397036dcb6bafab31141dd7e`.
Local isolated branch: `codex/ex13-royal-knights-training-gate`.

The unchanged two-layer gate admits **16/16 exact Royal Knights lists tagged
EX13**, previously 0/16. Their union is **75 unique cards**. Across historical
Royal Knights lists, **88/108** pass, previously 0/108. `admission.json` preserves
the exact cards, source URLs and before/after blockers for all 16 lists. No source
list was altered or relabeled. This is simulator training admission, not
family-wide rules or Oracle certification.

## Implementation

- BT13-112 Omnimon: official Q2367 requires playing all eligible distinct names
  or none. Source selection now enforces that requirement without the old
  12-name cap, retains physical duplicate choices, and resumes correctly in clones.
- Official Q2369 exposed missing ACE Overflow when trashing a breeding stack.
  That movement now uses the existing owner-relative Overflow processing.
- Restored BT13-102 Keenan's printed opponent-turn observer. Its old ignored
  test crossed the turn boundary at zero memory and observed the next turn's
  normal unsuspend. Acceptance, decline, both turn scopes, hard-play exclusion,
  already-suspended exclusion and the original zero-memory case now pass.
- Authored ST15-16 Trident Arm, P-238 Destruction Cannon, BT19-023 Huankunmon,
  BT20-026 MegaSeadramon (X Antibody), BT21-032 Veemon, BT9-098 Awakening of the
  Golden Knight, and ST17-13 Magnamon. Current main now supplies P-248
  Veemon and its metadata;
  this PR retains that implementation and all existing registry assignments.
  The action/tensor contracts are unchanged.
- ST17-13 uses three tested generic capabilities: selected-permanent rules-color
  count, exact effect-source-card evolution eligibility, and the existing
  end-of-battle scheduled continuation. Real security battles verify exact card
  identity, legal optional hosts, decline, no legal host and cloned continuation.
- BT9-098's DP protection is opponent-only and confers no extra De-Digivolve
  immunity. Own DP reduction still applies; declining evolution grants nothing.
- P-238's Use Req. is restricted to CS Digimon/Tamers, as CR 16-42-3 requires.
  The negative Option-enabler action-mask test failed before this correction.
- Added behavioral construct proof for source traits, breeding trash, printed
  keywords, triggering-card text and deleted-event level boundaries.
- Revalidated BT9-109 (15 canonical tests) and LM-028 (22); their blocked ledger
  statuses predated already-implemented shared primitives. Verdict updates are
  supported by production-card tests, not policy exemptions.

## Validation

Six focused targets reuse canonical card tests and cover all 75 admitted cards,
plus the unchanged EX3-024 regressions. Results:

| Target | Passed | Existing ignored |
| --- | ---: | ---: |
| royal_knights_gate | 513 | 2 |
| royal_knights_remaining | 64 | 0 |
| royal_knights_blocker_audit | 37 | 0 |
| royal_knights_hybrid | 186 | 0 |
| royal_knights_new_cards | 17 | 0 |
| royal_knights_craniamon | 30 | 0 |
| Total | 847 | 2 |

The two ignores are pre-existing EX8-074 tests; they are not counted as passes.
The new-card target includes matrix cases across both effects/owners, eligibility
boundaries and optional branches. New Royal Knights primitive tests: 9 passed, plus the player-directed
forced-attack scope witness.

- Current-main tier zero: all 13 checks pass, including 183 Python tests.
- All six focused targets above pass on current main: 847 passed, 2 pre-existing ignores.
- Current-main tier one: all six suites pass. DSL 1,683 passed/1 ignored;
  judge quiz 44/1; rules conformance 362/1; rules FAQ 31/0; keyword matrix 3/0;
  determinism guard 2/1 (41 unrelated tests filtered out). Zero test failures.
- Current-main exact-list smoke: 32/32 games completed, zero failures or
  step-cap exits (one combined test passed in 699.08 seconds).
- The first broader run was interrupted after 321 passing DSL tests and is
  preserved separately; it is not counted as a completed suite result.
- The original version 3 bundle preserves earlier-base validation separately
  under `original-v3/`; the numbers above are from the current-main rerun.
- The smoke fixture uses the 16 exact production-metadata lists, two seed
  parities each, legal masked actions, no concede actions and no forced
  step-cap result.

The first current-main default-profile engine-library build was killed with
SIGKILL on this cloud host. The successful retry uses one build job,
optimization level 1, debug information disabled, incremental compilation off,
and normal debug assertions. Focused tests use two test threads; the resumed
broader verification uses four. This is not a full-card-suite claim; the
original bundle also records a resource-killed monolithic cards_behavioral build.
Tests use the official onnxruntime 1.24.2 wheel library through ORT_LIB_LOCATION,
ORT_PREFER_DYNAMIC_LINK and LD_LIBRARY_PATH. No dependency source or repository
build profile was changed.

## Final exact list and scope

`dcg_nexus_627d76f178c6` now passes unchanged, including both BT13-077
Craniamon copies. James explicitly approved resuming only the Craniamon work
needed for this list. Its other former blocker, P-238, was qualified in version 2.
The admitted union adds BT13-077, P-238 and already-implemented BT22-041;
BT22-041's nine canonical tests are included in the final focused target.
No exact EX13 list remains blocked. Twenty other historical lists remain
outside admission; this does not claim every historical Royal Knights list or
all card interactions are certified.

### Craniamon authority and engine change

- Official Q2316/2317 make the initial controller choice optional despite the
  old image/bundle wording. Q2318 permits selecting unable attackers, which
  do not attack. Q2319 forbids a second declaration during an existing attack.
- Q2320 explicitly directs the player, so immunity of the chosen Digimon does
  not prevent an otherwise legal attack. The reusable DSL field
  `force_attack.affects_player` defaults to false; only Craniamon opts in.
  Ordinary legality, mandatory opponent target selection and the existing
  cloned BeginAttack continuation remain shared.
- The old path failed the Q2320 test before the change, with all 13 EX3-024
  tests passing. The new path passes Q2320 for both owners and clones while
  preserving both EX3 face/inherited immune-attacker regressions unchanged.
- CR 4.3 15-15-5-1/3 remains the default for Digimon-directed effects. Pinned
  DCGO's BT13_077 skips attacker immunity, while EX3_024's face checks it.
  Its inherited branch lacks that check: a pre-existing reference inconsistency,
  not authority to silently change the repository's tested inherited behavior.
- All eight Craniamon tests pass: immunity source-kind/controller/expiry,
  On Play/When Digivolving, live Blocker and Q2315, optional choice, unable
  attackers, attack restrictions, target ownership, both owners, clones and
  Q2319. The DSL test independently distinguishes both immunity scopes.
- Authority: https://world.digimoncard.com/rule/?card_no=BT13-077 ; official
  face https://world.digimoncard.com/images/cardlist/card/BT13-077.png ; pinned
  DCGO `02922babedf58c83b0687eeb91e2377cc977c58e` BT13/Black/BT13_077.cs and
  EX3/Blue/EX3_024.cs. Face SHA256:
  `817b682ee6fa7d729c2797ace7286f23b79388791e2fd6d81b71c978f5d6c01b`.

The global mechanics artifact remains ready with 314 unflagged open gaps.
No mechanics flags, admission rules, or failed-test requirements were relaxed.

## Sources and delivery

The restored patch was reconciled with current main, retaining its forced-attack
turn-player guard (CR 11-1-2), P-248 YAML/ledger/metadata and other independent
changes. The generic forced-attack witness explicitly stages the attacker's turn.
Generated admission and coverage artifacts are rebuilt from the resulting tree.

Official card faces and committed bundles were inspected for the implemented
cards and repaired core clauses. P-248's official source:
https://world.digimoncard.com/cards/?category=522901&search=true .
Official Q&A: data/card_qa.json Q2366–Q2369 and Q833–Q834. Canonical rules:
docs/digimon-rules/cr-4.3.txt, including source-card, Overflow, security-battle,
De-Digivolve and Use Req. rules. The pinned DCGO reference
02922babedf58c83b0687eeb91e2377cc977c58e BT13/White/BT13_112.cs corroborates
Omnimon's all-or-none selection. Live re-fetch of DigiLab 119763 returned 401;
the committed exact list remains the source of the report and smoke fixture.

The original version 3 review ZIP supplied the restored patch and its prior
validation logs. The PR includes the reconciled implementation, exact-list
admission report, and supporting tests. Local Oracle-readiness metadata was
checked; no training or Oracle workload was run. Daily jobs remain paused.
Only the explicitly approved narrow Craniamon work was resumed.
