//! Checked event references retain no unavailable concealed handles.
//! Reading, comparing, indexing or mutating an unavailable value aborts the
//! hypothetical request. Cloning carries the obligation with the value.
use crate::card_source::CardHandle;
use std::{collections::HashSet, ops::{Deref, DerefMut}};
use super::{EventSubject, MovedCardSet};

#[derive(Clone, Copy, Default)]
pub struct EventReference<T> {
    value: T,
    unavailable: bool,
}
pub type EventCardReference = EventReference<Option<CardHandle>>;
pub type EventSubjectReference = EventReference<Option<EventSubject>>;
pub type MovedCardSets = EventReference<Vec<MovedCardSet>>;

impl<T> From<T> for EventReference<T> {
    fn from(value: T) -> Self { Self { value, unavailable: false } }
}
impl<T> EventReference<T> {
    fn check(&self) {
        assert!(!self.unavailable, "closed-list unavailable event-card reference read");
    }
    pub(crate) fn is_unavailable(&self) -> bool { self.unavailable }
    /// Audit representation only, never effect execution.
    pub(super) fn value_for_audit(&self) -> &T { &self.value }
}
impl<T: Copy> EventReference<T> {
    pub fn read(&self) -> T { self.check(); self.value }
}
impl<T> Deref for EventReference<T> {
    type Target = T;
    fn deref(&self) -> &T { self.check(); &self.value }
}
impl<T> DerefMut for EventReference<T> {
    fn deref_mut(&mut self) -> &mut T { self.check(); &mut self.value }
}
impl<T: PartialEq> PartialEq for EventReference<T> {
    fn eq(&self, other: &Self) -> bool {
        self.check(); other.check(); self.value == other.value
    }
}
impl<T: Eq> Eq for EventReference<T> {}
impl<T: std::fmt::Debug> std::fmt::Debug for EventReference<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.unavailable { f.write_str("Unavailable") } else { self.value.fmt(f) }
    }
}
impl EventCardReference {
    pub(super) fn retained_handle_for_audit(&self) -> Option<CardHandle> { self.value }
    /// Projection is not, by itself, permission to execute the containing Game.
    pub(crate) fn project_if_unentitled(&mut self, allowed: &HashSet<CardHandle>) -> bool {
        if self.value.is_some_and(|handle| !allowed.contains(&handle)) {
            self.value = None; self.unavailable = true;
        }
        self.unavailable
    }
}
impl EventSubjectReference {
    pub(crate) fn project_if_unentitled(&mut self, allowed: &HashSet<CardHandle>) -> bool {
        if matches!(self.value, Some(EventSubject::Card { card, .. }) if !allowed.contains(&card)) {
            self.value = None; self.unavailable = true;
        }
        self.unavailable
    }
}
impl MovedCardSets {
    pub(crate) fn project_if_unentitled(&mut self, allowed: &HashSet<CardHandle>) -> bool {
        for set in &mut self.value {
            let before = set.cards.len();
            set.cards.retain(|handle| allowed.contains(handle));
            self.unavailable |= set.cards.len() != before;
        }
        self.unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{debug_runner::DebugRunner, trigger_context::TriggerContext};
    use std::panic::{catch_unwind, AssertUnwindSafe};

    #[test]
    fn subject_and_movement_aliases_cannot_expose_a_projected_event_card() {
        use crate::enums::Zone;
        let mut subject = EventSubjectReference::from(Some(EventSubject::Card {
            card: CardHandle(67), zone: Zone::Security,
        }));
        let mut sets = MovedCardSets::from(vec![MovedCardSet {
            cards: vec![CardHandle(67), CardHandle(71)], from: Some(Zone::Security), to: None,
        }]);
        let allowed = HashSet::from([CardHandle(71)]);
        assert!(subject.project_if_unentitled(&allowed));
        assert!(sets.project_if_unentitled(&allowed));
        assert_eq!(subject.value, None);
        assert_eq!(sets.value[0].cards, vec![CardHandle(71)]);
        assert_eq!(sets.value[0].from, Some(Zone::Security));
        assert_eq!(sets.value[0].to, None);
        assert!(catch_unwind(|| subject.read()).is_err());
        assert!(catch_unwind(|| subject.is_some()).is_err());
        assert!(catch_unwind(|| sets.len()).is_err());
        assert!(catch_unwind(|| sets[0].cards.len()).is_err());
        assert!(catch_unwind(|| sets.clone().iter().count()).is_err());
        assert!(catch_unwind(AssertUnwindSafe(|| sets.clear())).is_err());
        assert!(catch_unwind(|| sets == sets.clone()).is_err());
        assert_eq!(format!("{subject:?} {sets:?}"), "Unavailable Unavailable");
        assert!(subject.project_if_unentitled(&allowed));
        assert!(sets.project_if_unentitled(&allowed));
    }

    #[test]
    fn all_three_reference_aliases_are_removed_and_cloned_as_obligations() {
        use crate::enums::Zone;
        let mut context = TriggerContext::default();
        context.event_card = Some(CardHandle(67)).into();
        context.subject = Some(EventSubject::Card { card: CardHandle(67), zone: Zone::Security }).into();
        context.moved_card_sets.push(MovedCardSet { cards: vec![CardHandle(67)], from: Some(Zone::Security), to: None });
        assert!(!context.belief_safe(&HashSet::new()));
        assert!(context.event_card.project_if_unentitled(&HashSet::new()));
        assert!(context.subject.project_if_unentitled(&HashSet::new()));
        assert!(context.moved_card_sets.project_if_unentitled(&HashSet::new()));
        for c in [context.clone(), context] {
            assert!(c.belief_safe(&HashSet::new()));
            assert!(c.belief_missing_handles(&HashSet::new()).is_empty());
            assert!(!format!("{c:?}").contains("CardHandle(67)"));
            assert!(catch_unwind(|| c.event_card.read()).is_err());
            assert!(catch_unwind(|| c.subject.read()).is_err());
            assert!(catch_unwind(|| c.moved_card_sets.as_slice()).is_err());
        }
    }

    #[test]
    fn unavailable_reference_rejects_presence_and_identity_reads_without_retaining_the_card() {
        let mut reference = EventCardReference::from(Some(CardHandle(67)));
        assert!(reference.project_if_unentitled(&HashSet::new()));
        assert_eq!(reference.value, None);
        assert_eq!(format!("{reference:?}"), "Unavailable");
        for other in [EventCardReference::from(None), Some(CardHandle(67)).into(), reference] {
            assert!(catch_unwind(|| reference == other).is_err());
            assert!(catch_unwind(|| other == reference).is_err());
        }
        assert!(catch_unwind(|| reference.read().is_some()).is_err());
        assert!(catch_unwind(|| reference.read().map(|h| h.0)).is_err());
        assert!(reference.project_if_unentitled(&HashSet::from([CardHandle(67)])));
        assert!(catch_unwind(|| reference.read()).is_err());
    }

    #[test]
    fn absent_and_entitled_event_references_keep_their_exact_behavior() {
        for value in [None, Some(CardHandle(0)), Some(CardHandle(67)), Some(CardHandle(u16::MAX))] {
            let mut reference = EventCardReference::from(value);
            let allowed = value.into_iter().collect();
            assert!(!reference.project_if_unentitled(&allowed));
            assert!(!reference.is_unavailable());
            assert_eq!(reference.read(), value);
            assert_eq!(format!("{reference:?}"), format!("{value:?}"));
        }
    }

    #[test]
    fn target_alias_projects_recursively_and_cannot_reveal_or_clear_the_obligation() {
        for seat in [0, 1] { for nested in [false, true] {
            let mut leaf = TriggerContext::default();
            leaf.target_card = Some(CardHandle(67)).into();
            leaf.event_card = Some(CardHandle(71)).into();
            let mut context = if nested {
                let mut middle = TriggerContext::default(); middle.simultaneous_play_members.push(leaf);
                let mut parent = TriggerContext::default(); parent.simultaneous_play_members.push(middle); parent
            } else { leaf };
            assert!(context.project_checked_event_references(&HashSet::from([CardHandle(71)])));
            assert!(context.has_unavailable_event_references());
            assert!(context.belief_safe(&HashSet::from([CardHandle(71)])));
            assert!(context.belief_missing_handles(&HashSet::from([CardHandle(71)])).is_empty());
            let mut runner = DebugRunner::builder().memory(3).start();
            runner.set_first_player(seat); runner.game.current_trigger_context = Some(context.clone());
            let clone = runner.game.clone();
            let saved = clone.current_trigger_context.unwrap();
            for c in [context, saved] {
                assert!(!format!("{c:?}").contains("CardHandle(67)"));
                let mut leaf = if nested { c.simultaneous_play_members[0].simultaneous_play_members[0].clone() } else { c };
                assert_eq!(leaf.event_card.read(), Some(CardHandle(71)));
                assert!(leaf.target_card.is_unavailable());
                assert_eq!(leaf.target_card.retained_handle_for_audit(), None);
                assert!(catch_unwind(|| leaf.target_card.read()).is_err());
                assert!(catch_unwind(|| leaf.target_card.is_some()).is_err());
                assert!(catch_unwind(AssertUnwindSafe(|| leaf.target_card.take())).is_err());
                assert!(catch_unwind(|| leaf.target_card == None.into()).is_err());
                assert!(leaf.project_checked_event_references(&HashSet::from([CardHandle(67), CardHandle(71)])));
                assert!(catch_unwind(|| leaf.target_card.read()).is_err());
            }
        } }
    }

    #[test]
    fn unavailable_reference_survives_context_group_and_game_clones() {
        for seat in [0, 1] {
            let mut runner = DebugRunner::builder().memory(3).start();
            runner.set_first_player(seat);
            let mut child = TriggerContext::default();
            child.event_card = Some(CardHandle(67)).into();
            child.event_card.project_if_unentitled(&HashSet::new());
            let mut parent = TriggerContext::default();
            parent.simultaneous_play_members = vec![child.clone()];
            runner.game.current_trigger_context = Some(parent.clone());
            let clone = runner.game.clone();
            let saved = clone.current_trigger_context.as_ref().unwrap()
                .simultaneous_play_members[0].clone();
            for context in [&child, &saved] {
                assert!(context.event_card.is_unavailable());
                assert!(catch_unwind(AssertUnwindSafe(|| context.event_card.read())).is_err());
                assert!(context.belief_safe(&HashSet::new()));
                assert!(!format!("{context:?}").contains("CardHandle(67)"));
            }
            assert_eq!(parent.event_card.read(), None);
            assert_eq!(runner.game.memory, 3);
        }
    }
}
