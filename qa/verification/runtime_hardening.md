# Verification Ladder Runtime Hardening

## Nextest Adoption

`cargo-nextest` is configured in `.config/nextest.toml` for the heavy engine
binaries:

- `cards_behavioral`
- `dsl`

Both are assigned to the `engine-heavy` test group with one test thread and a
`rust-min-stack` wrapper (`scripts/nextest_rust_min_stack.py`) that sets
`RUST_MIN_STACK=268435456`. `scripts/verify --tier 3` runs the cargo harness
and nextest harness side by side for these binaries. CI installs nextest only
for the scheduled tier-3 seal.

Current local state: `cargo nextest --version` reports that nextest is not
installed in this workstation, so local post-nextest timings are deferred to
the scheduled CI run. The per-set `cards_behavioral` split is not warranted in
this change until the dual-harness CI run produces link/build timing data.

## Local Measurements

- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test judge_quiz`
  completed in 44.89s with 43 passed, 1 ignored.
- `cargo test --manifest-path code/digimon-engine/Cargo.toml --test invariant_fuzz`
  completed in 32.66s with 4 seeded games and 480 legal random steps.

## EX12 Dry-Run Consumer

The intended first consumer is the EX12 Guard/Engage keyword round. A dry run
against the EX12 card path proves the ladder scopes the change without mutating
engine behavior:

```bash
python scripts/verify --tier 2 --path code/digimon-engine/cards/ex12/EX12-018.yaml --dry-run --json
```

Observed tier-2 routing:

- `impact_scope`: card `EX12-018`, `full_suite_required=false`.
- `cards_behavioral_ex12_018`.
- `side_dsl`.
- `replay_goldens_verify`.
- `invariant_fuzz`.

This is the expected shape for a card-local EX12 behavior change: targeted
card behavior, DSL side binary, replay-golden diff, and invariant fuzz.
