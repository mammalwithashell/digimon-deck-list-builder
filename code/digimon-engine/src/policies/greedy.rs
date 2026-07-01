//! Heuristic greedy opponent — derived from Python's `greedy_policy()` in
//! `digimon_gym/digimon_gym.py`, with Rust-owned refinements documented in
//! `docs/RUST_PYTHON_PARITY.md`.
//!
//! Priorities:
//! - Mulligan: keep if hand contains a level-3 Digimon, else mulligan.
//! - Trash-from-hand selection: discard the lowest-value card (prefer
//!   non-Digimon, then lower cost).
//! - Breeding phase: Hatch > Move if the raising Digimon has value or a
//!   hand evolution line > Pass.
//! - Main phase: setup Play (search/resource cards or Tamers) > best
//!   keep-turn Digivolve > best Attack (security pressure before board
//!   trades) > best Play > Pass.
//!
//! The heuristic inspects the full `Game` state rather than the obs
//! tensor, so it's cheap and deterministic. Tie-breaks are deterministic
//! (by level, DP, cost, index).
//!
//! Replaces the `first_valid_action` placeholder previously wired into
//! `PlayerKind::Greedy` (see commit 4ab3c87a). Parity status is tracked in
//! `docs/RUST_PYTHON_PARITY.md` under "Policies".

use crate::action::space::{
    ATTACK_END, ATTACK_START, BREEDING_TARGET, CONCEDE_GAME, DIGIVOLVE_END, DIGIVOLVE_START,
    FIELDS_PER_HAND, HATCH, MOVE_FROM_BREEDING, PASS, PLAY_FIRST, PLAY_HAND_END, PLAY_HAND_START,
    PLAY_SECOND, SECURITY_TARGET, TARGETS_PER_ATTACKER,
};
use crate::card_data::CardData;
use crate::enums::{CardKind, GamePhase, PlayerId};
use crate::game::Game;
use crate::resource_flow::card_data_indicates_resource_flow;
use crate::selection::SelectionKind;

