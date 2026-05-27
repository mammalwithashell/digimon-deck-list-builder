//! Event-emission integration tests — exercises the `GameEvent` variants
//! wired by openspec change `add-reward-profiles` (capability:
//! `engine-event-emission`). Each module covers one variant's scenarios
//! from `openspec/changes/add-reward-profiles/specs/engine-event-emission/spec.md`.

mod attack;
mod digivolve_payload;
mod play_payload;
mod security_reveal;
mod trash;
