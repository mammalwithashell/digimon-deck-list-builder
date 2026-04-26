# digimon-engine-py

PyO3 bindings exposing the Rust [`digimon-engine`](../digimon-engine/) crate to Python as the `digimon_engine` module.

## Surface

- `RustHeadlessGame` — Python-facing wrapper around `HeadlessRunner`
- Translates the Python player-ID convention (1/2) ↔ Rust (0/1) at the binding boundary; callers on both sides depend on this — do not change it without auditing both sides

## Build

`maturin` is the build backend; the Python module is installed into the active env:

```bash
cd code/digimon-engine-py && maturin develop
```

For release wheels: `maturin build --release`.

## Consumers

- [`digimon_gym`](../digimon_gym/) — `DigimonEnv` swaps to the Rust backend behind `DIGIMON_BACKEND=rust`
- [`server/`](../server/) — hosted API uses the Rust engine through the same Python binding

## Parity tests

```bash
DIGIMON_BACKEND=rust python -m pytest code/tests/engine/test_rust_backend_parity.py -v
```

Cross-engine divergences are tracked in [`docs/RUST_PYTHON_PARITY.md`](../../docs/RUST_PYTHON_PARITY.md).
