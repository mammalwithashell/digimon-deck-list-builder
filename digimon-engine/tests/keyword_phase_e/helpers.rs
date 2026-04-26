//! Shared fixtures for Phase E behavioral tests. Mirrors
//! `tests/keyword_phase_d/helpers.rs`'s DebugRunner-oriented shape; adds a
//! `plain_digimon` and `digimon_with_keywords` builder for compact per-test
//! CardData customization.

use digimon_engine::card_data::CardData;
use digimon_engine::enums::{CardColor, CardKind, Keyword};

pub fn plain_digimon(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

pub fn digimon_with_keywords(id: &str, level: u8, dp: i32, kws: Vec<Keyword>) -> CardData {
    let mut c = plain_digimon(id);
    c.level = Some(level);
    c.dp = Some(dp);
    c.keywords = kws;
    c
}
