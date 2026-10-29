//! Opt-in viewer memory. No gameplay decision or action mask uses this state.
use super::beliefs::{KnowledgeLedger, KnowledgeZone};
use crate::{card_source::CardSource, Game};

impl Game {
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
