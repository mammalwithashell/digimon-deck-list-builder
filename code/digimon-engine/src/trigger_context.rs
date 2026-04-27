use crate::card_source::CardHandle;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TriggerContext {
    pub target_permanent: Option<PermanentHandle>,
    pub target_card: Option<CardHandle>,
    pub event_card: Option<CardHandle>,
    pub source_player: Option<PlayerId>,
    pub was_security_skill: bool,
}
