//! Real printed-text regression guard for `<Security A. +1>` on the six cards
//! that once declared it as a self `security_attack` aura
//! (G-ENGINE-SECURITY-ATTACK-AURA-PLUS-FACE-DOUBLE-COUNT, 2026-10-04).
//!
//! `raw_security_strike` sums base 1 + `SecurityAttackChange` modifiers +
//! `security_attack_keyword_bonus`, and the keyword bonus parses the PRINTED
//! face text. A self aura `security_attack: 1` on a card whose face also prints
//! `<Security A. +1>` therefore counted twice (3 checks) in any game built
//! from real card data (harness, live play, training). The per-card
//! `DebugRunner` tests can't see it: `dsl_card` registers compiled effects with
//! EMPTY printed text, so `face_keywords` falls back to `card_data.keywords`.
//!
//! These tests build from `full_card_data()` (populated `effect_text`, the
//! same path as the live game) and assert exactly 2 checks. The fix is the
//! `grant_keyword: SecurityAttackPlus` form, which the declarative tick skips
//! when the keyword is already printed.

use digimon_engine::deck_tools::full_card_data;
use digimon_engine::enums::GamePhase;
use digimon_engine::game::Game;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::rules::Rules;

/// Real-data game with `card_id` alone on player 0's field, on player 0's turn.
fn real_data_strike(card_id: &str) -> i32 {
    let db = full_card_data();
    let deck: Vec<String> = std::iter::repeat("ST1-01".to_string())
        .take(5)
        .chain(std::iter::repeat("ST1-03".to_string()).take(45))
        .collect();
    let decks = vec![deck.clone(), deck];
    let mut game = Game::new(&decks, &db, Rules::standard(), Some(0))
        .expect("deck must construct from full card data");
    game.start_game();
    while let Some(p) = game.mulligan_current_player() {
        let _ = game.accept_mulligan(p, true);
    }
    game.stage_set_first_player(0);
    game.current_phase = GamePhase::Main;
    let index = game.stage_place_field_stack(0, &[card_id], false, 1);
    let handle = PermanentHandle {
        player: 0,
        index: index as u8,
    };
    game.tick_declarative_effects();
    game.effective_security_strike(handle)
}

fn assert_two_checks(card_id: &str) {
    assert_eq!(
        real_data_strike(card_id),
        2,
        "{card_id} prints <Security A. +1>: base 1 + 1 = 2 checks with real card \
         text (the printed keyword must not be counted a second time)"
    );
}

#[test]
fn bt10_013_real_text_security_attack_plus_one_checks_two() {
    assert_two_checks("BT10-013");
}

#[test]
fn bt24_014_real_text_security_attack_plus_one_checks_two() {
    assert_two_checks("BT24-014");
}

#[test]
fn bt25_103_real_text_security_attack_plus_one_checks_two() {
    assert_two_checks("BT25-103");
}

#[test]
fn bt26_060_real_text_security_attack_plus_one_checks_two() {
    assert_two_checks("BT26-060");
}

#[test]
fn bt26_080_real_text_security_attack_plus_one_checks_two() {
    assert_two_checks("BT26-080");
}

#[test]
fn ex7_023_real_text_security_attack_plus_one_checks_two() {
    assert_two_checks("EX7-023");
}
