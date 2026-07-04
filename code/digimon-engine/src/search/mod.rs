//! Determinized neural MCTS (add-determinized-search Phase 2).
//!
//! PUCT tree search over cloneable `Game` states: node = a decision point
//! (`decision player` + action mask), edge = a masked `action_id`,
//! expansion = clone + `decode_action`, leaf evaluation = a policy+value
//! function over the observation tensor. See the change's `design.md` D3
//! and `specs/neural-mcts-search/spec.md`. The fork substrate is verified
//! by `tests/clone_fuzz/` (per-decision clone faithfulness on real decks).
//!
//! Layout:
//!   - [`evaluator`] — the [`PolicyValueFn`] abstraction (+ the stub
//!     [`UniformEvaluator`]); batch-shaped for Phase 4 leaf batching.
//!   - [`mcts`] — the PUCT search core ([`Mcts`], [`SearchConfig`],
//!     [`SearchResult`]) with identity-keyed value backup and forced-node
//!     auto-advance (search-granularity decision documented there).
//!   - `onnx_adapter` — `PolicyValueFn` for the task-0.3
//!     `inference::BatchedPolicyValueEvaluator` (kept separate so the core
//!     never touches `ort`).
//!   - `determinized` — PIMC / IS-MCTS aggregation over Phase 1's
//!     materialized worlds + the no-X-ray canonicalization (tasks 2.3/2.5).
//!   - `dirichlet` — seeded Dirichlet root noise (Marsaglia–Tsang gamma;
//!     no `rand_distr` dependency).
//!
//! Integration tests: `tests/search/` (perfect-information forced-win,
//! same-player sign handling, determinism, mid-game robustness).

mod determinized;
mod dirichlet;
mod evaluator;
mod mcts;
mod onnx_adapter;

pub use determinized::{determinized_search, DeterminizationMode, DeterminizedConfig};
pub use evaluator::{EvalRequest, Evaluation, PolicyValueFn, UniformEvaluator};
pub use mcts::{decision_player, Mcts, SearchConfig, SearchResult};
