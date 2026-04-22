//! TODO: populated by Task 4 of the Phase 0 plan (`docs/superpowers/plans/2026-04-21-card-scripting-dsl-phase-0.md`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySpec {}
