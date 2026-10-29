# EX13 timing audit evidence, October 26, 2026

This bundle preserves the evidence behind the
[timing repairs and pool refresh](../../../docs/superpowers/research/2026-10-26-timing-repairs-and-ex13-pool.md).
It contains exact test scopes, source hashes, historical and refreshed timing
inventories, independent review results and explicit coverage limits.

- `native-validation-results.json` and `native-validation.md`: final-source
  versus earlier integration results, ignored tests, source hashes and build
  settings. The source commit names the base checkout; the hashed sources were
  uncommitted when these tests ran. Original Windows source-byte hashes are
  retained alongside LF-normalized hashes for verification across Git checkouts.
- `dsl-focused-scope.json` and `cards-behavioral-scope.json`: exact selected test
  names, suitable as filters for the respective native test binaries.
- `timing-audit.md`, `timing-coverage-summary.json` and both timing inventories:
  frozen 280-card and refreshed 311-card denominators, authored clauses, input
  hashes and remaining unmeasured interactions. Inventories are not behavioral
  certification.
- `end-of-attack-scope-audit.json` and `end-of-attack-fix-results.json`: printed
  timing review and focused RED/GREEN evidence, including fixture corrections.
- `dsl-broad-interruption.json`: the incomplete broad DSL run, explicitly
  excluded from completed-suite totals.
- `repro-pool-comparison.json`: offline pool reproduction using recorded source
  data and baseline, with equivalent list contents and unchanged consumer files.

Raw logs named in these historical receipts remain in the local
`runs/timing-pool-refresh-20261026/` directory. Absolute build/runtime paths are
recorded observations from the Windows validation host, not portable dependencies.
No model weights, replay tensors, credentials or cloud log files are included.
The frozen pool and evidence JSON files have byte-preserving Git attributes so
checkout line-ending conversion cannot invalidate their recorded hashes.

The pool's exact prior-training exclusion baseline is available as
[`source_baseline_train_snapshot.json`](../../../data/training_pools/ex13-20261026/source_baseline_train_snapshot.json).
It is a byte-identical copy of the baseline named by the preparation receipt.
Supply that file with `--baseline` to reproduce selection from a fresh checkout;
admission still uses the checkout's current gates and native/Python registries.
Historical behavior audits additionally require the evaluated run's retained
reports or models as documented by their CLI inputs.
