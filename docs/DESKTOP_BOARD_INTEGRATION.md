# Desktop Board Integration

The desktop client uses the Rust engine through Tauri commands in
`code/src-tauri/src/engine_commands.rs`.

The live board renders the In Between design language:

- graphite/obsidian mat
- horizon memory seam
- sharp resource/security chrome
- action trace ticker
- tensor debug badge

Action semantics are not decoded in React. Rust produces `action_traces`
from `digimon_engine::action::explain::explain_action`, and the board renders
those traces. Agent tensor snapshots are summarized in `TensorSummaryDto`;
the raw 1375-float tensor is not sent to the UI by default.

Canonical contracts:

- Action mask size: `2168`
- Board-state tensor size: `1375`
- Rust action decoder: `code/digimon-engine/src/action/decode.rs`
- Rust action explainer: `code/digimon-engine/src/action/explain.rs`
- Rust tensor builder: `code/digimon-engine/src/tensor.rs`
