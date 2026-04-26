//! Heuristic greedy opponent — port of Python's `greedy_policy()` in
//! `digimon_gym/digimon_gym.py`.
//!
//! Priorities:
//! - Mulligan: keep if hand contains a level-3 Digimon, else mulligan.
//! - Trash-from-hand selection: discard the lowest-value card (prefer
//!   non-Digimon, then lower cost).
//! - Breeding phase: Hatch > Move > Pass.
//! - Main phase: best keep-turn Digivolve > best Attack > best Play > Pass.
//!
//! The heuristic inspects the full `Game` state rather than the obs
//! tensor, so it's cheap and deterministic. Tie-breaks are deterministic
//! (by level, DP, cost, index) to match the Python reference.
//!
//! Replaces the `first_valid_action` placeholder previously wired into
//! `PlayerKind::Greedy` (see commit 4ab3c87a). Parity hazard: any change
//! to `greedy_policy()` in Python must be mirrored here — tracked in
//! `docs/RUST_PYTHON_PARITY.md` under "Policies".

use crate::action::space::{
    ATTACK_END, ATTACK_START, BREEDING_TARGET, DIGIVOLVE_END, DIGIVOLVE_START,
    FIELDS_PER_HAND, HATCH, MOVE_FROM_BREEDING, PASS, PLAY_HAND_END, PLAY_HAND_START,
    SECURITY_TARGET, TARGETS_PER_ATTACKER,
};
use crate::card_data::CardData;
use crate::enums::{CardKind, GamePhase, PlayerId};
use crate::game::Game;
use crate::selection::SelectionKind;

/// Pick the next action for a `PlayerKind::Greedy` seat, using the
/// Python-parity heuristic. The returned id is guaranteed to be a valid
/// entry in the provided mask (or `PASS` if the mask is empty).
pub fn greedy_action(game: &Game, mask: &[f32]) -> u16 {
    let valid: Vec<u16> = mask
        .iter()
        .enumerate()
        .filter_map(|(i, v)| if *v > 0.0 { Some(i as u16) } else { None })
        .collect();

    if valid.is_empty() {
        return PASS;
    }

    // Mulligan phase — the deciding player is set by `mulligan_current_player`.
    if game.current_phase == GamePhase::Mulligan {
        return mulligan_choice(game, &valid);
    }

    // Selection-time: if the engine is asking for a hand card to trash,
    // pick the lowest-value card rather than the first.
    if let Some(sel) = game.pending_selection.as_ref() {
        if sel.kind == SelectionKind::Trash {
            if let Some(a) = best_trash_choice(game, sel.selecting_player, &valid) {
                return a;
            }
        }
    }

    let player_id = current_decider(game);

    match game.current_phase {
        GamePhase::Breeding => {
            if valid.contains(&HATCH) {
                return HATCH;
            }
            if valid.contains(&MOVE_FROM_BREEDING) {
                return MOVE_FROM_BREEDING;
            }
            if valid.contains(&PASS) {
                return PASS;
            }
            first_non_pass(&valid)
        }
        GamePhase::Main => {
            if let Some(a) = best_keep_turn_digivolve(game, player_id, &valid) {
                return a;
            }
            if let Some(a) = best_attack(game, player_id, &valid) {
                return a;
            }
            if let Some(a) = best_play(game, player_id, &valid) {
                return a;
            }
            if valid.contains(&PASS) {
                return PASS;
            }
            first_non_pass(&valid)
        }
        _ => {
            if valid.contains(&PASS) {
                return PASS;
            }
            first_non_pass(&valid)
        }
    }
}

/// Who is expected to submit the next action — mulligan decider > pending
/// selection holder > turn player. Mirrors
/// `engine_commands.rs::current_decision_player` exactly so the heuristic
/// reasons about the same seat the engine is asking.
fn current_decider(game: &Game) -> PlayerId {
    if let Some(p) = game.mulligan_current_player() {
        return p;
    }
    if let Some(sel) = game.pending_selection.as_ref() {
        return sel.selecting_player;
    }
    game.turn_player()
}

