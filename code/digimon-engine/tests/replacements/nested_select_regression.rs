//! Phase C regression — synchronous replacement_process closures (Barrier,
//! Evade auto-installs) must continue to work unchanged. The post-process
//! hook short-circuits when pending_selection.is_none().
//!
//! These tests overlap with `tests/replacements/native_keywords.rs` but
//! exist in this file as a Phase C-specific regression marker — if the
//! parked-replacement substrate ever inadvertently breaks the synchronous
//! path, this test catches it.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};
use digimon_engine::replacement::ReplacementCause;

fn with_keyword(id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 4,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords,
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

#[test]
fn barrier_synchronous_process_unchanged() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    let mut r = DebugRunner::builder()
        .add_card(with_keyword("BARRIER-D", 3000, vec![Keyword::Barrier]))
        .add_card(make_test_card("SEC", "SEC"))
        .security(0, &["SEC", "SEC"])
        .start();
    let b = r.place_on_field(0, "BARRIER-D", None);
    let security_size_before = r.game.players[0].security.len();

    r.game
        .delete_permanent_with_cause(b, ReplacementCause::Battle);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Barrier");

    // Barrier: trash top of security, cancel deletion.
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Barrier preserved digimon"
    );
    assert_eq!(
        r.game.players[0].security.len(),
        security_size_before - 1,
        "Barrier trashed top of security"
    );
    assert!(
        r.game.parked_replacement_outcome_for_test().is_none(),
        "synchronous path leaves parked slot None"
    );
}

#[test]
fn evade_synchronous_process_unchanged() {
    use digimon_engine::action::space::REPLACEMENT_ACCEPT;
    let mut r = DebugRunner::builder()
        .add_card(with_keyword("EVADE-D", 3000, vec![Keyword::Evade]))
        .start();
    let e = r.place_on_field(0, "EVADE-D", None);
    let deck_size_before = r.game.players[0].deck.len();

    r.game.delete_permanent_with_effects(e);
    assert!(r.game.pending_selection.is_some());
    r.game
        .resolve_selection(0, REPLACEMENT_ACCEPT)
        .expect("accept Evade");

    // Evade: suspend the carrier and cancel the deletion (per printed text:
    // "you may suspend it to prevent that deletion").
    assert_eq!(
        r.game.players[0].battle_area.len(),
        1,
        "Evade cancelled the deletion — carrier stays on field"
    );
    assert!(
        r.game.players[0].battle_area[0].is_suspended,
        "Evade paid its cost by suspending the carrier"
    );
    assert_eq!(
        r.game.players[0].deck.len(),
        deck_size_before,
        "Evade is not a deck-redirect — deck size unchanged"
    );
    assert!(r.game.parked_replacement_outcome_for_test().is_none());
}
