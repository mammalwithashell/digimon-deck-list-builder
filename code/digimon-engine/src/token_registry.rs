//! Token metadata catalog — the Rust analogue of
//! `digimon_gym/engine/data/token_registry.py`. Maps a canonical
//! token name (`"petrification"`) to `TokenDef` metadata that
//! `EffectContext::play_token` consumes to synthesize a `CardSource`
//! + `Permanent` directly onto the battle area, bypassing hand /
//! deck / play-cost.
//!
//! Tokens are `CardKind::Token` and obey leave-field removal-from-game
//! semantics (see `player::delete_permanent`). Printed abilities are
//! carried by the set-wide `CardEffectRegistry` under a synthetic
//! `card_id` like `"TOKEN_PETRIFICATION"` — `Game::card_data`
//! absorbs a `CardData` row for each registered token at `Game::new`
//! time so the rest of the engine (tensor, mask, combat) treats them
//! identically to deck-sourced cards.

use std::collections::HashMap;

use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind};

/// Declarative token metadata. No closures live here — printed abilities
/// live on the `CardEffect` implementation registered against
/// `TokenDef::card_id` in the main `CardEffectRegistry`.
#[derive(Debug, Clone)]
pub struct TokenDef {
    /// Canonical lookup key (e.g. `"petrification"`).
    pub name: String,
    /// Synthetic card_id used as the effect-registry key and as the
    /// `CardData::card_id` of materialized instances
    /// (e.g. `"TOKEN_PETRIFICATION"`).
    pub card_id: String,
    pub card_name: String,
    pub colors: Vec<CardColor>,
    pub dp: Option<i32>,
    pub level: Option<u8>,
    pub traits: Vec<String>,
}

impl TokenDef {
    /// Synthesize a `CardData` row from this token definition. `Game::new`
    /// calls this for every registered token and pushes the result into
    /// `game.card_data` so `CardSource::data_index` lookups work.
    pub fn to_card_data(&self) -> CardData {
        CardData {
            card_id: self.card_id.clone(),
            card_name: self.card_name.clone(),
            card_kind: CardKind::Token,
            level: self.level,
            dp: self.dp,
            play_cost: 0,
            colors: self.colors.clone(),
            traits: self.traits.clone(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            effect_class_name: self.card_id.clone(),
            keywords: Vec::new(),
            index: 0,
            norm_id: 0.0,
            dual: None,
        }
    }
}

/// Registry of token definitions, keyed by canonical name.
#[derive(Debug, Default, Clone)]
pub struct TokenRegistry {
    defs: HashMap<String, TokenDef>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, def: TokenDef) {
        self.defs.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&TokenDef> {
        self.defs.get(name)
    }

    pub fn len(&self) -> usize {
        self.defs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// All registered token definitions. Used by `Game::new` to
    /// seed `card_data` with synthetic rows.
    pub fn iter(&self) -> impl Iterator<Item = &TokenDef> {
        self.defs.values()
    }
}

/// Build the default token registry. Currently: Petrification (Medusamon),
/// Familiar (TS Olympos). Additional tokens land as their archetypes are
/// ported.
pub fn build_registry() -> TokenRegistry {
    let mut r = TokenRegistry::new();
    r.insert(TokenDef {
        name: "petrification".to_string(),
        card_id: "TOKEN_PETRIFICATION".to_string(),
        card_name: "Petrification Token".to_string(),
        colors: vec![CardColor::White],
        dp: Some(3000),
        level: None,
        traits: Vec::new(),
    });
    r.insert(TokenDef {
        name: "familiar".to_string(),
        card_id: "TOKEN_FAMILIAR".to_string(),
        card_name: "Familiar Token".to_string(),
        colors: vec![CardColor::Yellow],
        dp: Some(3000),
        level: None,
        traits: Vec::new(),
    });
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn petrification_registered() {
        let r = build_registry();
        let def = r.get("petrification").expect("petrification missing");
        assert_eq!(def.card_id, "TOKEN_PETRIFICATION");
        assert_eq!(def.dp, Some(3000));
        assert!(def.level.is_none());
    }

    #[test]
    fn to_card_data_marks_token_kind() {
        let r = build_registry();
        let def = r.get("petrification").unwrap();
        let cd = def.to_card_data();
        assert_eq!(cd.card_kind, CardKind::Token);
        assert_eq!(cd.play_cost, 0);
    }

    #[test]
    fn unknown_name_returns_none() {
        let r = build_registry();
        assert!(r.get("no-such-token-lol").is_none());
    }
}
