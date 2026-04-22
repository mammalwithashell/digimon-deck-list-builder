//! §4.6b end-of-turn phase transition tests.
//!
//! `Game::end_turn` must park in `GamePhase::EndOfTurnAction` when the ending
//! player has a permanent with a pending end-of-turn keyword action
//! (Vortex / Overclock / MayAttack). Turn rotation is deferred until the
//! player resumes via `pass_end_of_turn_action`.
//!
//! Mirrors Python `_complete_end_phase` and `next_phase` branches at
//! [digimon_gym/engine/game/__init__.py:294-324]. ForceAttack is NOT part
//! of the EOT-park check — it's enforced at the Main-phase mask (§4.7d).

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, GamePhase, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

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

// ─── Parking cases ────────────────────────────────────────────────────

#[test]
fn end_turn_parks_in_eot_phase_when_vortex_present() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);

    let turn_before = r.game.turn_count;
    r.game.end_turn();

    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);
    assert_eq!(r.game.turn_player(), tp, "turn must not rotate yet");
    assert_eq!(r.game.turn_count, turn_before, "turn_count must not advance");
}

#[test]
fn end_turn_parks_when_overclock_has_sacrifice() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", CardColor::Red))
        .add_card(make_digimon("SAC", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    let _sac = r.place_on_field(tp, "SAC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);

    r.game.end_turn();
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);
}

#[test]
fn end_turn_does_not_park_when_overclock_has_no_sacrifice() {
    // Overclock Digimon alone — no other permanent to sacrifice.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("OC", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let oc = r.place_on_field(tp, "OC", Some(0));
    r.game
        .modifiers
        .grant_keyword(oc, Keyword::Overclock, Expiry::EndOfTurn, tp);

    let opp = 1 - tp;
    r.game.end_turn();

    assert_ne!(
        r.game.current_phase,
        GamePhase::EndOfTurnAction,
        "Overclock with no sacrifice available must NOT park — behaves like a vanilla turn end"
    );
    assert_eq!(r.game.turn_player(), opp, "turn must rotate normally");
}

#[test]
fn end_turn_parks_when_may_attack_granted() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );

    r.game.end_turn();
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);
}

#[test]
fn end_turn_does_not_park_when_may_attack_cannot_attack() {
    // MayAttack granted to a permanent that can't actually attack (suspended).
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game.players[tp as usize].battle_area[attacker.index as usize].is_suspended = true;
    r.game.modifiers.add(
        attacker,
        ModifierEntry::simple(ModifierType::MayAttack, 1, Expiry::EndOfTurn, tp),
    );

    r.game.end_turn();
    assert_ne!(
        r.game.current_phase,
        GamePhase::EndOfTurnAction,
        "suspended MayAttack permanent has no legal attack — must not park",
    );
}

#[test]
fn end_turn_rotates_through_when_no_eot_keywords() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .start();
    let tp = r.game.turn_player();
    let opp = 1 - tp;
    r.place_on_field(tp, "ATK", Some(0));

    let turn_before = r.game.turn_count;
    r.game.end_turn();

    assert_eq!(r.game.turn_player(), opp, "rotation must happen with no EOT keywords");
    assert_eq!(r.game.turn_count, turn_before + 1);
    assert_ne!(r.game.current_phase, GamePhase::EndOfTurnAction);
}

// ─── Resumption ───────────────────────────────────────────────────────

#[test]
fn pass_end_of_turn_action_rotates_turn() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);

    let memory_before = r.game.memory;
    let turn_before = r.game.turn_count;
    r.game.end_turn();
    assert_eq!(r.game.current_phase, GamePhase::EndOfTurnAction);

    r.game.pass_end_of_turn_action();

    assert_eq!(r.game.turn_player(), opp, "turn must rotate on pass");
    assert_eq!(r.game.turn_count, turn_before + 1);
    assert_eq!(
        r.game.memory, -memory_before,
        "seesaw must flip during rotation",
    );
    assert_ne!(r.game.current_phase, GamePhase::EndOfTurnAction);
}

#[test]
fn pass_end_of_turn_action_is_noop_outside_eot() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .start();

    let tp_before = r.game.turn_player();
    let turn_before = r.game.turn_count;
    let phase_before = r.game.current_phase;

    r.game.pass_end_of_turn_action();

    assert_eq!(r.game.turn_player(), tp_before);
    assert_eq!(r.game.turn_count, turn_before);
    assert_eq!(r.game.current_phase, phase_before);
}

#[test]
fn pass_end_of_turn_action_expires_modifiers() {
    // The rotation must still expire EndOfTurn modifiers — moving the
    // bookkeeping from end_turn into rotate_turn_player must not lose it.
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATK", CardColor::Red))
        .start();

    let tp = r.game.turn_player();
    let attacker = r.place_on_field(tp, "ATK", Some(0));
    r.game
        .modifiers
        .grant_keyword(attacker, Keyword::Vortex, Expiry::EndOfTurn, tp);

    r.game.end_turn();
    assert!(
        r.game.modifiers.has_keyword(attacker, Keyword::Vortex),
        "modifiers should still be live while parked in EOT",
    );

    r.game.pass_end_of_turn_action();
    assert!(
        !r.game.modifiers.has_keyword(attacker, Keyword::Vortex),
        "EndOfTurn modifiers must expire after pass_end_of_turn_action",
    );
}
