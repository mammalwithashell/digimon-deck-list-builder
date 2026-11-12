//! Chapter 6 "Game Progression" (Comprehensive Rules Ver. 4.3): turn end
//! conditions.
//!
//! Every test asserts the RULE, never the engine's current output.
//!
//! Found by the DCGO playback of a sim game (2026-11-11): a breeding-phase
//! effect paid memory onto the opponent's side, and the engine still began the
//! main phase, where DCGO ended the turn.

use crate::fixtures;
use crate::support::*;
use digimon_engine::action::space::{MOVE_FROM_BREEDING, PASS};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::GamePhase;

/// P0's turn, in its breeding phase, with `memory` on P0's side (negative is
/// the opponent's side) and a Digimon in the breeding area.
fn breeding_board(memory: i16) -> DebugRunner {
    let mut r = fixtures::builder(&[])
        .deck(0, &[FILL; 6])
        .deck(1, &[FILL; 6])
        .memory(memory)
        .start();
    let staged = r.place_stack(0, &[FILL]);
    let perm = r.game.players[0]
        .battle_area
        .remove(staged.index as usize);
    r.game.players[0].breeding_area = Some(perm);
    r.set_phase(GamePhase::Breeding);
    r
}

/// §6-1-4-1 "The turn end conditions are met when the memory is at 1 or more
/// on your opponent's side and all processing has been resolved for the
/// current phase. Then, the turn will end with the current phase."
///
/// The condition is checked at the end of the breeding phase too: with memory
/// already on P1's side, ending the breeding phase (by passing, or by moving
/// the breeding Digimon out) ends P0's turn and no main phase begins. With
/// memory at 0 the main phase begins as usual.
#[test]
fn turn_ends_with_the_breeding_phase_when_memory_is_on_the_opponents_side() {
    cite!(
        "6-1-4-1",
        "bbff17a7df9616ebc2c30e09a4806ae5082d71105a436fa4457e3af41a070d77"
    );
    for action in [PASS, MOVE_FROM_BREEDING] {
        let mut r = breeding_board(-1);
        r.game.decode_action(action, 0);
        assert_ne!(
            (r.turn_player(), r.current_phase()),
            (0, GamePhase::Main),
            "action {action}: memory on P1's side, so P0's turn ends with the breeding phase"
        );
        assert_eq!(r.turn_player(), 1, "action {action}: P1's turn began");

        let mut r = breeding_board(0);
        r.game.decode_action(action, 0);
        assert_eq!(
            (r.turn_player(), r.current_phase()),
            (0, GamePhase::Main),
            "action {action}: memory 0 is not on P1's side, so P0's main phase begins"
        );
    }
}
