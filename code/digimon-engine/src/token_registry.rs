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
use crate::enums::{CardColor, CardKind, Keyword};

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
    /// Printed play cost, for the rare token that has one (DCGO
    /// `CEntity_Base.PlayCost`: Diaboromon 14). `None` = no play cost -- the
    /// stored `CardData::play_cost` is 0 and `top_card_has_play_cost` /
    /// play-cost-budget picks treat the token as costless. A token is never
    /// *paid* for (it is only ever put onto the field by an effect); the
    /// value matters to play-cost filters ("play cost 8 or less") reading
    /// the token on the field.
    pub play_cost: Option<u16>,
    /// Static printed keywords (e.g. `<Reboot> <Blocker> <Decoy (Red/Black)>`).
    /// Mirrors `CardData::keywords`. Defaults to empty for tokens with no
    /// printed keywords.
    pub keywords: Vec<Keyword>,
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
            play_cost: self.play_cost.unwrap_or(0),
            colors: self.colors.clone(),
            traits: self.traits.clone(),
            evo_costs: Vec::new(),
            dna_costs: Vec::new(),
            effect_text: String::new(),
            inherited_text: String::new(),
            security_text: String::new(),
            link_text: String::new(),
            requirement_text: String::new(),
            effect_class_name: self.card_id.clone(),
            keywords: self.keywords.clone(),
            index: 0,
            norm_id: 0.0,
            ace_overflow: None,
            dual: None,
            digixros_aliases: Vec::new(),
            also_treated_as: Vec::new(),
        }
    }
}

/// Registry of token definitions, keyed by canonical name.
#[derive(Debug, Default, Clone)]
pub struct TokenRegistry {
    defs: std::sync::Arc<HashMap<String, TokenDef>>,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, def: TokenDef) {
        std::sync::Arc::make_mut(&mut self.defs).insert(def.name.clone(), def);
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

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.defs.keys().map(String::as_str)
    }
}

/// Process-cached default token registry. Cloning returns an `Arc` bump.
pub fn build_registry_cached() -> TokenRegistry {
    static CACHED: std::sync::OnceLock<TokenRegistry> = std::sync::OnceLock::new();
    CACHED.get_or_init(build_registry).clone()
}

