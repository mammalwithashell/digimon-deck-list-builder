//! Shared types used across multiple DSL submodules.

use serde::{Deserialize, Serialize};

/// Player reference used by both predicate and step modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlayerRef {
    You,
    Opponent,
    Any,
    Active,
}
