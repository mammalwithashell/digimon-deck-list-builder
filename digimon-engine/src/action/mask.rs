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
use crate::effect_context::EffectReadContext;
use crate::enums::{CardColor, CardKind, EffectTiming, GamePhase, Keyword, ModifierType, PlayerId};
use crate::game::Game;
use crate::modifiers::ModifierRegistry;
use crate::permanent::PermanentHandle;
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
            // §4.7c CANNOT_PLAY_FROM_HAND — any active modifier of this
            // type (anywhere in the registry) suppresses every hand-play
            // bit. Python's context discriminant (the specific `card`
            // argument) isn't carried in Rust — tracked as §4.7x.
            let play_blocked = game
                .modifiers
                .any_with_type(ModifierType::CannotPlayFromHand);
            for i in 0..max_hand as usize {
                if play_blocked {
                    continue;
                }
                let card = &me.hand[i];
                let cost = card.play_cost(&game.card_data) as i16;
                // Memory check: card is affordable if memory - cost >= memory_min
                if (game.memory - cost) < game.rules.memory_range.0 {
                    continue;
                }
                // §4.2 Option color requirement: an Option is only playable
                // when the player has a Digimon or Tamer of a matching color
                // on the field or in the breeding area.
                if card.card_kind(&game.card_data) == CardKind::Option {
                    if !option_color_match_available(card, me, &game.card_data) {
                        continue;
                    }
                }
                mask[i] = 1.0;
            }

            // --- Attack (100-399) ---
            // Memory gate is per-attacker: baseline requires memory >= 0,
            // but §4.3 Blitz carves out "Blitz + digivolved this turn" even
            // when memory < 0. Native/static Blitz parsing remains §4.3b.
            let max_field = me.battle_area.len().min(FIELD_SLOTS);
            for i in 0..max_field {
                let attacker = &me.battle_area[i];
                let handle = PermanentHandle { player: player_id, index: i as u8 };
                if !can_basic_attack(
                    attacker,
                    handle,
                    game.turn_count,
                    &game.card_data,
                    &game.modifiers,
                ) {
                    continue;
                }
                let memory_ok = game.memory >= 0 || {
                    let digivolved_this_turn =
                        attacker.turn_digivolved == game.turn_count;
                    digivolved_this_turn
                        && game.modifiers.has_keyword(handle, Keyword::Blitz)
                };
                if !memory_ok {
                    continue;
                }

                // Security attack.
                let sec_action = encode_attack(i as u16, SECURITY_TARGET);
                mask[sec_action as usize] = 1.0;

                // Digimon attacks (§4.4). Suspended enemies are always valid.
                // Unsuspended enemies are valid iff attacker has:
                //   - CAN_ATTACK_UNSUSPENDED modifier (any unsuspended), or
                //   - Raid keyword (unsuspended enemies tied for highest
                //     effective DP).
                // Native `_is_can_attack_unsuspended` keyword awaits §4.5.
                let can_attack_unsuspended = game
                    .modifiers
                    .has(handle, ModifierType::CanAttackUnsuspended);
                let has_raid = game.modifiers.has_keyword(handle, Keyword::Raid);
                let max_opp = opp.battle_area.len().min(FIELD_SLOTS);

                // Precompute max effective DP among unsuspended enemy Digimon
                // only if Raid is relevant (skip when CAN_ATTACK_UNSUSPENDED
                // already broadens targeting).
                let raid_max_dp = if has_raid && !can_attack_unsuspended {
                    let mut best: Option<i32> = None;
                    for j in 0..max_opp {
                        let t = &opp.battle_area[j];
                        if t.is_suspended || !t.is_digimon(&game.card_data) {
                            continue;
                        }
                        let t_handle = PermanentHandle {
                            player: opp_id,
                            index: j as u8,
                        };
                        if let Some(dp) = game.effective_dp(t_handle) {
                            best = Some(best.map_or(dp, |b| b.max(dp)));
                        }
                    }
                    best
                } else {
                    None
                };

                for j in 0..max_opp {
                    let target = &opp.battle_area[j];
                    if !target.is_digimon(&game.card_data) {
                        continue;
                    }
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    // §4.7a CANNOT_ATTACK_TARGET — suppress this target if
                    // it carries the modifier. Per-attacker discriminant
                    // from Python is §4.7x.
                    if game
                        .modifiers
                        .has(t_handle, ModifierType::CannotAttackTarget)
                    {
                        continue;
                    }
                    let action_bit = encode_attack(i as u16, j as u16) as usize;
                    if target.is_suspended {
                        mask[action_bit] = 1.0;
                        continue;
                    }
                    if can_attack_unsuspended {
                        mask[action_bit] = 1.0;
                        continue;
                    }
                    if let Some(max_dp) = raid_max_dp {
                        if let Some(dp) = game.effective_dp(t_handle) {
                            if dp == max_dp {
                                mask[action_bit] = 1.0;
                            }
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
                    let base_handle = PermanentHandle { player: player_id, index: f as u8 };
                    // §4.7b CANNOT_DIGIVOLVE — suppress the bit if the
                    // base permanent carries an active CannotDigivolve
                    // modifier. Python's `{'digivolving_card': card}`
                    // discriminant is not carried in Rust (§4.7x).
                    if game
                        .modifiers
                        .has(base_handle, ModifierType::CannotDigivolve)
                    {
                        continue;
                    }
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

            // --- DNA Digivolve (63-92) --- §4.5 slice.
            // A hand Digimon with non-empty dna_costs is playable if some
            // pair of permanents in battle_area satisfies any of its DnaCost
            // entries (either ordering). Python's mask does NOT gate on
            // memory (action_mask.py:161-166) — the memory check runs at
            // action-execution time, not mask generation. Mirror that here.
            // Data population (cards.json ingest of dna_costs) is §4.5b.
            for h in 0..max_hand as usize {
                let card = &me.hand[h];
                let evo_meta = &game.card_data[card.data_index];
                if evo_meta.dna_costs.is_empty() {
                    continue;
                }
                if crate::dna_digivolve::has_valid_dna_targets(
                    evo_meta,
                    &me.battle_area,
                    &game.card_data,
                ) {
                    mask[(DNA_DIGIVOLVE_START + h as u16) as usize] = 1.0;
                }
            }

            // --- [Main] activated effects (§4.5c) ---
            //
            // Python iterates each card's effect list and filters by the
            // `_is_{hand,field,trash}_main` bool flag. Rust promotes the
            // zone distinction into the timing enum itself, so filtering
            // is a single `effect.timing ==` check.
            //
            // OPT enforcement:
            //   * Field — use the existing per-permanent
            //     `activation_count((handle, slot))` map (already populated
            //     by effect firing at runtime).
            //   * Hand / Trash — Python's mask does NOT check
            //     `_turn_activate_count` either; the effect's
            //     `can_use_condition` closure is responsible. Mirror that
            //     here. Adding explicit hand/trash activation maps is a
            //     follow-up tracked in §4.5c residuals.

            // Hand [Main] (bits 30-59). One bit per hand slot, mirroring
            // Python's `action_mask.py:176-185`. First-match-wins per slot.
            let hand_limit = me.hand.len().min(HAND_MAIN_LIMIT);
            for h in 0..hand_limit {
                let card = &me.hand[h];
                let card_id = card.card_id(&game.card_data).to_string();
                let effects = game.effects_for_card(&card_id, card.handle());
                for effect in &effects {
                    if effect.timing != EffectTiming::MainFromHand {
                        continue;
                    }
                    if let Some(cond) = &effect.condition {
                        let ctx = EffectReadContext::new(
                            game,
                            card.handle(),
                            None,
                            player_id,
                        );
                        if !cond(&ctx) {
                            continue;
                        }
                    }
                    mask[(HAND_EFFECT_START + h as u16) as usize] = 1.0;
                    break;
                }
            }

            // Field [Main] (bits 1000-1149). Python emits one bit per
            // permanent at sub-slot `+2` (FIELD_EFFECT_SLOT_FOR_MAIN),
            // first-match-wins across the entire digivolution stack.
            // Inherited-vs-top filter matches `source_dp_contribution`.
            let field_limit = me.battle_area.len().min(FIELD_SLOTS);
            for i in 0..field_limit {
                let perm = &me.battle_area[i];
                let perm_handle = PermanentHandle {
                    player: player_id,
                    index: i as u8,
                };
                let stack_size = perm.card_sources.len();
                let mut emitted = false;
                for (source_index, source) in perm.card_sources.iter().enumerate() {
                    if emitted {
                        break;
                    }
                    let is_under = source_index + 1 < stack_size;
                    let card_id = source.card_id(&game.card_data).to_string();
                    let effects = game.effects_for_card(&card_id, source.handle());
                    for (slot, effect) in effects.iter().enumerate() {
                        if effect.timing != EffectTiming::MainOnField {
                            continue;
                        }
                        if is_under != effect.inherited {
                            continue;
                        }
                        if effect.max_per_turn > 0
                            && perm.activation_count(source.handle(), slot as u8)
                                >= effect.max_per_turn
                        {
                            continue;
                        }
                        if let Some(cond) = &effect.condition {
                            let ctx = EffectReadContext::new(
                                game,
                                source.handle(),
                                Some(perm_handle),
                                player_id,
                            );
                            if !cond(&ctx) {
                                continue;
                            }
                        }
                        let bit = FIELD_EFFECT_START
                            + i as u16 * EFFECTS_PER_PERMANENT
                            + FIELD_EFFECT_SLOT_FOR_MAIN;
                        mask[bit as usize] = 1.0;
                        emitted = true;
                        break;
                    }
                }
            }

            // Trash [Main] (bits 1150-1194). One bit per trash slot,
            // first-match-wins, mirroring `action_mask.py:216-225`.
            let trash_limit = me.trash.len().min(TRASH_MAIN_LIMIT);
            for t in 0..trash_limit {
                let card = &me.trash[t];
                let card_id = card.card_id(&game.card_data).to_string();
                let effects = game.effects_for_card(&card_id, card.handle());
                for effect in &effects {
                    if effect.timing != EffectTiming::MainFromTrash {
                        continue;
                    }
                    if let Some(cond) = &effect.condition {
                        let ctx = EffectReadContext::new(
                            game,
                            card.handle(),
                            None,
                            player_id,
                        );
                        if !cond(&ctx) {
                            continue;
                        }
                    }
                    mask[(TRASH_EFFECT_START + t as u16) as usize] = 1.0;
                    break;
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

        GamePhase::EndOfTurnAction => {
            // Decline end-of-turn action — always legal.
            mask[PASS as usize] = 1.0;
            // §4.6 slice — Vortex. A permanent with modifier-granted
            // Keyword::Vortex whose `can_attack(handle, vortex=true)` is
            // true may attack any enemy Digimon (suspended or not).
            // Mirrors Python action_mask.py:321-335. Overclock /
            // MAY_ATTACK / FORCE_ATTACK bits are tracked as §4.6c.
            let max_field = me.battle_area.len().min(FIELD_SLOTS);
            for i in 0..max_field {
                let handle = PermanentHandle { player: player_id, index: i as u8 };
                if !game.modifiers.has_keyword(handle, Keyword::Vortex) {
                    continue;
                }
                if !game.can_attack(handle, /* vortex = */ true) {
                    continue;
                }
                mask[encode_attack(i as u16, SECURITY_TARGET) as usize] = 1.0;
                let max_opp = opp.battle_area.len().min(FIELD_SLOTS);
                for j in 0..max_opp {
                    let target = &opp.battle_area[j];
                    if !target.is_digimon(&game.card_data) {
                        continue;
                    }
                    let t_handle = PermanentHandle {
                        player: opp_id,
                        index: j as u8,
                    };
                    // §4.7a CANNOT_ATTACK_TARGET — suppress Vortex attacks
                    // against a target carrying the modifier.
                    if game
                        .modifiers
                        .has(t_handle, ModifierType::CannotAttackTarget)
                    {
                        continue;
                    }
                    mask[encode_attack(i as u16, j as u16) as usize] = 1.0;
                }
            }
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

/// §4.2 — Check that the player has at least one Digimon or Tamer of a
/// color matching any of the Option card's colors, either on the battle
/// area or in the breeding area.
///
/// Mirror of Python's `action_mask.py` lines 77-99. Script-level
/// `match_color_requirement=False` and the `IGNORE_COLOR_REQUIREMENT` aura
/// modifier are residual §4.2b work; both are absent here.
fn option_color_match_available(
    card: &crate::card_source::CardSource,
    me: &crate::player::Player,
    card_data: &[crate::card_data::CardData],
) -> bool {
    let option_colors = card.colors(card_data);

    // Battle area: Digimon or Tamer with an overlapping color.
    for perm in &me.battle_area {
        if !perm.is_digimon(card_data) && !perm.is_tamer(card_data) {
            continue;
        }
        let perm_colors = perm.top_card().colors(card_data);
        if option_colors.iter().any(|c| perm_colors.contains(c)) {
            return true;
        }
    }

    // Breeding area: only Digimon counts (Tamers can't be in breeding).
    if let Some(ref breeding) = me.breeding_area {
        if breeding.is_digimon(card_data) {
            let perm_colors = breeding.top_card().colors(card_data);
            if option_colors.iter().any(|c| perm_colors.contains(c)) {
                return true;
            }
        }
    }

    false
}

/// Basic attack eligibility: unsuspended Digimon not played this turn,
/// unless modifier-granted Rush exempts summoning sickness.
///
/// Vortex is not checked here — Vortex attacks belong to `EndOfTurnAction`
/// phase mask generation (§4.6), not the Main-phase attack range.
fn can_basic_attack(
    perm: &crate::permanent::Permanent,
    handle: PermanentHandle,
    turn: u16,
    card_data: &[CardData],
    modifiers: &ModifierRegistry,
) -> bool {
    if perm.is_suspended {
        return false;
    }
    if !perm.is_digimon(card_data) {
        return false;
    }
    // Summoning sickness: can't attack the turn it was played unless Rush
    // has been granted (modifier-granted only; native/static Rush pending
    // §2.1b).
    let is_fresh = perm.turn_played == turn && perm.turn_digivolved != turn;
    if is_fresh && !modifiers.has_keyword(handle, Keyword::Rush) {
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
