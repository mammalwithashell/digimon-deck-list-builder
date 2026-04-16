//! Main-phase action-mask parity tests.
//!
//! Each test in this file locks in a specific §4.x behavior gap from
//! RUST_PYTHON_PARITY.md so that regressions are caught immediately.
//!
//! §4.2 — Option card color requirement
//! §4.3 — Blitz attack exception (memory < 0)       [reserved]
//! §4.4 — Raid target rule (unsuspended targets)     [reserved]

use digimon_engine::action::{build_action_mask, encode_attack, SECURITY_TARGET};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType};

// ─── Card factories ────────────────────────────────────────────────────

fn make_option(id: &str, color: CardColor) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 3,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn make_digimon(id: &str, color: CardColor) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(4000),
        play_cost: 5,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

fn make_tamer(id: &str, color: CardColor) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Tamer,
        level: None,
        dp: None,
        play_cost: 4,
        colors: vec![color],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── §4.2 Option color requirement ────────────────────────────────────

/// An Option card must be masked out (mask[0] == 0.0) when the player has
/// no Digimon or Tamer of a matching color on the field.
#[test]
fn mask_option_requires_matching_color_on_field() {
    // Red Option in hand; Blue Digimon on field — no color match.
    let mut r = DebugRunner::builder()
        .add_card(make_option("OPT-R", CardColor::Red))
        .add_card(make_digimon("BLUE-MON", CardColor::Blue))
        .hand(0, &["OPT-R"])
        .start();

    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Place a Blue Digimon for the player — wrong color.
    r.place_on_field(0, "BLUE-MON", Some(0));

    let mask = build_action_mask(&r.game, 0);

    assert_eq!(
        mask[0], 0.0,
        "Option with no matching-color Digimon on field must be masked out"
    );
}

/// A Tamer of the matching color also satisfies the Option color requirement.
#[test]
fn mask_option_color_check_accepts_tamer() {
    // Red Option in hand; Red Tamer on field — should satisfy the requirement.
    let mut r = DebugRunner::builder()
        .add_card(make_option("OPT-R", CardColor::Red))
        .add_card(make_tamer("TAMER-R", CardColor::Red))
        .hand(0, &["OPT-R"])
        .start();

    r.game.set_memory(5);
    r.game.enter_main_phase();

    // Place a Red Tamer on field — correct color, Tamer type.
    r.place_on_field(0, "TAMER-R", Some(0));

    let mask = build_action_mask(&r.game, 0);

    assert_eq!(
        mask[0], 1.0,
        "Option with matching-color Tamer on field must be playable"
    );
}

/// When the field has a same-color Digimon, the Option is playable.
#[test]
fn mask_option_accepts_matching_digimon() {
    // Red Option in hand; Red Digimon on field — should be playable.
    let mut r = DebugRunner::builder()
        .add_card(make_option("OPT-R", CardColor::Red))
        .add_card(make_digimon("RED-MON", CardColor::Red))
        .hand(0, &["OPT-R"])
        .start();

    r.game.set_memory(5);
    r.game.enter_main_phase();

    r.place_on_field(0, "RED-MON", Some(0));

    let mask = build_action_mask(&r.game, 0);

    assert_eq!(
        mask[0], 1.0,
        "Option with matching-color Digimon on field must be playable"
    );
}
