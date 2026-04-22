//! TODO: populated by Task 7 of the Phase 0 plan (`docs/superpowers/plans/2026-04-21-card-scripting-dsl-phase-0.md`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredicateSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_is: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level_eq: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<crate::dsl::spec::CardKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trait_has: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub any_of: Vec<PredicateSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zone {
    Hand,
    Deck,
    Trash,
    BattleArea,
    Security,
    Breeding,
    Reveal,
    DigiEggDeck,
    Material,
}
