//! Identity aliases — mostly X-Antibody cards that are "treated as" their
//! un-X-Antibody name in certain zones. Spec §3.4.

use serde::{Deserialize, Serialize};

use crate::dsl::predicate::Zone;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub name_aliases: Vec<NameAliasSpec>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameAliasSpec {
    pub treat_as: String,
    pub when: AliasCondition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AliasCondition {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zone: Vec<Zone>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_inherited: Option<InheritedFilter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InheritedFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_number_is: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_is: Option<String>,
}
