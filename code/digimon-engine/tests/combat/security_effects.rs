//! Behavioral tests for `SecuritySkill` effect execution in the Rust engine.
//!
//! Covers the three pilot cards landed alongside the `resolve_security_card`
//! rewrite (RUST_PYTHON_PARITY §2.5):
//!   * TEST-020 — trigger-and-trash (draw 2 from security)
//!   * TEST-021 — play-from-security (card stays on field)
//!   * TEST-022 — memory gain from security (observer-style, trashed after)

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

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
        dual: None,
        effect_class_name: "ATK".to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
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
        dual: None,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
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
    assert_eq!(r.security_count(1), 0, "revealed card left security");
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
        memory_before - 3,
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

// §2.5c — Progress / ImmunityToOpponentEffects pilots removed.
// The initial implementation gated SecuritySkillDrain on these, which
// turned out to be semantically incorrect: Progress makes the attacker
// immune to opponent effects during the attack, but does NOT suppress
// the defender's SecuritySkill phase from firing. See
// docs/DCGO_KEYWORD_PARITY.md under "Progress" for the correct consumer
// shape (filter at opponent-effect mutation sites, not a phase gate).
// The Keyword::Progress + ModifierType::ImmunityToOpponentEffects enum
// primitives remain for the forthcoming correct implementation.

// ─── §2.5d: DontBattleSecurityDigimon modifier ───────────────────────

fn digimon_security(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
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

/// Attacker with `DontBattleSecurityDigimon` must NOT trade DP against a
/// Digimon security card — attacker survives even when it would normally
/// lose the DP compare.
#[test]
fn dont_battle_security_digimon_skips_dp_compare() {
    let mut r = DebugRunner::builder()
        .add_card(attacker()) // 8000 DP
        .add_card(digimon_security("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    r.game_mut().modifiers.add(
        atk,
        ModifierEntry::simple(
            ModifierType::DontBattleSecurityDigimon,
            1,
            Expiry::EndOfTurn,
            0,
        ),
    );

    let result = r.attack_player(atk, 1, false);
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        r.battle_area_size(0),
        1,
        "attacker must survive — no DP compare was run"
    );
    assert_eq!(r.trash_size(1), 1, "security card still trashes");
}

/// Baseline without the modifier — attacker's 8000 DP loses to a 9000 DP
/// security Digimon. Confirms the modifier is the thing doing the work.
#[test]
fn without_dont_battle_modifier_attacker_dies_to_higher_dp_security() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(digimon_security("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let result = r.attack_player(atk, 1, false);

    assert_eq!(result, AttackResult::AttackerDeletedBySecurity);
    assert_eq!(r.battle_area_size(0), 0);
}

// ─── §2.5e: inherited-stack DP adjustments vs. security ──────────────

/// Attacker with a digivolution source that carries an
/// `applies_to_opponent_security_dp` effect gains the DP swing in the
/// security battle. TEST-027 encodes a `-3000 DP` adjustment applied to
/// the opposing security Digimon (matching Python's sign convention),
/// which lets the attacker survive a security Digimon it would otherwise
/// lose to.
#[test]
fn inherited_applies_to_opponent_security_dp_adjusts_battle() {
    let mut test027 = make_test_card("TEST-027", "Test027");
    test027.play_cost = 3;

    let mut r = DebugRunner::builder()
        .add_card(attacker()) // 8000 DP
        .add_card(test027)
        .add_card(digimon_security("SEC", 9000))
        .security(1, &["SEC"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    // Push a TEST-027 source underneath the attacker so its inherited
    // effect contributes during the security DP battle.
    {
        let game = r.game_mut();
        let data_idx = game
            .card_data
            .iter()
            .position(|c| c.card_id == "TEST-027")
            .expect("TEST-027 registered");
        let next = game.next_card_index();
        let perm = &mut game.players[atk.player as usize].battle_area[atk.index as usize];
        let mut src = CardSource::new(data_idx, atk.player, next);
        src.card_index = next;
        perm.card_sources.insert(0, src);
    }

    let result = r.attack_player(atk, 1, false);
    assert_eq!(
        result,
        AttackResult::SecurityCheckSurvived,
        "8000 DP attacker must survive after the -3000 adjustment drops security to 6000"
    );
    assert_eq!(r.battle_area_size(0), 1);
}

// ─── §2.5i: TriggerOrder suppression for single-source security ──────

/// A card with two `[Security]` effects from the same source must fire
/// both in collection order WITHOUT installing a `TriggerOrder` selection.
/// Observable: both effects run, and `pending_selection` is `None`
/// throughout resolution (attack returns a terminal outcome rather than
/// `InProgress`).
#[test]
fn two_security_effects_same_source_auto_fire_in_order() {
    let mut r = DebugRunner::builder()
        .add_card(attacker())
        .add_card(option("TEST-028"))
        .add_card(make_test_card("FILLER", "Filler"))
        .deck(1, &["FILLER"])
        .security(1, &["TEST-028"])
        .start();

    let atk = r.place_on_field(0, "ATK", Some(0));
    let hand_before = r.hand_size(1);
    let memory_before = r.memory();

    let result = r.attack_player(atk, 1, false);

    // Terminal outcome (not InProgress) — no prompt was installed.
    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert!(
        r.game.pending_selection.is_none(),
        "TriggerOrder prompt must NOT install for single-source security bundle"
    );
    // Both effects ran: +2 memory AND +1 hand card.
    assert_eq!(r.memory(), memory_before - 2);
    assert_eq!(r.hand_size(1), hand_before + 1);
}
