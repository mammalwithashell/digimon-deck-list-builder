//! Shared fixtures for Phase F behavioral tests. Mirrors
//! `tests/keyword_phase_e/helpers.rs`'s shape; adds tamer builders for
//! later-phase keywords (MindLink) that need Tamer hosts.
//!
//! Task 5 added `attach_face_down_source` once the underlying
//! `face_down: bool` field on `CardSource` landed.

#![allow(dead_code)]

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::DebugRunner;
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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
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

/// Append a `face_down` digivolution source under the given permanent's
/// top — used by MindLink tests to verify the filter ignores face-down Tamer
/// sources. The new source is inserted at index 0 (bottom of stack), so
/// the existing top card remains the visible top.
///
/// `<Training>` (Phase F Task 6) is the production caller of the
/// `face_down: bool` flag; this helper exists purely for behavioral tests
/// in Phase F that need to verify face-down-Tamer-source filtering before
/// `<Training>`'s own auto-install lands.
pub fn attach_face_down_source(
    r: &mut DebugRunner,
    target_player: u8,
    target_index: usize,
    card_id: &str,
) {
    let data_index = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap_or_else(|| panic!("attach_face_down_source: unknown card_id {}", card_id));
    let next_card_index = r.game.next_card_index();
    let mut source = CardSource::new(data_index, target_player, next_card_index);
    source.face_down = true;
    let perm = r.game.players[target_player as usize]
        .battle_area
        .get_mut(target_index)
        .expect("attach_face_down_source: target permanent slot out of range");
    // Insert at index 0 (bottom of stack) so the original top remains visible.
    perm.card_sources.insert(0, source);
}
