//! Phase 2 search-core integration tests (add-determinized-search 2.1/2.2/2.4).
//!
//! Validates the PUCT-MCTS core on perfect-information (pre-determinized)
//! positions per `specs/neural-mcts-search/spec.md`:
//!   - masked-only expansion (every visited root action was legal),
//!   - the search finds a known forced win with a stub `UniformEvaluator`,
//!   - Q-backup does NOT flip sign across consecutive decisions by the SAME
//!     player (sign is keyed to player identity, not tree depth),
//!   - determinism per seed,
//!   - completes within budget on messy mid-game states from real decks,
//!   - determinized aggregation (PIMC / IS-MCTS, task 2.3) and the
//!     no-X-ray guard (task 2.5) in `determinized.rs`.
//!
//! Registry construction recurses deeply — every test body runs on a
//! 256 MB-stack thread (same mitigation as `tests/clone_fuzz/main.rs`).
//! Run: RUST_MIN_STACK=268435456 cargo test --test search

mod determinized;
mod midgame;
mod perfect_information;
mod profile;
mod support;
