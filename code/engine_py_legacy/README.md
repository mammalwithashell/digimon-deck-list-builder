# `engine_py_legacy/` — Sunset Python Engine

This package is **sunset reference material** for the Rust engine parity effort.

## Rules

- **Do not extend.** No new features, no new card scripts, no new tests.
- **Do not import from production code** unless the call site is tracked in
  `docs/RUST_PYTHON_PARITY.md` § "Phase 3 residue" (these are short-lived
  escape hatches with explicit removal triggers).
- **Tests here run on demand only.** They are excluded from default `pytest`
  collection. Run them explicitly with:
  ```bash
  python -m pytest engine_py_legacy/tests
  ```

## Deletion trigger

Delete this directory when `docs/RUST_PYTHON_PARITY.md` shows zero entries in
the residue table — i.e., when every parity-doc-tracked caller has migrated
to `digimon_engine` (Rust via PyO3) and the divergences table is empty.

## Layout

- `engine/` — verbatim move of the former `digimon_gym/engine/` (minus
  `onnx_policy.py`, which stays at `digimon_gym/engine/onnx_policy.py` until
  Phase 5 relocates it to `digimon_gym/inference/onnx_policy.py`).
- `tests/` — verbatim move of `tests/{engine,behavioral,runners,scenarios,helpers,tools}/`.
