//! Shared fixtures for Phase F behavioral tests. Mirrors
//! `tests/keyword_phase_e/helpers.rs`'s shape; adds tamer builders for
//! later-phase keywords (MindLink) that need Tamer hosts.
//!
//! Note: `attach_face_down_source` (face-down digivolution sources) is
//! deferred to Task 5, which adds the underlying `face_down: bool` field
//! on `CardSource`. Until then, this helper module stays minimal.

#![allow(dead_code)]

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

pub fn plain_tamer(id: &str) -> CardData {
    let mut c = plain_digimon(id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c
}

pub fn digimon_with_keywords(id: &str, level: u8, dp: i32, kws: Vec<Keyword>) -> CardData {
    let mut c = plain_digimon(id);
    c.level = Some(level);
    c.dp = Some(dp);
    c.keywords = kws;
    c
}

pub fn tamer_with_keywords(id: &str, kws: Vec<Keyword>) -> CardData {
    let mut c = plain_tamer(id);
    c.keywords = kws;
    c
}
