//! AlphaZero self-play data generation (add-determinized-search Phase 3,
//! task 3.1; design D9/D10).
//!
//! The generation-loop boundary is FILES: this module plays search-driven
//! self-play games (both seats searching their own infosets via
//! [`crate::search::determinized_search`]) and writes `(obs, π, z)` shards
//! as `.npy` arrays + a `manifest.json`; the Python trainer
//! (`code/digimon_gym/agents/`) consumes them and re-exports ONNX for the
//! next generation. No PyO3 search surface exists by design.
//!
//! Layout:
//!   - [`driver`] — [`SelfPlayConfig`] + [`run_self_play`]: the game loop,
//!     temperature schedule, z backfill, shard flushing.
//!   - [`npy`] — the minimal `.npy` v1.0 writer/reader the shards use.
//!
//! The CLI front-end is `digimon-engine-cli selfplay`.

pub mod driver;
pub mod npy;

pub use driver::{run_self_play, SelfPlayConfig, SelfPlayStats};
