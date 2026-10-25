//! Owned paging kernel for the coordinated source/security choice migration.
//!
//! This module is deliberately not installed by legacy selectors. A game owner
//! supplies immutable physical pins as `T`, revalidates them before committing,
//! and allocates prompt IDs. Presentation positions never identify a card.
use std::collections::HashSet;
use std::hash::Hash;

pub const CARD_CHOICES_PER_PAGE: usize = 28;
pub const CARD_CHOICE_START: u16 = 30;
pub const CARD_CHOICE_PREVIOUS: u16 = 58;
pub const CARD_CHOICE_NEXT: u16 = 59;
pub const CARD_CHOICE_DECLINE: u16 = 62;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceChoiceArea {
    Battle,
    Breeding,
}

/// Internal physical identity. Neither the source position nor the carrier's
/// current field slot participates in identity. Do not serialize CardHandle as
/// a policy card-ID feature: printed identity is separately visibility-filtered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceChoicePin {
    owner: crate::enums::PlayerId,
    area: SourceChoiceArea,
    carrier_identity: u64,
    card: crate::card_source::CardHandle,
}

impl SourceChoicePin {
    pub fn capture(
        game: &mut crate::game::Game,
        host: crate::permanent::PermanentHandle,
        card: crate::card_source::CardHandle,
    ) -> Option<Self> {
        if host.player > 1 {
            return None;
        }
        let stack = crate::effect_context::selections::material_zone_slice(game, host)?;
        if !stack
            .iter()
            .take(stack.len().saturating_sub(1))
            .any(|source| source.handle() == card)
        {
            return None;
        }
        let area = if host.index == crate::action::space::BREEDING_TARGET as u8 {
            SourceChoiceArea::Breeding
        } else {
            SourceChoiceArea::Battle
        };
        let carrier_identity = game.capture_source_play_host_identity(host)?;
        Some(Self {
            owner: host.player,
            area,
            carrier_identity,
            card,
        })
    }

    pub fn owner(&self) -> crate::enums::PlayerId {
        self.owner
    }
    pub fn area(&self) -> SourceChoiceArea {
        self.area
    }
    pub fn carrier_identity(&self) -> u64 {
        self.carrier_identity
    }
    pub fn card(&self) -> crate::card_source::CardHandle {
        self.card
    }

    /// Resolve a selected prefix to its current slot without changing identity.
    pub(crate) fn resolve_ref(&self, game: &crate::game::Game) -> Option<crate::selection::SourceSelectionRef> {
        let (permanent, source_index) = self.resolve(game)?;
        Some(crate::selection::SourceSelectionRef {
            permanent, field_index: permanent.index, source_index, card: self.card,
        })
    }

