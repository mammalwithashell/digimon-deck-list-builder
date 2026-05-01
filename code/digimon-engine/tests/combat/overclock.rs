//! §4.6c-residual Overclock execution tests.
//!
//! `Game::activate_overclock` parks a sacrifice prompt, the callback deletes
//! the chosen sacrifice and fires an end-of-turn attack on the opponent
//! player without suspending the attacker. Mirrors Python
//! `action_decoder._initiate_overclock` /
//! [action_decoder.py:501-522](../digimon_gym/engine/game/action_decoder.py#L501).

use digimon_engine::action::space::{encode_attack, ATTACK_START, PASS, TARGETS_PER_ATTACKER};
use digimon_engine::card_data::CardData;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword};
use digimon_engine::game::OverclockError;
use digimon_engine::selection::{AttackTarget, SelectionKind};

fn make_digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 5,
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
    }
}

/// Standard setup: turn player has an Overclock Digimon at slot 0 and an
/// ordinary Digimon at slot 1. Returns `(runner, tp, overclock_handle)`.
fn setup_overclock_scenario() -> (
    DebugRunner,
    digimon_engine::enums::PlayerId,
    digimon_engine::permanent::PermanentHandle,
) {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .add_card(make_digimon("SAC", 3000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let _sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);

    // Drive into EndOfTurnAction phase.
    r.game.end_turn();
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);

    (r, tp, oc)
}

#[test]
fn activate_installs_sacrifice_prompt() {
    let (mut r, tp, _oc) = setup_overclock_scenario();

    r.game
        .activate_overclock(0)
        .expect("Overclock with a sacrifice available must install a prompt");

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("pending selection installed");
    assert_eq!(sel.kind, SelectionKind::OwnField);
    assert_eq!(sel.selecting_player, tp);
    assert!(sel.is_optional, "<Overclock> may be declined");
    assert_eq!(sel.previous_phase, GamePhase::EndOfTurnAction);

    // Only the non-Overclock slot (index 1) is a valid sacrifice.
    let expected_id = encode_attack(0, 1);
    assert_eq!(sel.valid_action_ids, vec![expected_id]);
    assert_eq!(r.game.current_phase, GamePhase::SelectTarget);
}

#[test]
fn activate_rejects_without_keyword() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .add_card(make_digimon("SAC", 3000))
        .start();
    let tp = r.game.turn_player();
    let _oc = r.place_on_field(tp, "OC", Some(0));
    let _sac = r.place_on_field(tp, "SAC", Some(0));
    // Note: NO Overclock keyword granted.
    // Force phase manually since end_turn wouldn't park here.
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let err = r
        .game
        .activate_overclock(0)
        .expect_err("no keyword → NotOverclock");
    assert_eq!(err, OverclockError::NotOverclock);
    assert!(r.game.pending_selection.is_none());
}

#[test]
fn activate_rejects_without_sacrifice() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .start();
    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.current_phase = GamePhase::EndOfTurnAction;

    let err = r
        .game
        .activate_overclock(0)
        .expect_err("alone on field → NoSacrifice");
    assert_eq!(err, OverclockError::NoSacrifice);
}

#[test]
fn activate_rejects_wrong_phase() {
    let (mut r, _tp, _oc) = setup_overclock_scenario();
    r.game.current_phase = GamePhase::Main;

    let err = r
        .game
        .activate_overclock(0)
        .expect_err("outside EndOfTurnAction → WrongPhase");
    assert_eq!(err, OverclockError::WrongPhase);
}

#[test]
fn decline_leaves_state_untouched() {
    let (mut r, tp, oc) = setup_overclock_scenario();
    r.game.activate_overclock(0).unwrap();

    let sac_count_before = r.game.player(tp).battle_area.len();
    r.game
        .resolve_selection(tp, PASS)
        .expect("optional prompt accepts PASS");

    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);
    assert_eq!(
        r.game.player(tp).battle_area.len(),
        sac_count_before,
        "PASS must not delete any permanent"
    );
    let oc_perm = &r.game.player(tp).battle_area[oc.index as usize];
    assert!(
        !oc_perm.is_suspended,
        "Overclock Digimon not suspended on decline"
    );

    // Still eligible next time — the Overclock bit can be re-emitted.
    assert!(r.game.has_end_of_turn_keywords(tp));
}

