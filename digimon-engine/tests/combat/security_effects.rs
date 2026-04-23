//! Behavioral tests for `SecuritySkill` effect execution in the Rust engine.
//!
//! Covers the three pilot cards landed alongside the `resolve_security_card`
//! rewrite (RUST_PYTHON_PARITY §2.5):
//!   * TEST-020 — trigger-and-trash (draw 2 from security)
//!   * TEST-021 — play-from-security (card stays on field)
//!   * TEST-022 — memory gain from security (observer-style, trashed after)

use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn attacker() -> CardData {
    CardData {
        card_id: "ATK".to_string(),
        card_name: "Attacker".to_string(),
        card_kind: CardKind::Digimon,
        level: Some(5),
        dp: Some(8000),
        play_cost: 6,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: "ATK".to_string(),
        index: 0,
        norm_id: 0.0,
    }
}

fn option(id: &str) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Option,
        level: None,
        dp: None,
        play_cost: 2,
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

/// TEST-020 (draw 2) fires on reveal; revealed card trashes afterwards.
#[test]
fn test_020_draws_two_from_security() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-020"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"; 5])
        .security(1, &["TEST-020"])
        .start();

    let hand_before = r.hand_size(1);
    let atk = r.place_on_field(0, "ATK", Some(0));

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);

    assert_eq!(
        r.hand_size(1),
        hand_before + 2,
        "SecuritySkill effect must draw 2 cards for the defender"
    );
    assert_eq!(
        r.security_count(1),
        0,
        "the revealed card must leave the security stack"
    );
    assert_eq!(
        r.trash_size(1),
        1,
        "no effect played the card, so it trashes after the check"
    );
    assert_eq!(r.battle_area_size(1), 0);
}

/// TEST-021 (play self from security) keeps the revealed card on the
/// defender's field rather than trashing it.
#[test]
fn test_021_plays_self_from_security() {
    // TEST-021 is Option-kind; placing it as a permanent treats it as a
    // non-Digimon field card — that's fine for this test, we just need a
    // body to observe. We make it a Digimon so existing field-slot logic
    // stays happy without reaching into Option-specific play rules.
    let mut test021 = make_test_card("TEST-021", "Test021");
    test021.play_cost = 5; // deliberately expensive — we want the
                           // "without paying cost" aspect to show up.

    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(test021)
        .security(1, &["TEST-021"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let memory_before = r.memory();
    let result = r.attack_player(atk, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        r.battle_area_size(1),
        1,
        "play_from_security put the card on the defender's field"
    );
    assert_eq!(
        r.security_count(1),
        0,
        "revealed card left security"
    );
    assert_eq!(
        r.trash_size(1),
        0,
        "the card must NOT trash — security_played bit skips the default trash"
    );
    assert_eq!(
        r.memory(),
        memory_before,
        "no memory is paid — play_from_security bypasses the cost"
    );
}

/// TEST-022 (gain 3 memory) fires, memory changes, and the card trashes.
#[test]
fn test_022_gains_memory_from_security() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-022"))
        .security(1, &["TEST-022"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let memory_before = r.memory();

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);

    assert_eq!(
        r.memory(),
        memory_before + 3,
        "SecuritySkill memory gain must apply to the defender's side of the seesaw"
    );
    assert_eq!(r.trash_size(1), 1);
}

/// When the revealed card is Option/Tamer with NO security effect at all,
/// the pre-rewrite behavior (simple trash) still holds — regression guard
/// for the `resolve_security_card` rewrite.
#[test]
fn no_security_effect_trashes_as_before() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("PLAIN"))
        .security(1, &["PLAIN"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let result = r.attack_player(atk, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(r.trash_size(1), 1);
    assert_eq!(r.security_count(1), 0);
    assert_eq!(r.battle_area_size(1), 0);
}

/// After security resolution completes, `Game.pending_security` must be
/// cleared. Leaving it set would poison future security checks.
#[test]
fn pending_security_is_cleared_after_check() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-022"))
        .security(1, &["TEST-022"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);

    assert!(
        r.game.pending_security.is_none(),
        "pending_security must be cleared once the check resolves"
    );
}
