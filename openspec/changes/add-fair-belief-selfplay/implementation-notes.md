# Implementation and evidence

## Current implementation (2026-10-28 / overnight completion)

Preserved the original foundation as commit `879ce5147`, then merged fetched
remote main `d0d052026` into `feat/fair-belief-selfplay`. The branch began 130
commits behind; all implementation below uses the updated checkout.

The native CLI, both-seat self-play, PIMC and IS-MCTS now support closed-list
hypotheses from restricted viewer evidence. Policy/outcome shapes remain
unchanged. The generation runner rejects regime/prior resume drift; shards,
checkpoints and ONNX sidecars preserve information provenance. Untagged legacy
data remains exact-prior data. No training-completion path promotes a champion.

See `docs/FAIR_BELIEF_TRAINING.md` for API boundaries, conservative knowledge
invalidation, supported continuation forms, explicit unsupported errors,
observation limits and reproduction commands. The 45-trash-identity policy
window remains unchanged; full public counts are available to search.

`experiment/` freezes the repository standard/ENG restriction policy, globally
admitted fallback population, equal distinct-list weights, 20 admitted
ST1-derived variants with 16/2/2 disjoint list splits, seeds 11/22/33, two
32-game search generations per regime/seed, 32 simulations and four worlds,
2048-step fresh exploiters, and 256-game per-seed independent panels.
The `training_games_per_seed` protocol field is the per-generation quota;
two generations therefore request 64 games per training seed/regime.
The seed checkpoint is an untrained compatible MLP initializer; it is evaluated
separately. No compatible trained checkpoint was silently reshaped.
Stock ST1-09 failed the existing admission gate; it was excluded from this
explicitly named cohort. Admission and mechanics gates remain intact.

Completed validation so far: repository tier 0 passed all 13 checks including
183 tooling tests; Python focused suites passed 54 tests with one existing skip.
The Rust validation details follow below; the independent comparison is complete and failed promotion. See `evidence-report.md`.
Development-build preflight attempts are preserved separately from final
comparison evidence. Optimized executable hashes cannot be mixed into those
attempts on resume. No model-strength or promotion claim follows from them.

## Current validation detail

- 46 determinization and 19 search tests passed, including composition-changing
  PIMC/IS-MCTS guards, private inspection transfer, full trash accounting,
  transformed-hidden refusal and a legally resolved sampled card absent from
  the actual list.
- Tier 1's six Rust suites passed 2,077 tests with five existing ignores.
- Extra security-selection suite: two passed; the recorded game-268 prefix
  rejected PASS at decision 79 before reaching search. The independently
  installed pre-change PyO3 binding rejects the same action. This fixture
  failure is retained; it has not been relabelled as a successful aggregate.
- Closed smoke: generations 0 and 1 completed, then a new Python process resumed
  generation 2. Six games total, no truncated rows, SB3 + ONNX exports complete.
  Chunk scoring sidecars recorded genuine fallback worlds; no scorer truth
  enters actors, providers or trainers.
- Prior microbenchmark: 16 candidates, 1066 admitted IDs, 1000 requests each.
  Initial conditioning 0.129 s, mixture sampling 1.685 s (106 fallback worlds).
  Unfamiliar evidence eliminated all candidates: conditioning 0.175 s,
  sampling 14.039 s, all 1000 fallback requests succeeded, zero sample failures.
  Attempt bound 128; this is request success, not per-proposal acceptance.
- First full seed's closed training used a sampled peak process-tree RSS of
  4,592,140,288 bytes at 0.5-second intervals and 92.67 s total wall time.
  These are local measurements for the declared cohort/budget, not scaling
  guarantees. Final per-regime outcomes are recorded in `evidence-report.md`; no candidate qualified.

## Historical foundation evidence (2026-10-22)

## Scope

Worktree baseline: `3b6a2d8f4`, branch `feat/fair-belief-selfplay`.
The first slice adds isolated belief/knowledge primitives and tests. It does
not enable closed-list self-play, materialize hypothetical cards, promote a
model, or provide evidence of improved playing strength.

## Information audit

- `observation/tensor_v2_lite_deck.rs::write_original_decklist` reads only
  `game.player(player_id).original_deck`. The immediate leak being addressed
  is in search supervision, not an opponent-list tensor field.
