# src-tauri

Tauri v2 desktop shell. **Python-free at runtime** — gameplay, ONNX inference, and deck tooling all dispatch through Tauri `invoke()` into the embedded [`digimon-engine`](../digimon-engine/) crate (working rule 8).

## Surface

- `src/engine_commands.rs` — `rust_create_game` / step / submit + agent loop
- `src/inference_state.rs` — ONNX session cache keyed by `model_id`
- `src/models.rs` — manifest fetch + SHA-verified download cache
- `src/deck_commands.rs` — parse / validate / tested-cards Tauri wrappers

## Models at runtime

Trained models are **not** bundled with the installer. They are downloaded from the hosted API's `/models/manifest.json` and cached under `dirs::data_dir()/digimon-tcg/models/`. Each artifact is SHA-verified before use.

## Frontend build

The desktop variant tree-shakes admin/training UI via `VITE_BUILD_TARGET=desktop` (working rule 13). See [`frontend/`](../frontend/).

## Commands

```bash
cd code/src-tauri && cargo tauri dev      # development
cd code/src-tauri && cargo tauri build    # production installers
cargo test --manifest-path code/src-tauri/Cargo.toml
```

## Hard rule

The desktop binary must not link any Python runtime. If you find yourself reaching for `pyo3` or shelling out to `python`, the right move is to extend `digimon-engine` instead.
