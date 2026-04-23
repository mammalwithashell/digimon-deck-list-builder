//! Named-binding environment for DSL process steps.

use std::collections::HashMap;

use crate::card_source::CardHandle;
use crate::permanent::PermanentHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingValue {
    Permanent(PermanentHandle),
    Card(CardHandle),
    HandIndex(u16),
    TrashIndex(u16),
    Literal(i64),
}

#[derive(Debug, Default, Clone)]
pub struct Bindings {
    slots: HashMap<String, BindingValue>,
}

impl Bindings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: &str, value: BindingValue) {
        self.slots.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<BindingValue> {
        self.slots.get(name).copied()
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
            BindingValue::HandIndex(i) => Some(i),
            _ => None,
        }
    }

    pub fn get_trash_index(&self, name: &str) -> Option<u16> {
        match self.get(name)? {
            BindingValue::TrashIndex(i) => Some(i),
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

    pub fn insert_hand_index(&mut self, name: &str, i: u16) {
        self.insert(name, BindingValue::HandIndex(i));
    }

    pub fn insert_trash_index(&mut self, name: &str, i: u16) {
        self.insert(name, BindingValue::TrashIndex(i));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_preserves_slots() {
        let mut original = Bindings::new();
        original.insert_hand_index("card_a", 3);
        original.insert_trash_index("card_b", 7);
        original.insert("lit", BindingValue::Literal(42));

        let cloned = original.clone();

        assert_eq!(cloned.get_hand_index("card_a"), Some(3));
        assert_eq!(cloned.get_trash_index("card_b"), Some(7));
        assert_eq!(cloned.get_literal("lit"), Some(42));
        // Mutating original does not affect clone
        let mut original = original;
        original.insert_hand_index("card_a", 99);
        assert_eq!(cloned.get_hand_index("card_a"), Some(3));
    }
}