/// Pick the next action for a `PlayerKind::Greedy` seat, using the
/// Python-parity heuristic. The returned id is guaranteed to be a valid
/// entry in the provided mask (or `PASS` if the mask is empty).
pub fn greedy_action(game: &Game, mask: &[f32]) -> u16 {
    let all_valid: Vec<u16> = mask
        .iter()
        .enumerate()
        .filter_map(|(i, v)| if *v > 0.0 { Some(i as u16) } else { None })
        .collect();

    if all_valid.is_empty() {
        return PASS;
    }

    // CONCEDE_GAME (93) is legal at every agent decision point per
    // `add-bo3-match-training` + CLAUDE.md rule 26. The pre-fix heuristic
    // would pick it whenever it was the lowest non-PASS legal action
    // (falling through `first_non_pass` or `mulligan_choice`'s fallback
    // `valid[0]`), which made BO3 evals degenerate: greedy opponents
    // forfeited matches at every play-order pick. Strip CONCEDE from
    // the candidate set unless it is the ONLY option. Last-resort
    // concede is preserved: if the engine has somehow masked everything
    // else out (no real recovery path), the original valid is used and
    // CONCEDE may be picked.
    let valid: Vec<u16> = if all_valid.iter().any(|&a| a != CONCEDE_GAME) {
        all_valid
            .iter()
            .copied()
            .filter(|&a| a != CONCEDE_GAME)
            .collect()
    } else {
        all_valid.clone()
    };

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

    // Best-of-three play-order pick — `add-bo3-match-training` makes
    // these actions legal only during `GamePhase::SelectPlayOrder`,
    // but the pre-fix catch-all branch fell through to
    // `first_non_pass(valid)` which returned the lowest-id non-PASS
    // action — including `CONCEDE_GAME` (93). Explicit branch here so
    // the greedy heuristic picks a sensible play-order action instead
    // of forfeiting the match. Default: PLAY_FIRST when offered, else
    // PLAY_SECOND.
    if game.current_phase == GamePhase::SelectPlayOrder {
        if valid.contains(&PLAY_FIRST) {
            return PLAY_FIRST;
        }
        if valid.contains(&PLAY_SECOND) {
            return PLAY_SECOND;
        }
        // SelectPlayOrder should always offer at least one of 94/95;
        // if not, fall through to the general non-concede fallback.
    }

    match game.current_phase {
        GamePhase::Breeding => {
            if valid.contains(&HATCH) {
                return HATCH;
            }
            if valid.contains(&MOVE_FROM_BREEDING) && should_move_from_breeding(game, player_id) {
                return MOVE_FROM_BREEDING;
            }
            if valid.contains(&PASS) {
                return PASS;
            }
            first_non_pass(&valid)
        }
        GamePhase::Main => {
            if let Some(a) = best_setup_play(game, player_id, &valid) {
                return a;
            }
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

/// Fallback action selector — returns the first id in `valid` that is
/// neither PASS nor CONCEDE_GAME. CONCEDE is excluded because it is
/// legal at every agent decision point (per `add-bo3-match-training`
/// + CLAUDE.md rule 26) — picking it as a "default" forfeits the game
/// when the per-phase heuristic just has nothing better to suggest.
/// Falls through to PASS if nothing else is available, then to
/// CONCEDE_GAME only when every other slot is masked out (last resort).
fn first_non_pass(valid: &[u16]) -> u16 {
    for &a in valid {
        if a != PASS && a != CONCEDE_GAME {
            return a;
        }
    }
    // No real-action slot — try PASS, then last-resort CONCEDE.
    if valid.contains(&PASS) {
        return PASS;
    }
    if valid.contains(&CONCEDE_GAME) {
        return CONCEDE_GAME;
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

fn can_digivolve_card_onto(
    hand_card: &crate::card_source::CardSource,
    base_perm: &crate::permanent::Permanent,
    data: &[CardData],
) -> bool {
    let entry = &data[hand_card.data_index];
    let Some(base_level) = base_perm.level(data) else {
        return false;
    };
    let base_colors = base_perm.top_card().colors(data);

    entry.evo_costs.iter().any(|evo| {
        evo.level == base_level && base_colors.iter().any(|c| (*c as u8) == evo.card_color)
    })
}

fn has_valid_evolution_in_hand_for_base(
    game: &Game,
    pid: PlayerId,
    base_perm: &crate::permanent::Permanent,
) -> bool {
    game.player(pid)
        .hand
        .iter()
        .any(|card| can_digivolve_card_onto(card, base_perm, &game.card_data))
}

fn should_move_from_breeding(game: &Game, pid: PlayerId) -> bool {
    let Some(base) = game.player(pid).breeding_area.as_ref() else {
        return false;
    };
    let top = base.top_card();
    if top.level(&game.card_data) != Some(3) {
        return true;
    }
    if card_has_resource_flow(game, top) {
        return true;
    }
    has_valid_evolution_in_hand_for_base(game, pid, base)
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

fn best_setup_play(game: &Game, pid: PlayerId, valid: &[u16]) -> Option<u16> {
    let player = game.player(pid);
    let mut best: Option<((i32, i32, i32, i32, i32, i32), u16)> = None;
    for &action in valid {
        if !(PLAY_HAND_START..PLAY_HAND_END).contains(&action) {
            continue;
        }
        let hand_idx = action - PLAY_HAND_START;
        if hand_idx as usize >= player.hand.len() {
            continue;
        }
        let card = &player.hand[hand_idx as usize];
        let is_resource = card_has_resource_flow(game, card);
        let is_tamer = card.card_kind(&game.card_data) == CardKind::Tamer;
        if !is_resource && !is_tamer {
            continue;
        }

        let level_score = if card.is_digimon(&game.card_data) {
            10 - card.level(&game.card_data).unwrap_or(10) as i32
        } else {
            0
        };
        let dp = card.dp(&game.card_data).unwrap_or(0);
        let cost = card.play_cost(&game.card_data) as i32;
        let score = (
            i32::from(is_resource),
            i32::from(is_tamer),
            level_score,
            dp,
            -cost,
            -(hand_idx as i32),
        );
        if best.as_ref().map_or(true, |(s, _)| score > *s) {
            best = Some((score, action));
        }
    }
    best.map(|(_, a)| a)
}

fn card_has_resource_flow(game: &Game, card: &crate::card_source::CardSource) -> bool {
    let data = &game.card_data[card.data_index];
    if card_data_indicates_resource_flow(data) {
        return true;
    }

    if let Some(effect) = game.effect_registry.get(&data.card_id) {
        if effect
            .effects(card.handle())
            .iter()
            .any(|effect| effect.resource_flow)
        {
            return true;
        }
    }

    false
}

/// Best attack: prioritize lethal security hits, then security pressure,
/// then favorable trades, then unfavorable trades.
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
            let priority = if is_lethal { 3 } else { 2 };
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
            let priority = if favorable { 1 } else { 0 };
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

/// Best fallback play from hand: prefer Digimon that build a curve before
/// expensive hard-plays, then sort non-Digimon by kind/cost/index. Tamers
/// are normally handled earlier by `best_setup_play`.
fn best_play(game: &Game, pid: PlayerId, valid: &[u16]) -> Option<u16> {
    let player = game.player(pid);
    let mut best: Option<((i32, i32, i32, i32, i32), u16)> = None;
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
            CardKind::Digimon => 3,
            CardKind::Tamer => 2,
            CardKind::Option => 1,
            _ => 0,
        };
        let curve_score = if card.is_digimon(&game.card_data) {
            10 - card.level(&game.card_data).unwrap_or(10) as i32
        } else {
            0
        };
        let dp = card.dp(&game.card_data).unwrap_or(0);
        let cost = card.play_cost(&game.card_data) as i32;
        let score = (kind_score, curve_score, dp, -cost, -(hand_idx as i32));
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::action::space::PLAY_HAND_START;
    use crate::debug_runner::{make_test_card, DebugRunner};

    #[test]
    fn fallback_best_play_prefers_curve_digimon_over_tamer() {
        let mut rookie = make_test_card("BT1-010", "Agumon");
        rookie.level = Some(3);
        rookie.dp = Some(2000);

        let mut tamer = make_test_card("BT1-080", "Setup Tamer");
        tamer.card_kind = CardKind::Tamer;
        tamer.level = None;
        tamer.dp = None;

        let db = HashMap::from([
            (rookie.card_id.clone(), rookie),
            (tamer.card_id.clone(), tamer),
        ]);
        let runner = DebugRunner::builder()
            .with_card_data(db)
            .hand(0, &["BT1-080", "BT1-010"])
            .start();
        let valid = vec![PLAY_HAND_START, PLAY_HAND_START + 1];

        assert_eq!(
            best_play(&runner.game, 0, &valid),
            Some(PLAY_HAND_START + 1)
        );
    }
}
