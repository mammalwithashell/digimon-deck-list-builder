//! Named-binding environment for DSL process steps.

use std::collections::HashMap;

use crate::card_source::CardHandle;
use crate::enums::PlayerId;
use crate::permanent::PermanentHandle;
use crate::selection::{BreedingPermanentSelectionRef, SourceSelectionRef, UnionZoneOrigin};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingValue {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(PlayerId, u16),
    TrashIndex(PlayerId, u16),
    Literal(i64),
    PermanentList(Vec<PermanentHandle>),
    CardList(Vec<CardHandle>),
    SourceRefs(Vec<SourceSelectionRef>),
    BreedingPermanentRef(BreedingPermanentSelectionRef),
    /// A card picked via `select_union_zone` (hand ∪ trash), tagged with the
    /// `origin` zone it was in and the `owner` of that zone. Carrying the
    /// origin lets a downstream consumer (`play_union_bound_free`) replay
    /// the card from its *true* zone — a bare `CardHandle` cannot.
    UnionCard {
        card: CardHandle,
        origin: UnionZoneOrigin,
        owner: PlayerId,
    },
}

#[derive(Debug, Default, Clone)]
pub struct Bindings {
    slots: HashMap<String, BindingValue>,
    result_log: EffectResultLog,
}

#[derive(Debug, Default, Clone)]
pub struct EffectResultLog {
    pub suspended: Vec<PermanentHandle>,
    pub returned_to_deck: Vec<CardHandle>,
    pub deleted: Vec<PermanentHandle>,
    pub played: Vec<PermanentHandle>,
    pub digivolved: Vec<PermanentHandle>,
    pub added_to_hand: Vec<CardHandle>,
}

impl Bindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: BindingValue) {
        self.slots.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<BindingValue> {
        self.get_ref(name).cloned()
    }

    /// Borrowing variant of `get`. Prefer this when the caller only needs to
    /// inspect the value — avoids cloning the inner `Vec` for `PermanentList`
    /// / `CardList`. `get` remains the convenience method for callers that
    /// need to take ownership (e.g. `ResolvedBinding::PermanentList(v)`
    /// destructuring in `binding_ref::resolve_named`).
    pub fn get_ref(&self, name: &str) -> Option<&BindingValue> {
        self.slots.get(name)
    }

    pub fn result_log(&self) -> &EffectResultLog {
        &self.result_log
    }

    pub fn record_suspended(&mut self, handle: PermanentHandle) {
        self.result_log.suspended.push(handle);
    }

    pub fn record_returned_to_deck(&mut self, card: CardHandle) {
        self.result_log.returned_to_deck.push(card);
    }

    pub fn record_deleted(&mut self, handle: PermanentHandle) {
        self.result_log.deleted.push(handle);
    }

    pub fn record_played(&mut self, handle: PermanentHandle) {
        self.result_log.played.push(handle);
    }

    pub fn record_digivolved(&mut self, handle: PermanentHandle) {
        self.result_log.digivolved.push(handle);
    }

    pub fn record_added_to_hand(&mut self, card: CardHandle) {
        self.result_log.added_to_hand.push(card);
    }

    pub fn get_permanent(&self, name: &str) -> Option<PermanentHandle> {
        match self.get(name)? {
            BindingValue::Permanent(h) => Some(h),
            _ => None,
        }
    }

    pub fn get_card(&self, name: &str) -> Option<CardHandle> {
        match self.get(name)? {
            BindingValue::Card(h) => Some(h),
            _ => None,
        }
    }

    pub fn get_hand_index(&self, name: &str) -> Option<u16> {
        match self.get(name)? {
            BindingValue::HandIndex(_, i) => Some(i),
            _ => None,
        }
    }

    pub fn get_hand_index_with_player(&self, name: &str) -> Option<(PlayerId, u16)> {
        match self.get(name)? {
            BindingValue::HandIndex(p, i) => Some((p, i)),
            _ => None,
        }
    }

    pub fn get_trash_index(&self, name: &str) -> Option<u16> {
        match self.get(name)? {
            BindingValue::TrashIndex(_, i) => Some(i),
            _ => None,
        }
    }

    pub fn get_trash_index_with_player(&self, name: &str) -> Option<(PlayerId, u16)> {
        match self.get(name)? {
            BindingValue::TrashIndex(p, i) => Some((p, i)),
            _ => None,
        }
    }

    pub fn get_literal(&self, name: &str) -> Option<i64> {
        match self.get(name)? {
            BindingValue::Literal(v) => Some(v),
            _ => None,
        }
    }

    pub fn insert_permanent(&mut self, name: &str, h: PermanentHandle) {
        self.insert(name, BindingValue::Permanent(h));
    }

    pub fn insert_card(&mut self, name: &str, h: CardHandle) {
        self.insert(name, BindingValue::Card(h));
    }

