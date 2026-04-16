//! Action mask building for the RL action space.
//!
//! Produces a `Vec<f32>` of size `ACTION_SPACE_SIZE` where 1.0 means legal,
//! 0.0 means illegal. Mask is phase-aware.
//!
//! Note: Phase 2 only implements basic Main/Breeding/Mulligan masking using
//! engine state available so far. Combat phases (BlockTiming, CounterTiming,
//! AllianceTiming) and effect-driven actions (effect activations, [Hand][Main],
//! DNA Digivolve, blast, etc.) are filled in later phases.

use crate::action::space::*;
use crate::card_data::CardData;
use crate::enums::{CardColor, CardKind, GamePhase, PlayerId};
use crate::game::Game;
use crate::tensor::FIELD_SLOTS;

fn evo_color(raw: u8) -> Option<CardColor> {
    match raw {
        0 => Some(CardColor::Red),
        1 => Some(CardColor::Blue),
        2 => Some(CardColor::Yellow),
        3 => Some(CardColor::Green),
        4 => Some(CardColor::Black),
        5 => Some(CardColor::Purple),
        6 => Some(CardColor::White),
        _ => None,
    }
}

/// Build an action mask of size `ACTION_SPACE_SIZE`.
/// Returns 1.0 for legal actions, 0.0 for illegal.
pub fn build_action_mask(game: &Game, player_id: PlayerId) -> Vec<f32> {
    let mut mask = vec![0.0f32; ACTION_SPACE_SIZE];
    let me = game.player(player_id);
    let opp_id = game.next_clockwise(player_id);
    let opp = game.player(opp_id);

    match game.current_phase {
        GamePhase::Mulligan => {
            // Mulligan is sequential: only the currently-deciding player has
            // a non-empty mask. Everyone else sees all zeros.
            if game.mulligan_current_player() != Some(player_id) {
                return mask;
            }
            // Bit 0 = keep (always available for the decider).
            mask[0] = 1.0;
            // Bit 1 = mulligan (one per player). Suppress if already used.
            if !game.mulligan_used[player_id as usize] {
                mask[1] = 1.0;
            }
        }

        GamePhase::Main => {
            // --- Play cards (0-29) ---
            let max_hand = (me.hand.len() as u16).min(PLAY_HAND_END);
            for i in 0..max_hand as usize {
                let card = &me.hand[i];
                let cost = card.play_cost(&game.card_data) as i16;
                // Memory check: card is affordable if memory - cost >= memory_min
                if (game.memory - cost) >= game.rules.memory_range.0 {
                    // Color requirement for Options is deferred to effect system.
                    mask[i] = 1.0;
                }
            }

            // --- Attack (100-399) ---
            // Basic rule: attacker must be unsuspended Digimon and memory >= 0.
            // Blitz/special rules deferred to effect system.
            if game.memory >= 0 {
                let max_field = me.battle_area.len().min(FIELD_SLOTS);
                for i in 0..max_field {
                    let attacker = &me.battle_area[i];
                    if !can_basic_attack(attacker, game.turn_count, &game.card_data) {
                        continue;
                    }

                    // Security attack
                    let sec_action =
                        encode_attack(i as u16, SECURITY_TARGET);
                    mask[sec_action as usize] = 1.0;

                    // Digimon attacks: only suspended targets by default
                    let max_opp = opp.battle_area.len().min(FIELD_SLOTS);
                    for j in 0..max_opp {
                        let target = &opp.battle_area[j];
                        if target.is_suspended && target.is_digimon(&game.card_data) {
                            mask[encode_attack(i as u16, j as u16) as usize] = 1.0;
                        }
                    }
                }
            }

            // --- Digivolve (400-999) ---
            // Basic check: card in hand is Digimon and matching evo_costs.
            // Full digivolve validation (alt-digi, modifiers) deferred.
            for h in 0..max_hand as usize {
                let card = &me.hand[h];
                if card.card_kind(&game.card_data) != CardKind::Digimon {
                    continue;
                }
                let max_field = me.battle_area.len().min(FIELD_SLOTS);
                for f in 0..max_field {
                    let base_perm = &me.battle_area[f];
                    if can_basic_digivolve(card, base_perm, &game.card_data) {
                        mask[encode_digivolve(h as u16, f as u16) as usize] = 1.0;
                    }
                }
                // Breeding-area digivolve
                if let Some(ref breeding) = me.breeding_area {
                    if can_basic_digivolve(card, breeding, &game.card_data) {
                        mask[encode_digivolve(h as u16, BREEDING_TARGET) as usize] = 1.0;
                    }
                }
            }

            // --- Pass (62) ---
            mask[PASS as usize] = 1.0;
        }

        GamePhase::Breeding => {
            // Hatch (60)
            if me.breeding_area.is_none() && !me.digitama_deck.is_empty() {
                mask[HATCH as usize] = 1.0;
            }
            // Move from breeding (61): requires Digimon at level >= 3
            if let Some(ref perm) = me.breeding_area {
                if perm.level(&game.card_data).unwrap_or(0) >= 3 {
                    mask[MOVE_FROM_BREEDING as usize] = 1.0;
                }
            }
            // Pass (62)
            mask[PASS as usize] = 1.0;
        }

        // Selection / combat phases require effect system + pending_selection support.
        // Defer to later phases. For now, allow pass to avoid soft-locking.
        _ => {
            mask[PASS as usize] = 1.0;
        }
    }

    mask
}

// ─── Helpers ──────────────────────────────────────────────────────────

/// Basic attack eligibility: unsuspended Digimon not played this turn.
/// Full keyword/modifier checks deferred.
fn can_basic_attack(
    perm: &crate::permanent::Permanent,
    turn: u16,
    card_data: &[CardData],
) -> bool {
    if perm.is_suspended {
        return false;
    }
    if !perm.is_digimon(card_data) {
        return false;
    }
    // Summoning sickness: can't attack the turn it was played, unless Rush
    // (Rush handling deferred to keyword system).
    if perm.turn_played == turn && perm.turn_digivolved != turn {
        return false;
    }
    true
}

/// Basic digivolve eligibility: hand card has an evo_cost matching the base
/// permanent's color and level. Full validation (alt-digi, color modifiers,
/// CANNOT_DIGIVOLVE) deferred.
fn can_basic_digivolve(
    card: &crate::card_source::CardSource,
    base: &crate::permanent::Permanent,
    card_data: &[CardData],
) -> bool {
    let card_meta = &card_data[card.data_index];
    let base_top = base.top_card();
    let base_meta = &card_data[base_top.data_index];

    // Base must be Digimon or DigiEgg
    if base_meta.card_kind != CardKind::Digimon && base_meta.card_kind != CardKind::DigiEgg {
        return false;
    }

    let base_level = match base_meta.level {
        Some(l) => l,
        None => return false,
    };

    // Find a matching evo_cost
    for evo in &card_meta.evo_costs {
        if evo.level != base_level {
            continue;
        }
        let color = match evo_color(evo.card_color) {
            Some(c) => c,
            None => continue,
        };
        if !base_meta.colors.contains(&color) {
            continue;
        }
        return true;
    }
    false
}
