use crate::card_source::CardHandle;
use crate::enums::{CardKind, PlayerId, Zone};
use crate::permanent::PermanentHandle;

/// Stable token for effect-created objects or effect-initiated transitions.
/// Downstream cleanup/suppression should key off this token rather than a
/// battle-area index that can shift as permanents leave play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ProvenanceToken(pub u64);

impl From<CardHandle> for ProvenanceToken {
    fn from(value: CardHandle) -> Self {
        Self(value.0 as u64)
    }
}

/// Broad cause categories carried by observer payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventCause {
    BattleDeletion,
    EffectDeletion,
    OwnEffect,
    OpponentEffect,
    Overclock,
    Return,
    DeckBottom,
    SecurityPlacement,
    SecurityRemoval,
    Cost,
    Rule,
}

impl From<crate::replacement::ReplacementCause> for EventCause {
    fn from(value: crate::replacement::ReplacementCause) -> Self {
        match value {
            crate::replacement::ReplacementCause::Battle => EventCause::BattleDeletion,
            crate::replacement::ReplacementCause::OwnEffect => EventCause::OwnEffect,
            crate::replacement::ReplacementCause::OpponentEffect => EventCause::OpponentEffect,
            crate::replacement::ReplacementCause::SecurityCheck => EventCause::SecurityRemoval,
            crate::replacement::ReplacementCause::Cost => EventCause::Cost,
            crate::replacement::ReplacementCause::Overclock => EventCause::Overclock,
        }
    }
}

/// The object that caused the event. This is intentionally distinct from the
/// observer card/permanent carrying the triggered effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSubject {
    Permanent(PermanentHandle),
    Card { card: CardHandle, zone: Zone },
    Player(PlayerId),
}

/// Effect/source attribution for an event. `source_permanent` is optional
/// because hand, trash, security, and rule effects may not have a live
/// permanent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectAttribution {
    pub controller: PlayerId,
    pub source_card: Option<CardHandle>,
    pub source_permanent: Option<PermanentHandle>,
}

/// A selected result made by an earlier pending-selection step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResultBinding {
    pub name: &'static str,
    pub permanent: Option<PermanentHandle>,
    pub card: Option<CardHandle>,
}

/// A compact moved-card batch descriptor for observer predicates that care
/// which cards moved together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovedCardSet {
    pub cards: Vec<CardHandle>,
    pub from: Option<Zone>,
    pub to: Option<Zone>,
}

/// Snapshot captured before a permanent leaves the board. Observers must use
/// this for deletion predicates after the live battle-area slot is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedObjectSnapshot {
    pub former_controller: PlayerId,
    pub top_card: CardHandle,
    pub card_kind: CardKind,
    pub traits: Vec<String>,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub cause: EventCause,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TriggerContext {
    pub subject: Option<EventSubject>,
    pub target_permanent: Option<PermanentHandle>,
    pub target_card: Option<CardHandle>,
    pub event_permanent: Option<PermanentHandle>,
    pub event_card: Option<CardHandle>,
    pub event_source_card: Option<CardHandle>,
    pub event_host_card: Option<CardHandle>,
    pub event_host_permanent: Option<PermanentHandle>,
    pub affected_player: Option<PlayerId>,
    pub source_player: Option<PlayerId>,
    pub cause: Option<EventCause>,
    pub source_effect: Option<EffectAttribution>,
    pub selected_results: Vec<ResultBinding>,
    pub moved_card_sets: Vec<MovedCardSet>,
    pub effect_initiated: bool,
    pub dna_origin: bool,
    pub deleted_object: Option<DeletedObjectSnapshot>,
    pub old_attack_target: Option<crate::selection::AttackTarget>,
    pub new_attack_target: Option<crate::selection::AttackTarget>,
    pub provenance_token: Option<ProvenanceToken>,
    pub was_security_skill: bool,
}
