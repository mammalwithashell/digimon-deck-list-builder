/// Library target so that integration tests under `tests/` can reach
/// the internal modules without depending on Tauri globals.
///
/// `main.rs` re-uses these modules via `pub use` — there is no logic
/// duplication, just module visibility plumbing.
pub mod deck_commands;
pub mod deck_storage;
#[cfg(feature = "debug-bridge")]
pub mod debug_bridge;
pub mod engine_commands;
pub mod format_commands;
pub mod inference_state;
pub mod models;
pub mod updater;
