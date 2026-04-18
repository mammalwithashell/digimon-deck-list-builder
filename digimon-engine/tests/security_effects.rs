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

// ─── §2.5j / §2.5b / §2.5g / §2.5h / §2.5k / §2.5l coverage ──────────

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::enums::{Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;
use digimon_engine::selection::SelectionKind;

/// TEST-023 installs `select_hand` during SecuritySkill drain. Defender
/// picks a hand card → callback trashes + draws → resolution continues
/// through remaining phases. (§2.5j)
#[test]
fn selection_during_security_skill_resumes_on_accept() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-023"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(1, &["FILLER"])
        .deck(1, &["FILLER"; 5])
        .security(1, &["TEST-023"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let result = r.attack_player(atk, 1, false);

    assert_eq!(result, AttackResult::InProgress);
    let sel = r.game.pending_selection.as_ref().expect("selection installed");
    assert_eq!(sel.kind, SelectionKind::Hand);
    assert_eq!(sel.selecting_player, 1);
    assert!(r.game.security_resolution.is_some());
    assert!(r.game.pending_attack.is_some());

    r.game.resolve_selection(1, PLAY_HAND_START).expect("resolve");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.security_resolution.is_none());
    assert!(r.game.pending_attack.is_none());
    // 1 FILLER in hand → trashed 1, drew 2 = 2 in hand.
    assert_eq!(r.hand_size(1), 2);
    assert_eq!(r.security_count(1), 0);
    // Trash: TEST-023 (revealed) + FILLER (trashed from hand) = 2.
    assert_eq!(r.trash_size(1), 2);
}

/// Defender PASSes the optional select_hand; resolution still completes.
#[test]
fn selection_during_security_skill_resumes_on_decline() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-023"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(1, &["FILLER"])
        .deck(1, &["FILLER"; 5])
        .security(1, &["TEST-023"])
        .start();

    let hand_before = r.hand_size(1);
    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);

    r.game.resolve_selection(1, PASS).expect("PASS");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.security_resolution.is_none());
    assert_eq!(r.hand_size(1), hand_before);
    assert_eq!(r.trash_size(1), 1, "only revealed card trashes");
}

/// SA+1: two checks. First = TEST-020 (draw 2, no pause), second =
/// TEST-023 (pause). After pause + PASS, both checks finalize.
#[test]
fn second_of_two_security_checks_pauses_preserving_remaining_count() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-020"))
        .add_card(option("TEST-023"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(1, &["FILLER"])
        .deck(1, &["FILLER"; 5])
        // pop order: last-pushed first. Want TEST-020 first, TEST-023 second.
        .security(1, &["TEST-023", "TEST-020"])
        .start();

    let hand_before = r.hand_size(1);
    let atk = r.place_on_field(0, "ATK", Some(0));
    r.game.modifiers.add(
        atk,
        ModifierEntry {
            modifier: ModifierType::SecurityAttackChange,
            value: 1,
            expiry: Expiry::Permanent,
            source_player: 0,
        },
    );

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::InProgress);
    // First check fired TEST-020 (drew 2); second paused on TEST-023.
    assert_eq!(r.hand_size(1), hand_before + 2);
    assert_eq!(r.security_count(1), 0);
    // Only TEST-020 trashed so far — TEST-023 still parked.
    assert_eq!(r.trash_size(1), 1);

    r.game.resolve_selection(1, PASS).expect("PASS");

    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.trash_size(1), 2);
}