    /// Re-find both carrier and physical source. Compaction is allowed; another
    /// card taking an old slot, a different carrier or the active top is not.
    pub fn resolve(
        &self,
        game: &crate::game::Game,
    ) -> Option<(crate::permanent::PermanentHandle, usize)> {
        let zone_hint = crate::permanent::PermanentHandle {
            player: self.owner,
            index: match self.area {
                SourceChoiceArea::Battle => 0,
                SourceChoiceArea::Breeding => crate::action::space::BREEDING_TARGET as u8,
            },
        };
        let host = game.resolve_source_play_host_identity(zone_hint, self.carrier_identity)?;
        let stack = crate::effect_context::selections::material_zone_slice(game, host)?;
        stack
            .iter()
            .take(stack.len().saturating_sub(1))
            .position(|source| source.handle() == self.card)
            .map(|source_index| (host, source_index))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CardChoiceToken {
    pub prompt_id: u64,
    pub page_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardChoiceError {
    EmptyCandidates,
    DuplicateCandidate,
    StaleToken,
    InvalidAction,
    GenerationExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardChoiceResult<T> {
    Selected(T),
    Navigated,
    Declined,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardChoiceRow<T> {
    pub action_id: u16,
    pub pin: T,
}

/// Owns the complete, already-filtered candidate vector. The operation owner
/// decides legality and min-prefix feasibility; this kernel never infers rules
/// from metadata. No callback, RNG or game reference is held in this state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardChoicePage<T> {
    candidates: Vec<T>,
    page_index: usize,
    token: CardChoiceToken,
    allow_decline: bool,
}

impl<T: Clone + Eq + Hash> CardChoicePage<T> {
    pub fn new(
        prompt_id: u64,
        candidates: Vec<T>,
        allow_decline: bool,
    ) -> Result<Self, CardChoiceError> {
        if candidates.is_empty() {
            return Err(CardChoiceError::EmptyCandidates);
        }
        let mut seen = HashSet::with_capacity(candidates.len());
        if candidates.iter().any(|pin| !seen.insert(pin)) {
            return Err(CardChoiceError::DuplicateCandidate);
        }
        Ok(Self {
            candidates,
            page_index: 0,
            token: CardChoiceToken {
                prompt_id,
                page_generation: 0,
            },
            allow_decline,
        })
    }

    pub fn token(&self) -> CardChoiceToken {
        self.token
    }
    pub fn page_index(&self) -> usize {
        self.page_index
    }
    pub fn page_count(&self) -> usize {
        self.candidates.len().div_ceil(CARD_CHOICES_PER_PAGE)
    }
    pub fn candidates(&self) -> &[T] {
        &self.candidates
    }
    pub fn allows_decline(&self) -> bool { self.allow_decline }

    /// Rebuild presentation after live eligibility/order changed. No game work.
    pub fn refresh(&mut self, candidates: Vec<T>) -> Result<(), CardChoiceError> {
        let generation = self.token.page_generation.checked_add(1)
            .ok_or(CardChoiceError::GenerationExhausted)?;
        let mut rebuilt = Self::new(self.token.prompt_id, candidates, self.allow_decline)?;
        rebuilt.token.page_generation = generation;
        *self = rebuilt;
        Ok(())
    }

    pub fn rows(&self) -> Vec<CardChoiceRow<T>> {
        let start = self.page_index * CARD_CHOICES_PER_PAGE;
        let end = self
            .candidates
            .len()
            .min(start.saturating_add(CARD_CHOICES_PER_PAGE));
        self.candidates[start..end]
            .iter()
            .enumerate()
            .map(|(slot, pin)| CardChoiceRow {
                action_id: CARD_CHOICE_START + slot as u16,
                pin: pin.clone(),
            })
            .collect()
    }

    pub fn valid_action_ids(&self) -> Vec<u16> {
        let mut ids: Vec<_> = self.rows().iter().map(|row| row.action_id).collect();
        if self.page_index > 0 {
            ids.push(CARD_CHOICE_PREVIOUS);
        }
        if self.page_index + 1 < self.page_count() {
            ids.push(CARD_CHOICE_NEXT);
        }
        if self.allow_decline {
            ids.push(CARD_CHOICE_DECLINE);
        }
        ids
    }

    /// Reject stale transport tokens before interpreting phase-aware aliases.
    /// Selection returns a pin without moving a card or consuming the prompt.
    /// The operation owner must validate the live carrier/zone before payment.
    pub fn resolve(
        &mut self,
        expected: CardChoiceToken,
        action_id: u16,
    ) -> Result<CardChoiceResult<T>, CardChoiceError> {
        if expected != self.token {
            return Err(CardChoiceError::StaleToken);
        }
        if let Some(row) = self
            .rows()
            .into_iter()
            .find(|row| row.action_id == action_id)
        {
            return Ok(CardChoiceResult::Selected(row.pin));
        }
        let next_page = match action_id {
            CARD_CHOICE_PREVIOUS if self.page_index > 0 => self.page_index - 1,
            CARD_CHOICE_NEXT if self.page_index + 1 < self.page_count() => self.page_index + 1,
            CARD_CHOICE_DECLINE if self.allow_decline => return Ok(CardChoiceResult::Declined),
            _ => return Err(CardChoiceError::InvalidAction),
        };
        let next_generation = self
            .token
            .page_generation
            .checked_add(1)
            .ok_or(CardChoiceError::GenerationExhausted)?;
        self.page_index = next_page;
        self.token.page_generation = next_generation;
        Ok(CardChoiceResult::Navigated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_overflow_fails_without_mutating_owner() {
        let mut page = CardChoicePage::new(1, (0usize..29).collect(), false).unwrap();
        page.token.page_generation = u64::MAX;
        let snapshot = page.clone();
        assert_eq!(
            page.resolve(page.token(), CARD_CHOICE_NEXT),
            Err(CardChoiceError::GenerationExhausted)
        );
        assert_eq!(page, snapshot);
    }
}
