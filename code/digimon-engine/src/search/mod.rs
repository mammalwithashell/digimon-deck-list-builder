//! Determinized neural MCTS (add-determinized-search Phase 2).
//!
//! PUCT tree search over cloneable `Game` states: node = a decision point
//! (`decision player` + action mask), edge = a masked `action_id`,
//! expansion = clone + `decode_action`, leaf evaluation = a policy+value
//! function over the observation tensor. See the change's `design.md` D3
//! and `specs/neural-mcts-search/spec.md`. The fork substrate is verified
//! by `tests/clone_fuzz/` (per-decision clone faithfulness on real decks).