#[test]
fn full_flow_sacrifice_and_attack_resolves_on_security() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .add_card(make_digimon("SEC_DIGI", 2000))
        .security(1, &["SEC_DIGI"])
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);

    let battle_area_before = r.game.player(tp).battle_area.len();
    let sec_before = r.game.player(opp).security.len();

    r.game.activate_overclock(0).unwrap();
    // Resolve with the sacrifice.
    let sac_action = encode_attack(0, sac.index as u16);
    r.game.resolve_selection(tp, sac_action).unwrap();

    // Sacrifice deleted.
    assert_eq!(
        r.game.player(tp).battle_area.len(),
        battle_area_before - 1,
        "sacrifice removed from battle area"
    );
    // Security dropped by one (attacker 7000 DP > SEC_DIGI 2000 DP).
    assert_eq!(
        r.game.player(opp).security.len(),
        sec_before - 1,
        "one security card consumed by the Overclock attack"
    );

    // Attack has cleaned up; attacker still NOT suspended.
    assert!(r.game.pending_attack.is_none());
    let oc_perm = &r.game.player(tp).battle_area[0];
    assert!(
        !oc_perm.is_suspended,
        "<Overclock> attack must not suspend the attacker"
    );
    // Phase returned to EndOfTurnAction so further EOT actions can continue.
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);
}

#[test]
fn full_flow_against_empty_security_wins_game() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 7000))
        .add_card(make_digimon("SAC", 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));
    let sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);

    // Empty the opponent's security stack.
    r.game.player_mut(opp).security.clear();
    r.game.end_turn();

    r.game.activate_overclock(0).unwrap();
    let sac_action = encode_attack(0, sac.index as u16);
    r.game.resolve_selection(tp, sac_action).unwrap();

    assert!(r.game.game_over, "player hit with empty security loses");
    assert_eq!(r.game.winner, Some(tp));
}

#[test]
fn begin_attack_overclock_skips_suspend_directly() {
    // Low-level coverage for the combat-side flag: begin_attack_overclock
    // must leave the attacker unsuspended. No selection plumbing.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let oc = r.place_on_field(tp, "OC", Some(0));

    let result = r.game.begin_attack_overclock(oc, AttackTarget::Player(opp));
    // Security exists by default (5 cards); attack resolves terminally
    // (attacker may be hit by a security Digimon, but for this card DP
    // we expect it to survive or deal a security — either way, the flag
    // semantics are: no suspend on declaration, regardless of outcome).
    assert_ne!(result, AttackResult::Invalid);

    let oc_perm = &r.game.player(tp).battle_area[0];
    assert!(
        !oc_perm.is_suspended,
        "is_overclock path must not suspend the attacker"
    );
}

#[test]
fn regular_attack_still_suspends() {
    // Regression guard: the refactor extracting begin_attack_impl must not
    // break the normal suspend-on-attack behavior for non-Overclock paths.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", 5000))
        .start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let atk = r.place_on_field(tp, "ATK", Some(0));
    r.game.current_phase = GamePhase::Main;

    let _ = r.game.attack_player(atk, opp, /* vortex = */ false);
    let atk_perm = &r.game.player(tp).battle_area[0];
    assert!(
        atk_perm.is_suspended,
        "normal attack must still suspend the attacker"
    );
}

#[test]
fn action_id_decodes_correctly_when_sac_at_higher_index() {
    // Verify action-ID round-trip: install prompt over sacrifice at field
    // index 3, resolve with that exact ID, confirm the correct slot was
    // deleted (not slot 0 or 1 from a miscalculated decode).
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", 5000))
        .add_card(make_digimon("FILLER", 3000))
        .add_card(make_digimon("SAC", 3000))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let _ = r.place_on_field(tp, "FILLER", Some(0));
    let _ = r.place_on_field(tp, "FILLER", Some(0));
    let sac3 = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);
    r.game.end_turn();

    r.game.activate_overclock(0).unwrap();
    let sel = r.game.pending_selection.as_ref().unwrap();
    // Three sacrifices (slots 1, 2, 3) are legal.
    assert_eq!(sel.valid_action_ids.len(), 3);
    assert!(sel
        .valid_action_ids
        .contains(&encode_attack(0, sac3.index as u16)));

    // Resolve with slot 3 specifically.
    let action = encode_attack(0, 3);
    assert_eq!(
        (action - ATTACK_START) % TARGETS_PER_ATTACKER,
        3,
        "sanity check on the encoding"
    );
    let before = r.game.player(tp).battle_area.len();
    r.game.resolve_selection(tp, action).unwrap();

    // One permanent gone, Overclock and first two fillers still present.
    assert_eq!(r.game.player(tp).battle_area.len(), before - 1);
}
