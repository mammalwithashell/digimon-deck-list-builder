# Generation 2 loop and uploader recovery

The continuation stopped before fitting generation 2. All 1,100 games had been
collected, but game 593 in chunk 592 reached the 1,200-decision cap. Generation
1 and its completed evaluations were intact.

## Confirmed game failure

The exact Chronomon versus ShineGreymon Data Squad game was reconstructed from
the sampled prefix and committed search targets. All 1,193 stored observations
matched. From decision 67, player 1 repeatedly activated field-slot-2 Gekomon
(BT26-021, action 1022) and declined its optional trash selection (action 62).
The same Main state occurred 567 times; its selection state occurred 566 times.

Gekomon's YAML explicitly refunded its once-per-turn use when the pick was
declined. The local DCGO reference has that refund too, but CR 4.3 15-14-1-4
counts the chosen activation even when processing is not executed. This is
the same distinction previously corrected for Manekimon. The printed Main
effect remains optional; declining plays nothing. It simply cannot reopen the
spent activation. Gekomon's inherited triggered effect retains its separate
pre-activation decline behavior.

The corrected card test failed before the YAML change and all nine Gekomon
tests passed after it. Another copy retains its independent use. All 13 tier-0
checks passed after refreshing generated impact/coverage metadata. The native
image was built from `6ff39f838c2e0ee28cdc9dc17fdfed445e966568` in CI run
`40316486433`; the native change from the prior image is the Gekomon YAML fix.

The original shard arrays and commit manifest remain unchanged. A
manifest-bound `training_exclusions.json` excludes only game 593 from both
policy and value losses. The recovery preflight checked all 825 chunks across
the parent and two new generations: 290,628 usable positions remain after
excluding the loop's 1,193 rows. No other game was discarded. New collection
still fails on any unreviewed truncation.

## Confirmed uploader failure

Kernel logs show the monitor's Python process was killed at its 1 GiB cgroup
limit while the second evaluation report became available. Each complete JSON
report is about 245 MiB, with additional decoding and W&B table overhead.

The monitor now releases each decoded report before reading another, uses
`json.load`, and persists artifact-upload acknowledgments before proceeding.
Its container allowance was raised to 4 GiB. Restarting the same W&B run
successfully backfilled the missing report; the publisher confirmed two card
reports uploaded. The spool and original failure logs were preserved.

## Explicit recovery contract

`recover_small_generations.py` requires a hash-bound recovery plan, matching
runtime/image identities, an explicitly failed campaign, unchanged inputs and
last checkpoint, complete committed chunks, reviewed exclusions, and no fitted
or later-generation output to overwrite. Forty-two focused Python tests passed,
including recovery drift rejection, unchanged search parameters and quarantine
loader tests. Cloud preflight also exercised the actual retained replay.

Recovery refreshes checkpoint 1's evaluation on the fixed engine in window
1001, preserving original windows 0 and 1. It then fits the audited generation
2 collection, evaluates in window 1002, and collects/fits/evaluates generations
3 and 4. Search stays at 320 simulations, four worlds, batch four and eight
workers. Fitting retains the original two epochs, batch 512, learning rate
0.0001 and three-generation replay window. There is no automatic promotion or
automatic retry.

Cloud evidence is under
`/opt/digimon-search/mixed-continuation-20261026/diagnostic/`, including
`summary.json`, `truncations/593/replay.json`, `recovery-preflight.log`, the
original monitor unit, and the fixed-engine validation. The replay report's
SHA-256 is `76a05c5e5931b121d46ed1a4a09b9bf2eb0a7d1b5e5d9ba7d68ab8ab218afcf3`.
The recovery controller and monitor overlay are commit `3a1a24c01`.

## Deployment verification

CI run 40316486433 completed successfully. The deployed image is
`ghcr.io/mammalwithashell/digimon-search-cpu@sha256:6a6515fb1864096f1a431e8a296ce5998dfbfa40206ea501272ca06702d00f5f`.
On this exact image, replaying the first 69 recorded decisions leaves the Main
activation spent after decline. A fresh full-budget rerun of game 593 using
the same checkpoint, seed, decks and 320/4/batch-4 search finishes normally in
95 decisions (87 saved positions), rather than reaching the 1,200 cap. The
9434-float tensor and 2192-action model contract is unchanged.

Final recovery validation on the new image passed for all 290,628 retained
positions and exactly one excluded game. The existing campaign service was
explicitly restarted with the recovery controller; its previous unit,
deployment plan, failed state and logs are preserved in `recovery-history/`.
The same W&B run and dashboard cache remain in use.
