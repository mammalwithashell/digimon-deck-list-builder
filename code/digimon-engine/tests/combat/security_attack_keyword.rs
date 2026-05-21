//! Phase A §A3 — native `<Security A. +N>` / `<Security A. -N>` consumed
//! at the security-loop site. No hand-rolled `CardEffect` required.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType};

fn fighter(id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(dp),
        play_cost: 5,
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

fn seed_security(r: &mut DebugRunner, player: u8, card_id: &str, count: usize) {
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == card_id)
        .unwrap();
    for _ in 0..count {
        let idx = r.game.next_card_index();
        r.game.players[player as usize]
            .security
            .push(CardSource::new(data_idx, player, idx));
    }
}

#[test]
fn security_attack_plus_one_adds_one_check() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 6000, vec![Keyword::SecurityAttackPlus(1)]))
        .add_card(fighter("SEC", 0, vec![]))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    seed_security(&mut r, 1, "SEC", 3);

    let _result = r.attack_player(attacker, 1, false);
    // Expected: two security cards consumed (base 1 + 1 from keyword).
    assert_eq!(
        r.game.players[1].security.len(),
        1,
        "base 1 check + Plus(1) = 2 checks consumed"
    );
}

#[test]
fn security_attack_minus_one_gives_zero_checks() {
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 6000, vec![Keyword::SecurityAttackMinus(1)]))
        .add_card(fighter("SEC", 0, vec![]))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    seed_security(&mut r, 1, "SEC", 3);

    let result = r.attack_player(attacker, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "0 checks means the attacker survives without consuming security"
    );
    assert_eq!(
        r.game.players[1].security.len(),
        3,
        "Minus(1) cancels the base check -> no security consumed"
    );
}

#[test]
fn security_attack_keyword_stacks_with_modifier() {
    // Native keyword + modifier-granted SecurityAttackChange should sum.
    let mut r = DebugRunner::builder()
        .add_card(fighter("ATK", 6000, vec![Keyword::SecurityAttackPlus(1)]))
        .add_card(fighter("SEC", 0, vec![]))
        .start();

    let attacker = r.place_on_field(0, "ATK", None);
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Rush, Expiry::EndOfTurn, 0);
    r.game.modifiers.add(
        attacker,
        digimon_engine::modifiers::ModifierEntry::simple(
            ModifierType::SecurityAttackChange,
            1,
            Expiry::EndOfTurn,
            0,
        ),
    );
    seed_security(&mut r, 1, "SEC", 5);

    let _result = r.attack_player(attacker, 1, false);
    assert_eq!(
        r.game.players[1].security.len(),
        2,
        "base 1 + keyword +1 + modifier +1 = 3 checks"
    );
}
