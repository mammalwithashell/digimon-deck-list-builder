//! Opt-in viewer memory. No gameplay decision or action mask uses this state.
use super::beliefs::{KnowledgeLedger, KnowledgeZone};
use crate::{card_source::CardSource, Game};

impl Game {
    /// Only for committed public plays, uses, digivolutions and discards.
    /// The declared identity, not the hidden native slot, determines what
    /// an opponent can rule out among remembered hand memberships.
    pub(crate) fn remember_belief_public_hand_departure(&mut self, owner: u8, index: usize) {
        if self.belief_knowledge.is_none() || index >= self.player(owner).hand.len() {
            return;
        }
        let _ = self.refresh_belief_knowledge();
        let card = &self.player(owner).hand[index];
        let instance = card.card_index;
        let id = card.card_id(&self.card_data).to_string();
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                m.public_hand_departure(owner, instance, &id);
            }
        }
    }
    /// The source is public or an owner-inspectable face-down field card.
    /// Unknown deck/hand/security placements must use anonymous positions.
    pub(crate) fn remember_belief_public_security_placement(
        &mut self,
        card: &CardSource,
        position: crate::enums::StackPosition,
    ) {
        if self.belief_knowledge.is_none() {
            return;
        }
        let pin = match position {
            crate::enums::StackPosition::Top => Some(0),
            crate::enums::StackPosition::Bottom => Some(self.player(card.owner).security.len()),
            crate::enums::StackPosition::Random => {
                self.shuffle_belief_knowledge(card.owner, KnowledgeZone::Security);
                None
            }
        };
        if card.face_down {
            let retained: Vec<_> = self
                .belief_knowledge
                .as_ref()
                .unwrap()
                .iter()
                .map(|m| m.known_identity(card.owner, card.card_index))
                .collect();
            let _ = self.refresh_belief_knowledge();
            for (m, id) in self
                .belief_knowledge
                .as_mut()
                .unwrap()
                .iter_mut()
                .zip(retained)
            {
                if pin == Some(0) {
                    m.insert_known_top(card.owner, KnowledgeZone::Security);
                }
                if let Some(id) = id {
                    if let Err(e) = m.observe(
                        card.owner,
                        card.card_index,
                        &id,
                        KnowledgeZone::Security,
                        pin,
                    ) {
                        self.belief_knowledge_error = Some(e);
                    }
                }
                m.remember_concealed_position(
                    card.owner,
                    card.card_index,
                    KnowledgeZone::Security,
                    pin,
                );
            }
        } else {
            self.learn_belief_card(card, KnowledgeZone::Security, pin);
        }
    }

    pub(crate) fn remember_belief_security_hand_return(&mut self, owner: u8, instance: u16) {
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                m.commit_concealed_hand(owner, instance);
            }
        }
    }
    pub(crate) fn remember_belief_security_departure(
        &mut self,
        owner: u8,
        instance: u16,
        from_top: usize,
    ) {
        if self.belief_knowledge.is_none() {
            return;
        }
        let _ = self.refresh_belief_knowledge();
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                m.conceal_security_departure(owner, instance, from_top);
            }
        }
    }

    pub(crate) fn remember_belief_security_deck_return(
        &mut self,
        owner: u8,
        instance: u16,
        to_bottom: bool,
    ) {
        if self.belief_knowledge.is_none() {
            return;
        }
        let position = if to_bottom {
            self.player(owner).deck.len()
        } else {
            0
        };
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                if !to_bottom {
                    m.insert_known_top(owner, KnowledgeZone::Deck);
                }
                m.commit_concealed_return(owner, instance, position);
            }
        }
    }
    pub(crate) fn remember_belief_token(&mut self, instance: u16) {
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                m.observe_public_token(instance);
            }
        }
    }

    pub fn enable_belief_knowledge(&mut self) -> Result<(), String> {
        if self.players.len() != 2 {
            return Err("belief knowledge requires two players".into());
        }
        if self.belief_knowledge.is_none() {
            // Enabling mid-game cannot recover historical peeks. Callers that
            // require complete history must enable before the first action.
            if self.turn_count > 1
                || self.current_phase != crate::enums::GamePhase::Mulligan
                || self.mulligan_pending.len() != self.players.len()
            {
                return Err("cannot recover legacy knowledge mid-game".into());
            }
            self.belief_knowledge = Some([KnowledgeLedger::new(0)?, KnowledgeLedger::new(1)?]);
        }
        self.refresh_belief_knowledge()
    }

    pub fn belief_knowledge(&self, viewer: u8) -> Result<&KnowledgeLedger, String> {
        if let Some(e) = &self.belief_knowledge_error {
            return Err(e.clone());
        }
        self.belief_knowledge
            .as_ref()
            .and_then(|m| m.get(viewer as usize))
            .ok_or("complete viewer knowledge is unavailable".into())
    }

    pub(crate) fn refresh_belief_knowledge(&mut self) -> Result<(), String> {
        if let Some(mut memory) = self.belief_knowledge.take() {
            let result = memory.iter_mut().try_for_each(|m| m.refresh(self));
            self.belief_knowledge = Some(memory);
            if let Err(e) = &result {
                self.belief_knowledge_error = Some(e.clone());
            }
            result
        } else {
            Ok(())
        }
    }

    pub(crate) fn learn_belief_card(
        &mut self,
        c: &CardSource,
        zone: KnowledgeZone,
        position: Option<usize>,
    ) {
        if self.belief_knowledge.is_some() {
            let _ = self.refresh_belief_knowledge();
        }
        if let Some(memory) = &mut self.belief_knowledge {
            if !c.is_token && !c.is_opaque_placeholder {
                let id = c.card_id(&self.card_data);
                for m in memory {
                    if position == Some(0) {
                        m.insert_known_top(c.owner, zone);
                    }
                    if let Err(e) = m.observe(c.owner, c.card_index, id, zone, position) {
                        self.belief_knowledge_error = Some(e);
                    }
                }
            }
        }
    }

    pub(crate) fn learn_belief_private_card(
        &mut self,
        card: &CardSource,
        viewer: u8,
        zone: KnowledgeZone,
        position: Option<usize>,
    ) {
        if self.belief_knowledge.is_none() {
            return;
        }
        let _ = self.refresh_belief_knowledge();
        if let Some(memory) = &mut self.belief_knowledge {
            // Inserting at the top is public; the inserted identity is private.
            if position == Some(0) {
                for m in memory.iter_mut() {
                    m.insert_known_top(card.owner, zone);
                }
            }
            if !card.is_token && !card.is_opaque_placeholder {
                if let Some(m) = memory.get_mut(viewer as usize) {
                    if let Err(e) = m.observe(
                        card.owner,
                        card.card_index,
                        card.card_id(&self.card_data),
                        zone,
                        position,
                    ) {
                        self.belief_knowledge_error = Some(e);
                    }
                }
            }
        }
    }

    pub(crate) fn transfer_belief_position(
        &mut self,
        owner: u8,
        from: KnowledgeZone,
        position: usize,
        to: KnowledgeZone,
    ) {
        if self.belief_knowledge.is_some() {
            let _ = self.refresh_belief_knowledge();
        }
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                m.transfer_position(owner, from, position, to);
            }
        }
    }

    /// Called only at a committed recovery, after replacement/cancellation.
    /// Track the publicly identified top position, never inspect its identity.
    pub(crate) fn remember_belief_recovery(
        &mut self,
        owner: u8,
        position: crate::enums::StackPosition,
    ) {
        if self.belief_knowledge.is_none() || self.player(owner).deck.is_empty() {
            return;
        }
        let _ = self.refresh_belief_knowledge();
        let security_count = self.player(owner).security.len();
        let instance = self.player(owner).deck.last().unwrap().card_index;
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                let pin = match position {
                    crate::enums::StackPosition::Top => {
                        m.insert_known_top(owner, KnowledgeZone::Security);
                        Some(0)
                    }
                    crate::enums::StackPosition::Bottom => Some(security_count),
                    crate::enums::StackPosition::Random => {
                        m.shuffle(owner, KnowledgeZone::Security);
                        None
                    }
                };
                if let Some(instance) =
                    m.transfer_position(owner, KnowledgeZone::Deck, 0, KnowledgeZone::Security)
                {
                    if let Err(e) = m.move_known(owner, instance, KnowledgeZone::Security, pin) {
                        self.belief_knowledge_error = Some(e);
                    }
                }
                m.remember_concealed_position(owner, instance, KnowledgeZone::Security, pin);
            }
        }
    }

    pub(crate) fn shuffle_belief_knowledge(&mut self, owner: u8, zone: KnowledgeZone) {
        if self.belief_knowledge.is_some() {
            let _ = self.refresh_belief_knowledge();
        }
        if let Some(memory) = &mut self.belief_knowledge {
            for m in memory {
                m.shuffle(owner, zone);
            }
        }
    }
}
