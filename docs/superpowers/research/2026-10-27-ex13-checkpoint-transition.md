# Repaired-engine checkpoint assessment and EX13 continuation

The user approved this rollout after merging PR #753 (`a640a4aaa`). The
previous fixed-engine campaign is retained as diagnostic evidence. Its four
1,100-game collections and fitted checkpoints completed; checkpoint 4's final
evaluation failed after 3,386 completed games. This document records a new
assessment and continuation, not a successful recovery of that failed window.

## Frozen source

- Campaign: `english-meta-fixed-4x1100-20261026`.
- Image source: `6627540bab607109b9eb8d5216ff647b8e4b9ae8`.
- Checkpoint 3 SHA-256:
  `f83eadfe779a0a559fa72e0eff40e8dc3e82c03e9fd29a245ea9535de629dd96`.
- Checkpoint 4 SHA-256:
  `9e575c4cf6e51300c98e0c59e0df5979ed54d1105486ac324520e079b56ecf75`.
- Both ZIP archives passed CRC verification before the transition.
- Old replay, reports, failure recording, campaign state, and frozen inputs
  remain unchanged. A separate checkpoint/evaluation archive records file hashes.

## Additional failure reproduced

The recorded Glowing Dawn versus SW game selected end-of-turn attack 116 from
Monarchlizamon into Paishu Token. The mask accepted the token as an effective
Digimon, but the decoder checked only the printed card kind and rejected the
same action. The resulting repeated no-op was reproduced on merged PR #753.
The targeted repair makes decoding use the same effective-kind predicate as
masking. The original recording is retained as a native regression, including
cloned continuation. Search budgets and observation/action contracts are unchanged.

The first image build from the merge alone was cancelled before publication
when this additional defect was identified. The deployed release must include
the verified token-target repair as well as PRs #752 and #753.

## Assessment protocol

Evaluate checkpoints 3 and 4 under the same new immutable image, frozen old
deck panel, anchor weights, seeds and starting-player schedule. Use 200 games
per archetype per anchor, or 7,800 games per checkpoint. These are policy-only
evaluations with full card telemetry. The current schedule is
`archetype-paired-v2`; historical v1 reports are not paired with these results.

Require complete, identical scheduled cohorts and preserve failed windows.
Compare checkpoint 4 minus checkpoint 3 with paired confidence intervals.
For this continuation, select checkpoint 4 only if the lower 95% paired bound
is at least -2 percentage points against each learned anchor and at least
-1 point against greedy; otherwise retain checkpoint 3. This conservative
choice is initialization for an experiment, not champion promotion.

## New campaign

Use a separate directory and W&B run. Validate the selected checkpoint's
contract against the image and copy weights only into generation 0. Do not
import old replay or alter the failed parent state to satisfy a completion
check. Bind preparation to the complete assessment, selected checkpoint,
image, controller, card data and frozen pool hashes.

The refreshed EX13 training pool has 13 archetypes, 26 equally weighted
sampling slots and 21 distinct lists. Retain every versioned pool asset and
revalidate native legality, registration and the training admission gate.
Run full-budget native health games before launch.

Retain four generations of 1,100 games, eight workers, 320 simulations, four
worlds, leaf batch four and a three-generation replay window. Each generation
has about 169 expected appearances per archetype; this is not an exact quota.

Evaluate each new model once on the combined EX13 diagnostic panel and the
five original pet-list regression panel: 10,800 games per checkpoint with
three anchors and 200 games per candidate archetype per panel. The combined
panel has unseen lists for 12 archetypes and a same-list Light Hybrid fallback;
it must not be called entirely held out. The original pets remain a separate
regression panel. Baseline evaluation precedes new training.

The training service fails closed on incomplete evaluations or truncated
collection, does not restart automatically and does not promote a model.
Its W&B sidecar reads experiment artifacts; only that sidecar receives logging
credentials. Final deployment and assessment results are recorded in the
run's hash-bound artifacts and summarized in the rollout handoff.

## Local verification before image publication

The exact recorded attack and effective-kind controls failed before the repair
and pass afterward, including cloned continuation. The combat suite passed
302 tests with one pre-existing ignored test. Transition/preparation/publisher
tests cover immutable inputs, failed-parent preservation, complete matched
cohorts, recomputed selection, runtime contracts, report hashes and upload
retry state. Repository Tier 0 passed all 13 checks.

The 105-file checkpoint/evaluation archive was copied back to the development
machine and verified against SHA-256
`528cf3219216c29086137f206673ca9a1f3c4b8986d4c5501810b90c3ba59536`.
The previous campaign's replay remains intact on the source host.
