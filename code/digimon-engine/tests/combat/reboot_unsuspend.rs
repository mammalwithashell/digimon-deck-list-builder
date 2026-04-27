//! Phase 9 Task 7 — <Reboot> unsuspend-phase consumer.
//!
//! "At the start of your opponent's unsuspend phase, unsuspend this Digimon."
//! The keyword is parsed into `CardData.keywords` by Phase 3 printed-keyword
//! parsing (card_data.rs). Phase 9 Task 7 wires the runtime consumer into
//! `begin_turn`: during the Unsuspend step, after the turn player's bulk
//! unsuspend, the opposing player's battle area is scanned for Reboot Digimon
//! and they too are unsuspended.
//!
//! Spec §9.4.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, Keyword, ModifierType};
use digimon_engine::modifiers::ModifierEntry;

/// Digimon with an optional native printed keyword list.
fn digimon(id: &str, dp: i32, keywords: Vec<Keyword>) -> CardData {
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
        keywords,
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

#[test]
fn reboot_digimon_unsuspends_on_opponents_turn() {
    // P0 has a Digimon with <Reboot>. It starts suspended (as if from attacking
    // on P0's prior turn). When P1 begins a new turn, P1's unsuspend phase
    // runs — the Reboot scan should unsuspend P0's Digimon too.
    let mut r = DebugRunner::builder()
        .add_card(digimon("REBOOTER", 5000, vec![Keyword::Reboot]))
        .add_card(digimon("FILLER", 3000, vec![]))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;

    // Place Reboot Digimon on the opposing side (relative to current turn
    // player) so the upcoming end_turn swaps us into its controller's turn —
    // wait, we want the OTHER direction: Reboot is on P0, it unsuspends
    // when P1's turn begins. So place on `tp` (current turn player), suspend
    // it, then end the turn. After end_turn the new turn player is `opp`,
    // and the unsuspend step should have rebooted our Digimon on `tp`.
    let reb = r.place_on_field(tp, "REBOOTER", None);
    let _filler = r.place_on_field(opp, "FILLER", None);

    // Suspend the Reboot Digimon manually — simulates "attacked last turn".
    r.game.players[tp as usize].battle_area[reb.index as usize].is_suspended = true;
    assert!(
        r.game.players[tp as usize].battle_area[reb.index as usize].is_suspended,
        "precondition: Reboot Digimon is suspended"
    );

    // End turn — rotates to opponent, which runs their begin_turn /
    // unsuspend phase. The Reboot consumer must also unsuspend our Digimon.
    r.game.end_turn();

    // After the opponent's unsuspend phase, our Reboot Digimon should be
    // unsuspended even though it's not on the new turn player's side.
    assert_eq!(r.game.turn_player(), opp, "turn rotated to opponent");
    assert!(
        !r.game.players[tp as usize].battle_area[reb.index as usize].is_suspended,
        "Reboot Digimon unsuspends at start of opponent's unsuspend phase"
    );
}

#[test]
fn reboot_combined_with_overclock_composes() {
    // Overclock and Reboot are orthogonal keywords — a Digimon with both
    // should still have its Reboot consumer fire normally when the opponent
    // begins their turn. No interaction, just verify co-presence doesn't
    // break the unsuspend scan.
    let mut r = DebugRunner::builder()
        .add_card(digimon(
            "REB_OC",
            6000,
            vec![Keyword::Reboot, Keyword::Overclock],
        ))
        .add_card(digimon("PLAIN", 4000, vec![]))
        .add_card(digimon("FILLER", 3000, vec![]))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;

    // Both on tp's side; suspend both.
    let reb_oc = r.place_on_field(tp, "REB_OC", None);
    let plain = r.place_on_field(tp, "PLAIN", None);
    let _filler = r.place_on_field(opp, "FILLER", None);

    r.game.players[tp as usize].battle_area[reb_oc.index as usize].is_suspended = true;
    r.game.players[tp as usize].battle_area[plain.index as usize].is_suspended = true;

    r.game.end_turn();
    // Overclock parks `end_turn` in EndOfTurnAction waiting for the player
    // to declare an Overclock attack. Decline it so the turn actually
    // rotates to the opponent and their unsuspend phase runs.
    if r.game.current_phase == digimon_engine::enums::GamePhase::EndOfTurnAction {
        r.game.pass_end_of_turn_action();
    }

    assert_eq!(r.game.turn_player(), opp, "turn rotated to opponent");

    // Reboot Digimon unsuspends via the Reboot scan.
    assert!(
        !r.game.players[tp as usize].battle_area[reb_oc.index as usize].is_suspended,
        "Reboot+Overclock Digimon unsuspends via Reboot on opponent's turn"
    );
    // The plain suspended Digimon on tp's side stays suspended — Reboot is
    // per-keyword, not a bulk unsuspend.
    assert!(
        r.game.players[tp as usize].battle_area[plain.index as usize].is_suspended,
        "non-Reboot Digimon on tp's side stays suspended on opponent's turn"
    );
}

#[test]
fn cannot_be_unsuspended_by_effect_gates_reboot() {
    // A Reboot Digimon under the `CannotUnsuspend` modifier should NOT
    // unsuspend via the Reboot scan — Reboot is an effect-driven unsuspend
    // and is gated by the suspend-lock.
    //
    // (Phase 6 landed the `CannotUnsuspend` modifier enum; this test is the
    // first consumer that enforces it on the Reboot path. Other callers
    // will follow as Phase 9/10 wires up the keyword consumers.)
    let mut r = DebugRunner::builder()
        .add_card(digimon("LOCKED", 5000, vec![Keyword::Reboot]))
        .add_card(digimon("FILLER", 3000, vec![]))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let locked = r.place_on_field(tp, "LOCKED", None);
    let _filler = r.place_on_field(opp, "FILLER", None);

    r.game.players[tp as usize].battle_area[locked.index as usize].is_suspended = true;

    // Lock the Digimon with a CannotUnsuspend modifier (sourced from the
    // opponent, expiry doesn't matter for this turn's unsuspend phase).
    r.game.modifiers.add(
        locked,
        ModifierEntry::simple(ModifierType::CannotUnsuspend, 0, Expiry::Permanent, opp),
    );

    r.game.end_turn();

    assert_eq!(r.game.turn_player(), opp, "turn rotated to opponent");
    assert!(
        r.game.players[tp as usize].battle_area[locked.index as usize].is_suspended,
        "CannotUnsuspend gates the Reboot-driven unsuspend — Digimon stays suspended"
    );
}
