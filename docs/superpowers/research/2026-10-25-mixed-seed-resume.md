# Explicit resume of the matched seed comparison

The user authorized resuming after the deleted-token serialization repair.
The existing controller was resumed at 22:40 UTC (5:40 PM Central) on October 25.
All three seed models were retained; bootstrap games and fitting were not rerun.
The completed 5,200-game greedy evaluation was retained after provenance checks.
The failed development window was archived and its full 5,200-game evaluation
restarted. The mixed baseline evaluation and all three search updates follow.

## Runtime and provenance

- Experiment: `/opt/digimon-search/mixed-seed-comparison-20261025`.
- Fixed image: `ghcr.io/mammalwithashell/digimon-search-cpu@sha256:76d7cf1e6fd0c25af41bbb4b0199c5da812eb0bac952d0db3dcbf57072ab8444`.
- Native/runtime source: `7465b31a41340a8b1584356a81a2a1c11fa9a866`.
- Controller source: `2e0ea96ad6178e08c77ac8c753341d75c3cd2ce3`, based on merged PR #748.
- Read-only controller overlay: `/opt/digimon-search/resume-runtime-2e0ea96ad/run_seed_comparison.py`.
- Deployed controller SHA-256: `cef53ffbf0b65ec685b545b87940ff1381de73ecb2640caa9637124d056c40a4`.
- Host backup and deployment record: `host-resumes/1792968004120507672/`.
- Controller resume record, previous state and failed evaluation archive:
  `resume-history/1792968009188853829/`.

Both the original controller hash and its replacement hash are retained in the
host record. The original service definitions, initial experiment record and
seed/model copies are preserved. W&B uses the original three run IDs; monitors
were restarted on the repaired image after their normal exit on failed state.

## Gates and limits

Twenty focused Python tests passed, including no-refit/archive/reuse behavior,
changed input/model/seed rejection, refusal once search has started, unchanged
search settings and the existing evaluation failure-handling tests. Live
preflight verified the actual model files, runtime contract, card database,
deck snapshots, shared anchor and completed greedy report provenance.

The native fix's prior Linux replay gate is recorded in
`2026-10-25-token-choice-image-validation.json`: four full games, each with
telemetry enabled and disabled, identical actions and outcomes, including the
original failed game. This resume does not claim a completed comparison.

Search remains 1,100 games per arm, 320 simulations, four worlds, batch four,
eight workers. Services retain `Restart=no` and a 48-hour limit. No model is
automatically promoted. The new resume command only supports this pre-search
evaluation boundary; a failure after search starts requires separate inspection.
