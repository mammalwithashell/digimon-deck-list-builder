# Generation 3 failure and corrected-engine continuation

The `english-meta-pets-4x1000-20261025` campaign stopped during generation 3
with 976 committed games, two capped games, and no generation-3 checkpoint.
Checkpoint 2 and all original artifacts remain preserved. The failing engine
image is recorded in the adjacent regression fixture.

## Reproductions and repairs

- Game 340: SW versus Toho Braves, seed `16030609207348766736`. The native
  trace reaches decision 111 before search panics at `misc.rs:1307`, battlefield
  length 2/index 2. Replaying all 111 legal actions and the frozen checkpoint's
  exact 320-simulation/four-world/batch-four search reproduces the same panic
  locally. DigiXros's material-removal code locates moved materials by physical
  card identity, but adjusted the entering host index using the material's old
  slot. Host relocation now uses physical identity too, preserving every
  consumed card under the intended host. Both the focused relocation/clone
  regression and the recorded frozen-model search are retained.
- Game 521: Toho Braves versus TS Jupitermon, seed `10960339676619312696`.
  The original 1,200-decision replay matches every stored observation and
  repeats an orphan EffectChoice/PASS state from decision 22. The existing
  nested-Delay continuation repair preserves the replacement at decision 19;
  resolving it and the remaining choices returns to Main with real choices.
  A recorded-prefix regression also checks clone parity.
- Game 858: Toho Braves versus Three Musketeers, seed `2559991177805855381`.
  Darumamon BT26-014's optional Shambala retrieval returned Onibimon EX12-004
  to hand at decision 44. The mask then offered hand play 6 repeatedly from
  decision 49, although execution rejects Digi-Egg play. CR 4.3 3-1-3-9-2
  requires the bottom of the owner's Digi-Egg deck instead. The trash-to-hand
  primitive now redirects there and reports no actual hand addition (no false
  OnAddToHand trigger or DSL result binding). The hand-play mask rejects eggs.
  Printed BT26-014 image and DCGO's trait-only retrieval predicate agree that
  selecting the egg is legal; the destination needed correction.

Local validation before image publication: 13 private-destination tests,
four recorded-game tests (including the opt-in frozen checkpoint test),
12 campaign tests, and 16 monitor/scheduler tests passed; one existing optional
scheduler test skipped. Full library and CI checks are recorded separately.
These are targeted simulator regressions, not a DCGO certification or a claim
that every possible training loop has been eliminated.

## Continuation contract

The plan below describes the validated schema-1 continuation image. It has
not been started. The subsequent merge of main at `054a06b12` admits the saved
Light Hybrid list but changes the active model contract to 9,434 observations
and action schema 2. Checkpoint 2 is rejected by that contract. The new
13-archetype pool and two 1,100-game generation plan are prepared in
`2026-10-25-light-hybrid-pool.json`; restart remains pending an explicit model
strategy. Neither the preserved checkpoint nor its metadata has been changed.

`deploy_repaired_continuation.py` prepares a separate campaign from checkpoint
2 and never mounts or copies prior-engine replay shards. Two 1,000-game
generations finish the originally approved four updates. The first update
uses 1,000 fresh games; the second pools 2,000, within the unchanged three-
generation replay window. Search remains 320 simulations / four worlds /
batch four, eight workers, cap 1,200; fitting and rewards are unchanged.

Checkpoint 2 is evaluated first under the corrected engine on the same fixed
cohort (200 games per archetype per anchor), then each new checkpoint is
evaluated once. W&B uses a new run ID with explicit parent provenance. The
controller retains fail-on-truncation, no automatic restart, and no automatic
promotion. Immutable image digest, source revision, checkpoint, inputs, and
runtime hashes are verified before start. Linux image validation must pass
before writing a successful parent-validation report or starting the service.

## Image validation and main integration

The schema-1 CPU image from commit `52750be64631e09b376491b8560513f76a0e2fe1`
built successfully in GitHub Actions run `40213459155`. Its immutable digest is
`sha256:3502b10431373a0538e8a970dde0b3460847879155e87e4d55c6df79869c2d96`.
Linux preflight passed all 27 full games (the three recorded failing seeds plus
24 additional games), with the preserved checkpoint and unchanged 320/4/4
search settings. No game crashed or reached the decision cap. The old-image
control failed the orphan-choice check as expected. This evidence applies to
that image, not to a new image incorporating Light Hybrid.

After merging main, the three recorded-prefix regressions, all 206 Light
Hybrid behavioral tests, and 182 mask/observation tests pass. Main-wheel
contract and owned-choice binding checks also pass (32 Python tests), and
all 13 tier-0 checks pass. Game 340's two archived source choices are
translated to schema-2 page actions by their recorded owner, carrier, source
position, and printed card ID. The original action IDs remain in the fixture;
the test requires exactly one matching physical choice. The opt-in frozen
checkpoint search test now requires a compatible current-contract model.
Historical exact-checkpoint search evidence remains tied to the schema-1
revision above.