/// TEST-025's callback trashes a hand card then calls `play_from_security`.
/// After resume, the `played` bit suppresses trash — card lands on field.
#[test]
fn play_from_security_after_pause_suppresses_trash() {
    let mut t025 = make_test_card("TEST-025", "Test025");
    t025.play_cost = 4;

    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(t025)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(1, &["FILLER"])
        .security(1, &["TEST-025"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);
    r.game.resolve_selection(1, PLAY_HAND_START).expect("resolve");

    assert!(r.game.pending_attack.is_none());
    assert_eq!(r.battle_area_size(1), 1);
    assert_eq!(r.trash_size(1), 1); // FILLER
    assert_eq!(r.security_count(1), 0);
}

/// TEST-024 as OnSecurityCheck observer on defender's field — fires once,
/// after the revealed card's SecuritySkill drain. (§2.5b)
#[test]
fn on_security_check_fires_once_after_full_resume() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-022"))
        .add_card(make_test_card("TEST-024", "Test024"))
        .security(1, &["TEST-022"])
        .start();

    let _obs = r.place_on_field(1, "TEST-024", Some(0));
    let atk = r.place_on_field(0, "ATK", Some(0));
    let memory_before = r.memory();

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    // TEST-022 gain 3 + TEST-024 observer gain 1 = +4.
    assert_eq!(r.memory(), memory_before + 3 + 1);
}

/// TEST-027 gains +7 memory iff `ctx.attacker.is_some()` AND
/// `ctx.turn_player == ctx.attacker.player`. (§2.5g)
#[test]
fn effect_context_carries_attacker_and_turn_player_during_security_skill() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-027"))
        .security(1, &["TEST-027"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let memory_before = r.memory();
    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(r.memory(), memory_before + 7);
}

/// After a paused security check (TEST-023), the DP battle phase must
/// honor Jamming on the attacker.
#[test]
fn jamming_survives_security_pause() {
    let mut big = make_test_card("BIG", "Big");
    big.card_kind = CardKind::Digimon;
    big.dp = Some(20000);
    big.level = Some(7);

    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-023"))
        .add_card(big)
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(1, &["FILLER"])
        .deck(1, &["FILLER"; 5])
        // pop order: TEST-023 first (pause), BIG second (DP battle).
        .security(1, &["BIG", "TEST-023"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.game.modifiers.add(
        atk,
        ModifierEntry {
            modifier: ModifierType::SecurityAttackChange,
            value: 1,
            expiry: Expiry::Permanent,
            source_player: 0,
        },
    );
    r.game
        .modifiers
        .grant_keyword(atk, Keyword::Jamming, Expiry::Permanent, 0);

    let _ = r.attack_player(atk, 1, false);
    r.game.resolve_selection(1, PASS).expect("resolve");

    assert_eq!(r.battle_area_size(0), 1, "Jamming kept attacker alive");
}

/// §2.5k: reveal must remove the card's `card_index` from
/// `face_up_security`.
#[test]
fn face_up_security_removed_on_reveal() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-022"))
        .security(1, &["TEST-022"])
        .start();

    let idx = r.game.player(1).security.last().unwrap().card_index;
    r.game.player_mut(1).face_up_security.insert(idx);

    let atk = r.place_on_field(0, "ATK", Some(0));
    let _ = r.attack_player(atk, 1, false);

    assert!(!r.game.player(1).face_up_security.contains(&idx));
}

/// §2.5h: Python skips `can_use_condition` for SecuritySkill — Rust now
/// matches. TEST-026 has `condition: |_| false` but still fires.
#[test]
fn security_skill_condition_is_not_evaluated() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-026"))
        .security(1, &["TEST-026"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let memory_before = r.memory();
    let _ = r.attack_player(atk, 1, false);
    assert_eq!(r.memory(), memory_before + 5);
}

/// TEST-028: outer callback installs inner `select_hand`. Both prompts
/// resolve before security advances past SecuritySkillDrain.
#[test]
fn nested_selection_in_security_callback_pauses_again() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-028"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(1, &["FILLER", "FILLER"])
        .security(1, &["TEST-028"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let memory_before = r.memory();

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::InProgress);

    r.game.resolve_selection(1, PLAY_HAND_START).expect("outer");
    assert!(r.game.pending_selection.is_some(), "nested pause");
    assert!(r.game.security_resolution.is_some());

    r.game.resolve_selection(1, PLAY_HAND_START).expect("inner");

    assert!(r.game.pending_selection.is_none());
    assert!(r.game.security_resolution.is_none());
    assert_eq!(r.memory(), memory_before + 1 + 2);
}
