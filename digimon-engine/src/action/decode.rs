//! Action-ID dispatcher — digivolve slice of the spec §3.2 decoder.
//!
//! Only the basic-digivolve (`DIGIVOLVE_START..DIGIVOLVE_END`) and
//! DNA-digivolve (`DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END`) bands are
//! routed today. Other bands (play-from-hand, attacks, hatch, breeding
//! movement, hand/field/trash effect activations, source selection,
//! pass) land in follow-up slices and currently fall through to `false`.

use crate::action::space::*;
use crate::enums::PlayerId;
use crate::game::Game;

impl Game {
    /// Dispatch an integer action ID to the matching high-level Game
    /// method. Returns `true` if the action was accepted and executed,
    /// `false` if rejected (bad range, guard failed, or the band is not
    /// yet wired).
    pub fn decode_action(&mut self, action_id: u16, player_id: PlayerId) -> bool {
        match action_id {
            a if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&a) => {
                let (hand, field) = decode_digivolve(a);
                if field == BREEDING_TARGET {
                    self.digivolve_from_hand_onto_breeding(player_id, hand as usize)
                } else {
                    self.digivolve_from_hand(player_id, hand as usize, field as usize)
                }
            }
            a if (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&a) => {
                let hand = (a - DNA_DIGIVOLVE_START) as usize;
                self.initiate_dna_digivolve(player_id, hand)
            }
            // TODO(§3.2): PLAY_HAND, ATTACK, HATCH, MOVE_FROM_BREEDING,
            // HAND/FIELD/TRASH_EFFECT, SOURCE_SELECT, PASS — future slices.
            _ => false,
        }
    }
}
