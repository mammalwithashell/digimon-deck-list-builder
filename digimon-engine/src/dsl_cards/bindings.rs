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

#[derive(Debug, Default)]
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

    pub fn get_literal(&self, name: &str) -> Option<i64> {
        match self.get(name)? {
            BindingValue::Literal(v) => Some(v),
            _ => None,
        }
    }
}
