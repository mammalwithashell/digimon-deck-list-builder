//! Exact JSON transport for count choices. u64 values are decimal strings,
//! because JavaScript numbers cannot represent all physical identities exactly.
use crate::count_choice::{CountChoiceMetadata, CountChoiceOperation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountChoiceHostDto {
    pub player: crate::enums::PlayerId,
    pub field_index: u8,
    pub battle_identity: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountChoiceOperationDto {
    CommitCount,
    PreviousPage,
    NextPage,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CountChoiceDto {
    pub operation: CountChoiceOperationDto,
    pub declared_count: Option<String>,
    pub payable_count: Option<String>,
    pub page_index: String,
    pub page_count: String,
    pub host: Option<CountChoiceHostDto>,
}
impl From<CountChoiceMetadata> for CountChoiceDto {
    fn from(value: CountChoiceMetadata) -> Self {
        let (operation, declared_count, payable_count) = match value.operation {
            CountChoiceOperation::Commit(v) => (
                CountChoiceOperationDto::CommitCount,
                Some(v.declared.to_string()),
                Some(v.payable.to_string()),
            ),
            CountChoiceOperation::PreviousPage => {
                (CountChoiceOperationDto::PreviousPage, None, None)
            }
            CountChoiceOperation::NextPage => (CountChoiceOperationDto::NextPage, None, None),
        };
        Self {
            operation,
            declared_count,
            payable_count,
            page_index: value.page_index.to_string(),
            page_count: value.page_count.to_string(),
            host: value.host.map(|h| CountChoiceHostDto {
                player: h.player,
                field_index: h.field_index,
                battle_identity: h.battle_identity.to_string(),
            }),
        }
    }
}
