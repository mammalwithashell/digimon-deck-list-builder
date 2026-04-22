//! TODO: populated by Task 5 of the Phase 0 plan (`docs/superpowers/plans/2026-04-21-card-scripting-dsl-phase-0.md`); also extended in Task 9.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClauseSpec {}