- `observation/tensor_v2_lite.rs::write_known_zone_cards` allocates 45 trash
  rows per seat. The player summary carries full trash length, but identities
  beyond the row capacity are absent. A policy cannot be assumed to reconstruct
  full long-game copy counts from one such observation. This change does not
  expand the tensor contract.
- Existing `Infoset::extract` and `materialize` use the actual opposing
  hand/deck/security composition. Existing hidden-permutation tests do not
  detect dependence on a different concealed composition.
- `GameEvent` does not currently provide a complete, viewer-entitled stream of
  peeks, hidden transfers, and knowledge invalidations. The new
  `KnowledgeLedger` is an event-input primitive, not an automatically wired
  tracker. Engine event coverage remains an explicit integration task.
- A deck shuffle preserves knowledge that a seen card belongs somewhere in
  that deck, while destroying its position. An unobserved transfer cannot be
  resolved by asking which true physical instance moved; the primitive exposes
  conservative invalidation for affected zones. Stronger lower-bound belief
  tracking remains future lifecycle work.
- Hypothetical-world work must rebuild hidden-dependent prompt indices,
  continuations, original-list metadata and instance IDs. Closed search cannot
  reuse the existing materializer merely by replacing mixture weights.

## Prior primitives

`WeightedDeckPrior` accepts a `BeliefEvidence` value, never a `Game`.
Current public accounting extraction reads public opposing-owned cards across
both players' public zones and hidden zone lengths only; unsupported opaque games and hidden digivolution sources
fail explicitly. Physical instances already public are not counted twice
when a ledger entry is stale.

Snapshots canonicalize candidate order and card order, deduplicate repeated
lists without increasing their weight, reject conflicting duplicate weights,
and fingerprint the admitted population, weights, format, provenance and
compiled legality data. Supplied populations are explicitly admitted inputs:
production preparation still needs an implemented-card intersection, source
snapshot and independent train/validation/test split.

Candidate compatibility checks actual current card counts, known-hidden
membership, zone sizes and egg counts. The unseen component retains positive
mass even with compatible known lists. Fallback completions sample uniformly
over eligible IDs per step and are accepted only by the shared complete-deck
validator. Attempts are bounded; exhaustion is an error, never a request for
the true opposing list. These are legal hypotheses, not claims of calibrated
real-world deck likelihood.

## Validation

Completed 2026-10-22:

- Determinization: **38 passed**, including **20 new belief tests**.
- Existing search regressions: **10 passed** (PIMC, IS-MCTS, masks, profiles,
  deterministic seeds, same-player value signs and hidden-permutation guards).
- Existing self-play shard integration: **1 passed** (two complete starter
  games, existing exact-list mode; this is not closed-list integration).
- Repository verification tier 0: **all 13 checks passed**, including
  **174 verification-tool tests**.
- Both this change and the amended `add-determinized-search` pass strict
  OpenSpec validation. Changed Rust files pass rustfmt; diff whitespace is clean.

The public-owner accounting regression was observed failing before the fix.
The security-pin fixture initially failed because security is dealt after
mulligan; the fixture now advances legal masked mulligan decisions first.
The final aggregate rerun includes both fixes.

Rust verification used an isolated
`CARGO_TARGET_DIR=D:\cargo-target-wt\closed-deck-belief-training`,
`RUST_MIN_STACK=268435456`, normal default features, and per-package
`profile.dev.package.digimon-engine` / `digimon-dsl` overrides
`opt-level=0, debug=0`. These checks verify behavior, not throughput.
The initial `--no-default-features` build failed in pre-existing unconditional
DSL imports; that configuration is not claimed to pass. An initial optimized
build was stopped in this worktree only before switching to the faster
verification profile.

Reproduction: `cargo test -p digimon-engine --test determinization --test search --test selfplay`
with the isolation/stack/profile settings above; `python scripts/verify --tier 0 --continue-on-failure`.
Raw logs are saved under the isolated target directory.

## Remaining integration

**5/29 tasks complete.** The remaining work includes production pool/split and
experiment snapshots, full engine event coverage and knowledge persistence,
hypothetical-world construction and continuation hygiene, both-seat fair search
targets, run/shard/model/resume provenance, and independent policy-only and
exploiter experiments. No model has been retrained or promoted. The full
engine/card suite and cloud training have not been run for this first slice.
