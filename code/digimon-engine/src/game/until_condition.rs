//! Until-condition modifier expiry (Tier 1).

#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

impl Game {
    pub fn until_condition_last_cycle_evaluations(&self) -> usize {
        self.until_condition_last_cycle_evaluations
    }

    pub fn until_condition_reevaluation_cycles(&self) -> u64 {
        self.until_condition_reevaluation_cycles
    }

    pub fn mark_until_condition_dirty(&mut self) {
        self.until_condition_dirty = true;
    }

    pub fn reevaluate_until_condition_modifiers_if_dirty(&mut self) {
        if !self.until_condition_dirty {
            return;
        }
        if self.pending_selection.is_some()
            || !self.effect_queue.is_empty()
            || self.effect_chain_depth != 0
        {
            return;
        }
        self.until_condition_dirty = false;
        self.reevaluate_until_condition_modifiers();
    }

    pub fn reevaluate_until_condition_modifiers(&mut self) {
        let candidates = self.modifiers.until_condition_candidates();
        let mut evaluations = 0usize;
        for (install_order, subject) in candidates {
            let keep = self
                .modifiers
                .evaluate_until_condition(subject, install_order, self);
            let Some(keep) = keep else {
                continue;
            };
            evaluations += 1;
            if !keep {
                self.modifiers
                    .remove_until_condition_by_order(subject, install_order);
            }
        }
        self.until_condition_last_cycle_evaluations = evaluations;
        self.until_condition_total_evaluations = self
            .until_condition_total_evaluations
            .saturating_add(evaluations as u64);
        self.until_condition_reevaluation_cycles =
            self.until_condition_reevaluation_cycles.saturating_add(1);
    }
}