/// Build the default token registry. Currently: Petrification (Medusamon),
/// Familiar (TS Olympos), Atho/René/Por (Royal Knights — BT20-017 Jesmon,
/// BT23-013 Jesmon CS), Hinukamuy (Royal Knights — BT23-057 Gankoomon),
/// Kotenken (EX12-034), and Paishu (EX12-057). Additional tokens land as
/// their archetypes are ported.
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
        play_cost: None,
        keywords: Vec::new(),
    });
    r.insert(TokenDef {
        name: "familiar".to_string(),
        card_id: "TOKEN_FAMILIAR".to_string(),
        card_name: "Familiar Token".to_string(),
        colors: vec![CardColor::Yellow],
        dp: Some(3000),
        level: None,
        traits: Vec::new(),
        play_cost: None,
        keywords: Vec::new(),
    });
    // [Atho, René & Por] — printed stats and keywords from BT20-017 Jesmon:
    // "(Digimon/White/6000 DP/<Reboot> <Blocker> <Decoy (Red/Black)>)".
    // Decoy color bitmask: bit 0 (Red) | bit 5 (Black) == 0b0010_0001 == 33.
    r.insert(TokenDef {
        name: "atho_rene_por".to_string(),
        card_id: "TOKEN_ATHO_RENE_POR".to_string(),
        card_name: "Atho, René & Por".to_string(),
        colors: vec![CardColor::White],
        dp: Some(6000),
        level: None,
        traits: Vec::new(),
        play_cost: None,
        keywords: vec![
            Keyword::Reboot,
            Keyword::Blocker,
            Keyword::Decoy((1u8 << CardColor::Red as u8) | (1u8 << CardColor::Black as u8)),
        ],
    });
    // [Hinukamuy] — printed stats and keywords from BT23-057 Gankoomon:
    // "(Digimon/White/6000 DP/<Alliance> <Reboot> <Blocker>)".
    r.insert(TokenDef {
        name: "hinukamuy".to_string(),
        card_id: "TOKEN_HINUKAMUY".to_string(),
        card_name: "Hinukamuy Token".to_string(),
        colors: vec![CardColor::White],
        dp: Some(6000),
        level: None,
        traits: Vec::new(),
        play_cost: None,
        keywords: vec![Keyword::Alliance, Keyword::Reboot, Keyword::Blocker],
    });
    // [Kotenken] — printed stats from EX12-034 Erlangmon:
    // "(Digimon/Black/9000 DP/<Blocker>)".
    r.insert(TokenDef {
        name: "kotenken".to_string(),
        card_id: "TOKEN_KOTENKEN".to_string(),
        card_name: "Kotenken Token".to_string(),
        colors: vec![CardColor::Black],
        dp: Some(9000),
        level: None,
        traits: Vec::new(),
        play_cost: None,
        keywords: vec![Keyword::Blocker],
    });
    // [Paishu] — printed stats from EX12-057 Takutoumon:
    // "(Digimon/Yellow/6000 DP/<Blocker> <Guard>)".
    r.insert(TokenDef {
        name: "paishu".to_string(),
        card_id: "TOKEN_PAISHU".to_string(),
        card_name: "Paishu Token".to_string(),
        colors: vec![CardColor::Yellow],
        dp: Some(6000),
        level: None,
        traits: Vec::new(),
        play_cost: None,
        keywords: vec![Keyword::Blocker, Keyword::Guard],
    });
    // [Diaboromon] -- printed stats from BT5-067 Infermon / EX6-043 Diaboromon:
    // "(Digimon/Cost 14/Lv.6/White/Mega/Unknown/Unidentified/3000 DP)". DCGO
    // `ContinuousController.DiaboromonToken` (CardName "Diaboromon", PlayCost
    // 14, Level 6, White, Mega / Unknown / Unidentified, DP 3000). Named plain
    // "Diaboromon" (not "... Token") because the printed cards address it by
    // that exact name ("1 of your other [Diaboromon]", `EqualsCardName`).
    // Traits are the folded trait line (form + attribute + type, CR 4.3
    // 2-3-2-1) like every production card.
    r.insert(TokenDef {
        name: "diaboromon".to_string(),
        card_id: "TOKEN_DIABOROMON".to_string(),
        card_name: "Diaboromon".to_string(),
        colors: vec![CardColor::White],
        dp: Some(3000),
        level: Some(6),
        traits: vec![
            "Mega".to_string(),
            "Unknown".to_string(),
            "Unidentified".to_string(),
        ],
        play_cost: Some(14),
        keywords: Vec::new(),
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
    fn all_registered_tokens_synthesize_token_card_data() {
        let registry = build_registry();
        for name in registry.names() {
            let token = registry.get(name).expect("registered token exists");
            let data = token.to_card_data();
            assert_eq!(data.card_kind, CardKind::Token, "{name}");
            // Only a token that prints a play cost (Diaboromon 14) stores one.
            assert_eq!(
                data.play_cost,
                token.play_cost.unwrap_or(0),
                "{name}"
            );
            assert!(data.evo_costs.is_empty(), "{name}");
            assert!(data.dna_costs.is_empty(), "{name}");
            assert_eq!(data.effect_class_name, data.card_id, "{name}");
        }
    }

    #[test]
    fn diaboromon_registered_with_printed_stats() {
        let r = build_registry();
        let def = r.get("diaboromon").expect("diaboromon missing");
        assert_eq!(def.card_id, "TOKEN_DIABOROMON");
        assert_eq!(def.card_name, "Diaboromon");
        assert_eq!(def.colors, vec![CardColor::White]);
        assert_eq!(def.dp, Some(3000));
        assert_eq!(def.level, Some(6));
        assert_eq!(def.play_cost, Some(14));
        for t in ["Mega", "Unknown", "Unidentified"] {
            assert!(def.traits.iter().any(|x| x == t), "{t}");
        }
        let cd = def.to_card_data();
        assert_eq!(cd.card_kind, CardKind::Token);
        assert_eq!(cd.play_cost, 14);
    }

    #[test]
    fn unknown_name_returns_none() {
        let r = build_registry();
        assert!(r.get("no-such-token-lol").is_none());
    }

    #[test]
    fn atho_rene_por_registered_with_printed_stats_and_keywords() {
        let r = build_registry();
        let def = r.get("atho_rene_por").expect("Atho, René & Por missing");
        assert_eq!(def.card_id, "TOKEN_ATHO_RENE_POR");
        assert_eq!(def.card_name, "Atho, René & Por");
        assert_eq!(def.colors, vec![CardColor::White]);
        assert_eq!(def.dp, Some(6000));
        assert!(def.keywords.contains(&Keyword::Reboot));
        assert!(def.keywords.contains(&Keyword::Blocker));
        let red_black = (1u8 << CardColor::Red as u8) | (1u8 << CardColor::Black as u8);
        assert!(def.keywords.contains(&Keyword::Decoy(red_black)));
    }

    #[test]
    fn atho_rene_por_card_data_carries_keywords() {
        let r = build_registry();
        let def = r.get("atho_rene_por").unwrap();
        let cd = def.to_card_data();
        assert_eq!(cd.card_kind, CardKind::Token);
        assert_eq!(cd.dp, Some(6000));
        assert!(cd.keywords.contains(&Keyword::Reboot));
        assert!(cd.keywords.contains(&Keyword::Blocker));
    }

    #[test]
    fn hinukamuy_registered_with_printed_stats_and_keywords() {
        let r = build_registry();
        let def = r.get("hinukamuy").expect("Hinukamuy missing");
        assert_eq!(def.card_id, "TOKEN_HINUKAMUY");
        assert_eq!(def.card_name, "Hinukamuy Token");
        assert_eq!(def.colors, vec![CardColor::White]);
        assert_eq!(def.dp, Some(6000));
        assert!(def.level.is_none());
        assert!(def.traits.is_empty());
        assert!(def.keywords.contains(&Keyword::Alliance));
        assert!(def.keywords.contains(&Keyword::Reboot));
        assert!(def.keywords.contains(&Keyword::Blocker));
    }

    #[test]
    fn hinukamuy_card_data_carries_keywords() {
        let r = build_registry();
        let def = r.get("hinukamuy").unwrap();
        let cd = def.to_card_data();
        assert_eq!(cd.card_kind, CardKind::Token);
        assert_eq!(cd.dp, Some(6000));
        assert!(cd.keywords.contains(&Keyword::Alliance));
        assert!(cd.keywords.contains(&Keyword::Reboot));
        assert!(cd.keywords.contains(&Keyword::Blocker));
    }

    #[test]
    fn ex12_kotenken_registered_with_printed_stats_and_keywords() {
        let r = build_registry();
        let def = r.get("kotenken").expect("Kotenken missing");
        assert_eq!(def.card_id, "TOKEN_KOTENKEN");
        assert_eq!(def.card_name, "Kotenken Token");
        assert_eq!(def.colors, vec![CardColor::Black]);
        assert_eq!(def.dp, Some(9000));
        assert!(def.level.is_none());
        assert!(def.keywords.contains(&Keyword::Blocker));
    }

    #[test]
    fn ex12_paishu_registered_with_printed_stats_and_keywords() {
        let r = build_registry();
        let def = r.get("paishu").expect("Paishu missing");
        assert_eq!(def.card_id, "TOKEN_PAISHU");
        assert_eq!(def.card_name, "Paishu Token");
        assert_eq!(def.colors, vec![CardColor::Yellow]);
        assert_eq!(def.dp, Some(6000));
        assert!(def.level.is_none());
        assert!(def.keywords.contains(&Keyword::Blocker));
        assert!(def.keywords.contains(&Keyword::Guard));
    }
}