fn first_non_pass(valid: &[u16]) -> u16 {
    for &a in valid {
        if a != PASS {
            return a;
        }
    }
    PASS
}

/// Mulligan heuristic: keep (action 0) if hand has a level-3 Digimon,
/// else mulligan (action 1). Matches Python's mulligan branch
/// (`digimon_gym.py` around the `ACTION_PASS_TURN` mulligan block).
fn mulligan_choice(game: &Game, valid: &[u16]) -> u16 {
    let pid = game.mulligan_current_player().unwrap_or(0);
    let has_level3 = game
        .player(pid)
        .hand
        .iter()
        .any(|c| c.is_digimon(&game.card_data) && c.level(&game.card_data) == Some(3));

    if !has_level3 && valid.contains(&1) {
        return 1; // mulligan
    }
    if valid.contains(&0) {
        return 0; // keep
    }
    valid[0]
}

/// Active-player-relative memory. Positive = the decider has memory to
/// spend. Python: `int(game.memory) if turn_player is player1 else -game.memory`.
fn relative_memory(game: &Game, pid: PlayerId) -> i32 {
    if pid == 0 {
        game.memory as i32
    } else {
        -(game.memory as i32)
    }
}

/// Estimate the memory cost to digivolve `hand_card` onto `base_perm`.
/// Matches Python's `_estimate_digivolve_cost`.
fn estimate_digivolve_cost(
    hand_card: &crate::card_source::CardSource,
    base_perm: &crate::permanent::Permanent,
    data: &[CardData],
) -> i32 {
    let fallback_cost = hand_card.play_cost(data) as i32;
    let entry = &data[hand_card.data_index];

    if entry.evo_costs.is_empty() {
        return fallback_cost;
    }

    let base_level = match base_perm.level(data) {
        Some(l) => l,
        None => return fallback_cost,
    };
    let base_colors = base_perm.top_card().colors(data);

    let mut best: Option<i32> = None;
    for evo in &entry.evo_costs {
        if evo.level != base_level {
            continue;
        }
        let matches_color = base_colors.iter().any(|c| (*c as u8) == evo.card_color);
        if !matches_color {
            continue;
        }
        let cost = evo.memory_cost as i32;
        best = Some(best.map_or(cost, |b| b.min(cost)));
    }
    best.unwrap_or(fallback_cost)
}

/// Best digivolve that keeps the turn (relative memory stays ≥ 0). None if
/// no valid digivolve in `valid` meets the memory constraint.
fn best_keep_turn_digivolve(game: &Game, pid: PlayerId, valid: &[u16]) -> Option<u16> {
    let player = game.player(pid);
    let rel_mem = relative_memory(game, pid);
    // Score tuple: (level, base_dp, -cost, -hand_idx, -field_idx) — higher
    // level/DP preferred, lower cost preferred, lower indices preferred.
    let mut best: Option<((i32, i32, i32, i32, i32), u16)> = None;

    for &action in valid {
        if !(DIGIVOLVE_START..=DIGIVOLVE_END).contains(&action) {
            continue;
        }
        let offset = action - DIGIVOLVE_START;
        let hand_idx = offset / FIELDS_PER_HAND;
        let field_idx = offset % FIELDS_PER_HAND;

        if hand_idx as usize >= player.hand.len() {
            continue;
        }
        let card = &player.hand[hand_idx as usize];

        let base_perm = if (field_idx as usize) < player.battle_area.len() {
            &player.battle_area[field_idx as usize]
        } else if field_idx == BREEDING_TARGET {
            match &player.breeding_area {
                Some(b) => b,
                None => continue,
            }
        } else {
            continue;
        };

        let cost = estimate_digivolve_cost(card, base_perm, &game.card_data);
        if rel_mem - cost < 0 {
            continue;
        }

        let level = card.level(&game.card_data).unwrap_or(0) as i32;
        let dp = card.dp(&game.card_data).unwrap_or(0);
        let score = (level, dp, -cost, -(hand_idx as i32), -(field_idx as i32));
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, action));
        }
    }
    best.map(|(_, a)| a)
}

