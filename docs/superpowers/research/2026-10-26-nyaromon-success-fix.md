# Nyaromon DNA-success fix

The user authorized correcting EX12-001 after the diagnostic replay exposed
109 attacks in 98 Virus Busters player-games with no DNA by that player.
Training remains stopped. Existing experiment/model artifacts are preserved.

## Correction and evidence

Printed EX12-001 requires the DNA-digivolved Digimon to perform the optional
attack. DCGO's EX12/Yellow/EX12_001.cs schedules it only in the DNA operation's
success callback. The old YAML unconditionally executed the attack step after
the optional DNA operation, including when that operation could not start.

The native partner/result executor now records the existing per-effect
completed-evolution receipt only after successful DNA commitment, scoped to
continuations for the initiating source/controller. The card uses
`effect_digivolved_any_digimon` to gate its attack. Source-carrier identity
already follows the merged Digimon across field compaction; its regression
also clones the game at the pending result prompt. No action/tensor contract,
reward, model parameter, or search-budget changes are involved.

Before implementation, the unavailable-target and invalidated-result regressions
failed by offering an attack. Afterward, all eight card tests pass: missing
target/partner, decline at either prompt, vanished target, prohibited commit,
successful attack, and compaction plus cloning. The focused entry point is
`cargo test -p digimon-engine --test nyaromon_dna_success --features dsl-yaml-loader`.
The same card tests are included in the normal `cards_behavioral` suite.

Additional validation: 25 event-emission tests, four DNA recipe/cost controls,
23 Python diagnostic/scheduling tests, and all 13 Tier-0 stages pass. Impact
and lowering-coverage artifacts were regenerated after adding the card's guard.
Native development checks used the isolated `p3-eval-telemetry-fast` Cargo
target with optimization disabled and `RUST_MIN_STACK=67108864`.

## Fixed-engine evaluation

Source commit: `6627540bab607109b9eb8d5216ff647b8e4b9ae8`.
CI image build: <https://github.com/mammalwithashell/digimon-deck-list-builder/actions/runs/40370197866>.
Release tag: `training-search-v20261026-nyaromon-success`.

The separate evaluation uses the original 6,760-game checkpoint 3/4 schedule,
frozen models and 13 lists, with `comparison_type=rules_fix`. Outcome differences
are expected and retained; this must not be described as an observer-equivalence
replay or pooled with earlier runs as independent evidence. Its publication
gate rejects remaining Nyaromon attack declarations in games where that player
never DNA evolved. This conservative audit complements the focused native tests;
it does not certify every possible timing interaction.

Planned root: `/opt/digimon-search/nyaromon-fixed-eval-20261026/evaluation`.
Planned W&B run: `james-peters97-mammal/digimon-rl/nyaromon-fixed-eval-20261026`.
The deployment verifies the immutable image label and frozen inputs, refuses
overwrites, checks that training services are inactive, and starts only bounded
evaluation/report services. Runtime results are recorded below after completion.
