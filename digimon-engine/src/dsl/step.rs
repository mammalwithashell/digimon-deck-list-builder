//! TODO: populated by Task 6 of the Phase 0 plan (`docs/superpowers/plans/2026-04-21-card-scripting-dsl-phase-0.md`).

use serde::{Deserialize, Serialize};

/// Step in a `process:` or `extra_cost:` list. Expanded in Task 6 to the
/// full mutation-verb set (§3.7); for now a free-form map so Task 3's
/// burst-digivolve YAML parses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepSpec(pub serde_yml::Value);