    /// Record a union-zone (hand ∪ trash) pick together with its origin zone
    /// and owner. Used by `select_union_zone`'s success callback so a later
    /// `play_union_bound_free` step can replay the card from its true zone.
    pub fn insert_union_card(
        &mut self,
        name: &str,
        card: CardHandle,
        origin: UnionZoneOrigin,
        owner: PlayerId,
    ) {
        self.insert(name, BindingValue::UnionCard { card, origin, owner });
    }

    /// Fetch a union-zone pick `(card, origin, owner)`. Returns `None` if the
    /// binding is absent or holds a different `BindingValue` kind.
    pub fn get_union_card(&self, name: &str) -> Option<(CardHandle, UnionZoneOrigin, PlayerId)> {
        match self.get(name)? {
            BindingValue::UnionCard { card, origin, owner } => Some((card, origin, owner)),
            _ => None,
        }
    }

    pub fn insert_hand_index(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert(name, BindingValue::HandIndex(player, i));
    }

    pub fn insert_hand_index_for(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert_hand_index(name, player, i);
    }

    pub fn insert_trash_index(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert(name, BindingValue::TrashIndex(player, i));
    }

    pub fn insert_trash_index_for(&mut self, name: &str, player: PlayerId, i: u16) {
        self.insert_trash_index(name, player, i);
    }

    pub fn insert_literal(&mut self, name: &str, v: i64) {
        self.insert(name, BindingValue::Literal(v));
    }

    pub fn insert_permanent_list(&mut self, name: &str, list: Vec<PermanentHandle>) {
        self.insert(name, BindingValue::PermanentList(list));
    }

    pub fn insert_card_list(&mut self, name: &str, list: Vec<CardHandle>) {
        self.insert(name, BindingValue::CardList(list));
    }

    pub fn insert_source_refs(&mut self, name: &str, refs: Vec<SourceSelectionRef>) {
        self.insert(name, BindingValue::SourceRefs(refs));
    }

    pub fn insert_breeding_permanent_ref(
        &mut self,
        name: &str,
        breeding_ref: BreedingPermanentSelectionRef,
    ) {
        self.insert(name, BindingValue::BreedingPermanentRef(breeding_ref));
    }

    pub fn get_permanent_list(&self, name: &str) -> Option<Vec<PermanentHandle>> {
        match self.get(name)? {
            BindingValue::PermanentList(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_card_list(&self, name: &str) -> Option<Vec<CardHandle>> {
        match self.get(name)? {
            BindingValue::CardList(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_source_refs(&self, name: &str) -> Option<Vec<SourceSelectionRef>> {
        match self.get(name)? {
            BindingValue::SourceRefs(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_breeding_permanent_ref(&self, name: &str) -> Option<BreedingPermanentSelectionRef> {
        match self.get(name)? {
            BindingValue::BreedingPermanentRef(v) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_preserves_slots() {
        let mut original = Bindings::new();
        original.insert_hand_index("card_a", 0, 3);
        original.insert_trash_index("card_b", 1, 7);
        original.insert("lit", BindingValue::Literal(42));

        let cloned = original.clone();

        assert_eq!(cloned.get_hand_index("card_a"), Some(3));
        assert_eq!(cloned.get_hand_index_with_player("card_a"), Some((0, 3)));
        assert_eq!(cloned.get_trash_index("card_b"), Some(7));
        assert_eq!(cloned.get_trash_index_with_player("card_b"), Some((1, 7)));
        assert_eq!(cloned.get_literal("lit"), Some(42));
        // Mutating original does not affect clone
        let mut original = original;
        original.insert_hand_index("card_a", 0, 99);
        assert_eq!(cloned.get_hand_index("card_a"), Some(3));
    }

    #[test]
    fn permanent_list_round_trip() {
        let mut b = Bindings::new();
        let h0 = PermanentHandle {
            player: 0,
            index: 0,
        };
        let h1 = PermanentHandle {
            player: 1,
            index: 3,
        };
        b.insert_permanent_list("targets", vec![h0, h1]);
        let got = b.get_permanent_list("targets").expect("set");
        assert_eq!(got, vec![h0, h1]);
    }

    #[test]
    fn card_list_round_trip() {
        let c0 = CardHandle(0);
        let c1 = CardHandle(1);

        let mut b = Bindings::new();
        b.insert_card_list("picks", vec![c0, c1]);
        let got = b.get_card_list("picks").expect("set");
        assert_eq!(got, vec![c0, c1]);
    }

    #[test]
    fn list_clone_is_deep() {
        let mut b = Bindings::new();
        let h0 = PermanentHandle {
            player: 0,
            index: 0,
        };
        b.insert_permanent_list("xs", vec![h0]);
        let cloned = b.clone();
        b.insert_permanent_list("xs", vec![]); // mutate original
        assert_eq!(cloned.get_permanent_list("xs").unwrap().len(), 1);
    }

    #[test]
    fn literal_insert_round_trip() {
        let mut b = Bindings::new();
        b.insert_literal("branch", 1);
        assert_eq!(b.get_literal("branch"), Some(1));
    }
}
