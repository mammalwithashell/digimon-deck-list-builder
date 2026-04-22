//! Top-level DSL card-specification types (spec §3.2).

use serde::{Deserialize, Serialize};

/// A complete card definition as authored in YAML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardSpec {
    /// Primary key — must match `cards.json` `card_id`.
    pub card: String,
    /// Authored name — cross-checked against `cards.json` `card_name_eng`.
    pub name: String,
    /// Card kind.
    pub kind: CardKind,
    /// Level — required for digimon + digi_egg; absent for tamer / option / token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    /// Printed colors (may contain >1 for multi-color cards).
    pub color: Vec<ColorSpec>,
    /// Printed play cost. Absent for digi_egg / token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
    /// Printed DP (digimon only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dp: Option<i32>,
    /// Traits from `type_eng`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub traits: Vec<String>,
    /// Form from `form_eng`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub form: Option<String>,
    /// Attribute from `attribute_eng`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// Ace `<-N>` — negative integer; lowered to on-leave-field hook.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ace_overflow: Option<i32>,
    /// Identity section (§3.4) — name aliases, mostly X-Antibody.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<crate::dsl::identity::IdentitySpec>,
    /// Alternate entry paths — digivolve / DNA / DigiXros / etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alt_paths: Vec<crate::dsl::alt_path::AltPathSpec>,
    /// Triggered + declarative clauses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<crate::dsl::clause::ClauseSpec>,
    /// DSL file-format version; reserved for §9 open question #7.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_version: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Digimon,
    Tamer,
    Option,
    DigiEgg,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSpec {
    Red,
    Blue,
    Yellow,
    Green,
    Black,
    Purple,
    White,
}
