//! Recorder-only public physical-card watches. Never consulted by gameplay.
use crate::{
    card_source::{CardHandle, CardSource},
    Game, PermanentHandle,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCard {
    pub owner: u8,
    pub card_id: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleTrash {
    pub lower_sources: Vec<PublicCard>,
    pub linked: Vec<PublicCard>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementWatch {
    pub version: u32,
    pub deletion_dispatches: u32,
    pub rule_trashes: Vec<RuleTrash>,
}
impl MovementWatch {
    pub fn is_valid_claim(&self) -> bool {
        self.version == 1
            && self.rule_trashes.iter().all(|r| {
                r.lower_sources
                    .iter()
                    .chain(r.linked.iter())
                    .all(|c| c.owner < 2 && !c.card_id.trim().is_empty())
            })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovementAudit {
    pub binding_start: u32,
    pub owner: u8,
    pub card_id: String,
    pub native_top: u64,
    pub native_lower: Vec<u64>,
    pub native_linked: Vec<u64>,
    pub kind: String,
}
#[derive(Debug, Clone)]
struct BoundWatch {
    physical: CardHandle,
    valid: bool,
    value: MovementWatch,
}
#[derive(Debug, Clone, Default)]
pub struct MovementObserver {
    watches: BTreeMap<(u8, String), BoundWatch>,
    audit: Vec<MovementAudit>,
    current_step: u32,
}
#[derive(Debug, Clone)]
pub struct MovementSnapshot {
    pub(crate) top: (CardHandle, PublicCard),
    pub(crate) lower: Vec<(CardHandle, PublicCard)>,
    pub(crate) linked: Vec<(CardHandle, PublicCard)>,
}
impl MovementObserver {
    fn invalidate_physical(&mut self, physical: CardHandle) {
        for watch in self.watches.values_mut() {
            if watch.physical == physical {
                watch.valid = false;
            }
        }
    }
    fn see(&mut self, c: CardHandle, p: PublicCard) {
        if !self.watches.contains_key(&(p.owner, p.card_id.clone())) {
            self.audit.push(MovementAudit {
                binding_start: self.current_step,
                owner: p.owner,
                card_id: p.card_id.clone(),
                native_top: c.0 as u64,
                native_lower: vec![],
                native_linked: vec![],
                kind: "public_watch_bound".into(),
            });
        }
        let w = self
            .watches
            .entry((p.owner, p.card_id))
            .or_insert_with(|| BoundWatch {
                physical: c,
                valid: true,
                value: MovementWatch {
                    version: 1,
                    deletion_dispatches: 0,
                    rule_trashes: vec![],
                },
            });
        if w.physical != c {
            w.valid = false;
        }
    }
    fn event(&mut self, s: &MovementSnapshot, rule: bool, complete: bool) {
        let Some(w) = self
            .watches
            .get_mut(&(s.top.1.owner, s.top.1.card_id.clone()))
        else {
            return;
        };
        if w.physical != s.top.0 || !complete {
            w.valid = false;
            return;
        }
        if !w.valid {
            return;
        }
        self.audit.push(MovementAudit {
            binding_start: self.current_step,
            owner: s.top.1.owner,
            card_id: s.top.1.card_id.clone(),
            native_top: s.top.0 .0 as u64,
            native_lower: s.lower.iter().map(|c| c.0 .0 as u64).collect(),
            native_linked: s.linked.iter().map(|c| c.0 .0 as u64).collect(),
            kind: if rule {
                "rule_trash_completed"
            } else {
                "deletion_dispatch"
            }
            .into(),
        });
        if rule {
            w.value.rule_trashes.push(RuleTrash {
                lower_sources: s.lower.iter().map(|x| x.1.clone()).collect(),
                linked: s.linked.iter().map(|x| x.1.clone()).collect(),
            });
        } else if let Some(n) = w.value.deletion_dispatches.checked_add(1) {
            w.value.deletion_dispatches = n;
        } else {
            w.valid = false;
        }
    }
    pub fn projection(&self, owner: u8) -> BTreeMap<String, MovementWatch> {
        self.watches
            .iter()
            .filter(|((p, _), w)| *p == owner && w.valid)
            .map(|((_, id), w)| (id.clone(), w.value.clone()))
            .collect()
    }
}
impl Game {
    /// Raw recorder audit only; native handles never participate in cross-engine comparison.
    pub fn movement_audit(&self, owner: u8) -> Vec<MovementAudit> {
        self.movement_observer
            .borrow()
            .as_ref()
            .map(|o| {
                o.audit
                    .iter()
                    .filter(|a| a.owner == owner)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    fn observation_card(&self, c: &CardSource) -> Option<(CardHandle, PublicCard)> {
        if c.is_opaque_placeholder || c.face_down {
            return None;
        }
        let id = self.card_data.get(c.data_index)?.card_id.trim();
        if id.is_empty() {
            return None;
        }
        Some((
            c.handle(),
            PublicCard {
                owner: c.owner,
                card_id: id.to_owned(),
            },
        ))
    }
    /// Called only by the harness before decisions. First public sight pins the physical card.
    pub fn observe_public_movement_at(&self, step: u32) -> [BTreeMap<String, MovementWatch>; 2] {
        self.movement_observer
            .borrow_mut()
            .get_or_insert_with(MovementObserver::default)
            .current_step = step;
        self.observe_public_movement()
    }
    pub fn observe_public_movement(&self) -> [BTreeMap<String, MovementWatch>; 2] {
        let cards: Vec<_> = self
            .players
            .iter()
            .flat_map(|p| p.battle_area.iter())
            .flat_map(|p| p.card_sources.iter().chain(p.linked_cards.iter()))
            .filter_map(|c| self.observation_card(c))
            .collect();
        let mut slot = self.movement_observer.borrow_mut();
        let obs = slot.get_or_insert_with(MovementObserver::default);
        for (h, c) in cards {
            obs.see(h, c);
        }
        [obs.projection(0), obs.projection(1)]
    }
    pub(crate) fn movement_snapshot(&self, h: PermanentHandle) -> Option<MovementSnapshot> {
        let p = self
            .players
            .get(h.player as usize)?
            .battle_area
            .get(h.index as usize)?;
        let (top, lower) = p.card_sources.split_last()?;
        let snapshot = (|| Some(MovementSnapshot {
            top: self.observation_card(top)?,
            lower: lower
                .iter()
                .map(|c| self.observation_card(c))
                .collect::<Option<Vec<_>>>()?,
            linked: p
                .linked_cards
                .iter()
                .map(|c| self.observation_card(c))
                .collect::<Option<Vec<_>>>()?,
        }))();
        // An incomplete public stack invalidates only the history of its own
        // physical top. Other watches must not lose unrelated observations.
        if snapshot.is_none() {
            if let Some(observer) = self.movement_observer.borrow_mut().as_mut() {
                observer.invalidate_physical(top.handle());
            }
        }
        snapshot
    }
    pub(crate) fn observe_deletion_dispatch_batch(&self, handles: &[PermanentHandle]) {
        let mut seen = std::collections::HashSet::new();
        for &handle in handles {
            let top = self.players.get(handle.player as usize)
                .and_then(|p| p.battle_area.get(handle.index as usize))
                .and_then(|p| p.card_sources.last()).map(CardSource::handle);
            if top.is_some_and(|card| seen.insert(card)) {
                self.observe_deletion_dispatch(handle);
            }
        }
    }
    pub(crate) fn observe_deletion_dispatch(&self, h: PermanentHandle) {
        if self.movement_observer.borrow().is_none() {
            return;
        }
        let s = self.movement_snapshot(h);
        if let Some(o) = self.movement_observer.borrow_mut().as_mut() {
            if let Some(s) = s {
                o.event(&s, false, true);
            }
        }
    }
    pub(crate) fn observe_rule_trash_completed(&self, s: Option<MovementSnapshot>) {
        let Some(s) = s else {
            return;
        };
        let complete = std::iter::once(&s.top)
            .chain(s.lower.iter())
            .chain(s.linked.iter())
            .all(|(h, c)| {
                self.players
                    .get(c.owner as usize)
                    .is_some_and(|p| p.trash.iter().any(|x| x.handle() == *h))
            });
        if let Some(o) = self.movement_observer.borrow_mut().as_mut() {
            o.event(&s, true, complete);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn c(i: u16, id: &str) -> (CardHandle, PublicCard) {
        (
            CardHandle(i),
            PublicCard {
                owner: 0,
                card_id: id.into(),
            },
        )
    }
    fn snapshot() -> MovementSnapshot {
        MovementSnapshot {
            top: c(1, "X"),
            lower: vec![c(2, "LOW")],
            linked: vec![],
        }
    }
    #[test]
    fn public_watch_dispatch_rule_and_clone() {
        let mut o = MovementObserver::default();
        let s = snapshot();
        o.see(s.top.0, s.top.1.clone());
        let mut clone = o.clone();
        o.event(&s, true, true);
        clone.event(&s, false, true);
        assert_eq!(o.projection(0)["X"].deletion_dispatches, 0);
        assert_eq!(
            o.projection(0)["X"].rule_trashes[0].lower_sources[0].card_id,
            "LOW"
        );
        assert_eq!(clone.projection(0)["X"].deletion_dispatches, 1);
    }
    #[test]
    fn late_event_never_creates_watch() {
        let mut o = MovementObserver::default();
        o.event(&snapshot(), false, true);
        assert!(o.projection(0).is_empty());
    }
    #[test]
    fn different_same_id_never_rebinds() {
        let mut o = MovementObserver::default();
        o.see(CardHandle(1), c(1, "X").1);
        o.see(CardHandle(3), c(3, "X").1);
        assert!(o.projection(0).is_empty());
    }
    #[test]
    fn incomplete_movement_invalidates_watch() {
        let mut o = MovementObserver::default();
        o.see(CardHandle(1), c(1, "X").1);
        o.event(&snapshot(), true, false);
        assert!(o.projection(0).is_empty());
    }
}