/// Best attack: prioritize lethal security hits, then favorable trades,
/// then security pokes, then unfavorable trades (Python priorities 3/2/1/0).
fn best_attack(game: &Game, pid: PlayerId, valid: &[u16]) -> Option<u16> {
    let player = game.player(pid);
    let opp_id = 1 - pid;
    let opponent = game.player(opp_id);

    let mut best: Option<((i32, i32, i32, i32), u16)> = None;

    for &action in valid {
        if !(ATTACK_START..ATTACK_END).contains(&action) {
            continue;
        }
        let offset = action - ATTACK_START;
        let attacker_idx = offset / TARGETS_PER_ATTACKER;
        let target_idx = offset % TARGETS_PER_ATTACKER;

        if attacker_idx as usize >= player.battle_area.len() {
            continue;
        }
        let attacker = &player.battle_area[attacker_idx as usize];
        let attacker_dp = game
            .effective_dp(crate::permanent::PermanentHandle {
                player: pid,
                index: attacker_idx as u8,
            })
            .or_else(|| attacker.base_dp(&game.card_data))
            .unwrap_or(0);

        let score: (i32, i32, i32, i32);
        if target_idx == SECURITY_TARGET {
            let is_lethal = opponent.security.is_empty();
            let priority = if is_lethal { 3 } else { 1 };
            score = (priority, attacker_dp, -(attacker_idx as i32), 0);
        } else {
            if target_idx as usize >= opponent.battle_area.len() {
                continue;
            }
            let target = &opponent.battle_area[target_idx as usize];
            let target_dp = game
                .effective_dp(crate::permanent::PermanentHandle {
                    player: opp_id,
                    index: target_idx as u8,
                })
                .or_else(|| target.base_dp(&game.card_data))
                .unwrap_or(0);
            let favorable = attacker_dp > target_dp;
            let priority = if favorable { 2 } else { 0 };
            score = (
                priority,
                attacker_dp - target_dp,
                -(attacker_idx as i32),
                -(target_idx as i32),
            );
        }
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, action));
        }
    }
    best.map(|(_, a)| a)
}

/// Best play from hand: prefer higher cost (bigger board presence) and
/// Digimon > Option > Tamer. Matches Python's `_best_play`.
fn best_play(game: &Game, pid: PlayerId, valid: &[u16]) -> Option<u16> {
    let player = game.player(pid);
    let mut best: Option<((i32, i32, i32), u16)> = None;
    for &action in valid {
        if !(PLAY_HAND_START..PLAY_HAND_END).contains(&action) {
            continue;
        }
        let hand_idx = action - PLAY_HAND_START;
        if hand_idx as usize >= player.hand.len() {
            continue;
        }
        let card = &player.hand[hand_idx as usize];
        let kind_score = match card.card_kind(&game.card_data) {
            CardKind::Digimon => 2,
            CardKind::Option => 1,
            _ => 0,
        };
        let cost = card.play_cost(&game.card_data) as i32;
        let score = (cost, kind_score, -(hand_idx as i32));
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, action));
        }
    }
    best.map(|(_, a)| a)
}

/// Pick the lowest-value hand card to trash (non-Digimon first, then
/// lowest cost, then lowest hand index). Returns `None` if the valid
/// actions aren't hand-index-shaped.
fn best_trash_choice(game: &Game, pid: PlayerId, valid: &[u16]) -> Option<u16> {
    let player = game.player(pid);
    let mut best: Option<((i32, i32, i32), u16)> = None;
    for &action in valid {
        // Trash-from-hand uses hand-index-shaped ids. PASS is always valid
        // and exits the selection — treat non-hand actions as non-candidates.
        if action == PASS {
            continue;
        }
        let hand_idx = action as usize;
        if hand_idx >= player.hand.len() {
            continue;
        }
        let card = &player.hand[hand_idx];
        let kind_score = match card.card_kind(&game.card_data) {
            CardKind::Digimon => 2,
            CardKind::Option => 1,
            _ => 0,
        };
        let cost = card.play_cost(&game.card_data) as i32;
        // Lowest-value first → negate for max-selection.
        let score = (-kind_score, -cost, -(hand_idx as i32));
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, action));
        }
    }
    best.map(|(_, a)| a)
}
