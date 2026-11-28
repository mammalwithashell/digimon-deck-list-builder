//! Remap candidate `target` / `avoid` lines from a staged board onto a play-in
//! reached board (where inert filler cards may shift hand/trash/field indices)
//! and verify `verify_puzzle` goal semantics.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::action::build_action_mask;
use crate::action::space::{
    decode_attack, decode_digivolve, decode_field_effect, decode_source_select, encode_attack,
    encode_digivolve, encode_source_select, ATTACK_END, ATTACK_START, BREEDING_SOURCE_SELECT_END,
    BREEDING_SOURCE_SELECT_START, BREEDING_TARGET, CONCEDE_GAME, DIGIVOLVE_END, DIGIVOLVE_START,
    DNA_DIGIVOLVE_END, DNA_DIGIVOLVE_START, FIELD_EFFECT_END, FIELD_EFFECT_START, HAND_EFFECT_END,
    HAND_EFFECT_START, HATCH, MOVE_FROM_BREEDING, PASS, PLAY_FIRST, PLAY_HAND_END, PLAY_HAND_START,
    PLAY_SECOND, REPLACEMENT_ACCEPT, SECURITY_TARGET, SOURCE_SELECT_END, SOURCE_SELECT_START,
    TARGETS_PER_ATTACKER, TRASH_EFFECT_END, TRASH_EFFECT_START,
};
use crate::card_registry::CardRegistry;
use crate::enums::{GamePhase, PlayerId};
use crate::game::Game;
use crate::inference::{
    match_refutation_trigger, score_line_with_evaluator, RefutationTrigger, ScoreLineParams,
};
use crate::observation::ObservationProfileId;
use crate::owned_card_choice::{CardChoiceOperation, CardChoiceZone};
use crate::permanent::PermanentHandle;
use crate::policies::greedy_action;
use crate::search::{decision_player, UniformEvaluator};
use crate::selection::SelectionKind;

use super::deck_stacker::{perm_linked_ids, perm_stack_ids};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemappedLine {
    pub id: String,
    pub name: String,
    pub role: String,
    pub actions: Vec<u16>,
}

pub fn parse_u16_vec(val: Option<&Value>, label: &str) -> Result<Vec<u16>, String> {
    let Some(v) = val else {
        return Ok(Vec::new());
    };
    if v.is_null() {
        return Ok(Vec::new());
    }
    let arr = v
        .as_array()
        .ok_or_else(|| format!("{label} must be an array of integers"))?;
    arr.iter()
        .map(|item| {
            item.as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .ok_or_else(|| format!("non-u16 entry in {label}"))
        })
        .collect()
}

pub fn extract_puzzle_lines(doc: &Value) -> Result<Vec<RemappedLine>, String> {
    let mut out = Vec::new();
    let Some(lines_val) = doc.get("lines") else {
        return Ok(out);
    };
    if let Some(arr) = lines_val.as_array() {
        for (idx, ln) in arr.iter().enumerate() {
            let id = ln
                .get("id")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| format!("line-{}", idx + 1));
            let name = ln
                .get("name")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| id.clone());
            let role = ln
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("target")
                .to_string();
            let actions = parse_u16_vec(ln.get("actions"), &format!("lines[{idx}].actions"))?;
            out.push(RemappedLine {
                id,
                name,
                role,
                actions,
            });
        }
    } else if let Some(obj) = lines_val.as_object() {
        let avoided_prefixes: Vec<Vec<u16>> = doc
            .get("expectations")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter(|e| {
                        matches!(
                            e.get("kind").and_then(Value::as_str),
                            Some("not_chosen" | "avoid")
                        )
                    })
                    .filter_map(|e| parse_u16_vec(e.get("plan"), "expectation.plan").ok())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        for (name, acts_val) in obj {
            let actions = parse_u16_vec(Some(acts_val), &format!("lines.{name}"))?;
            let lower = name.to_lowercase();
            let is_avoid = lower.contains("avoid")
                || lower.contains("trap")
                || avoided_prefixes.iter().any(|p| actions.starts_with(p));
            let role = if is_avoid { "avoid" } else { "target" }.to_string();
            out.push(RemappedLine {
                id: name.clone(),
                name: name.clone(),
                role,
                actions,
            });
        }
    }
    Ok(out)
}

fn match_zone_map(
    orig_ids: &[String],
    perm_ids: &[String],
    prev: &HashMap<usize, usize>,
) -> Option<HashMap<usize, usize>> {
    let mut used = vec![false; perm_ids.len()];
    let mut map = HashMap::new();

    for (o_idx, cid) in orig_ids.iter().enumerate() {
        if let Some(&pref) = prev.get(&o_idx) {
            if pref < perm_ids.len() && !used[pref] && perm_ids[pref] == *cid {
                map.insert(o_idx, pref);
                used[pref] = true;
            }
        }
    }

    for (o_idx, cid) in orig_ids.iter().enumerate().rev() {
        if map.contains_key(&o_idx) {
            continue;
        }
        let mut found = None;
        for (p_idx, p_cid) in perm_ids.iter().enumerate().rev() {
            if !used[p_idx] && p_cid == cid {
                found = Some(p_idx);
                break;
            }
        }
        let p_idx = found?;
        map.insert(o_idx, p_idx);
        used[p_idx] = true;
    }
    Some(map)
}

fn match_field_map(
    orig_game: &Game,
    perm_game: &Game,
    player: PlayerId,
    prev: &HashMap<usize, usize>,
) -> Option<HashMap<usize, usize>> {
    let o_ba = &orig_game.players[player as usize].battle_area;
    let p_ba = &perm_game.players[player as usize].battle_area;
    if o_ba.len() != p_ba.len() {
        return None;
    }
    let sig = |g: &Game, perm: &crate::permanent::Permanent| {
        (
            perm_stack_ids(g, perm),
            perm_linked_ids(g, perm),
            perm.is_suspended,
            perm.top_card().is_token,
        )
    };
    let o_sigs: Vec<_> = o_ba.iter().map(|p| sig(orig_game, p)).collect();
    let p_sigs: Vec<_> = p_ba.iter().map(|p| sig(perm_game, p)).collect();

    let mut used = vec![false; p_ba.len()];
    let mut map = HashMap::new();
    for (o_idx, o_s) in o_sigs.iter().enumerate() {
        if let Some(&pref) = prev.get(&o_idx) {
            if pref < p_sigs.len() && !used[pref] && &p_sigs[pref] == o_s {
                map.insert(o_idx, pref);
                used[pref] = true;
            }
        }
    }
    for (o_idx, o_s) in o_sigs.iter().enumerate() {
        if map.contains_key(&o_idx) {
            continue;
        }
        let p_idx = (0..p_sigs.len()).find(|&pi| !used[pi] && &p_sigs[pi] == o_s)?;
        map.insert(o_idx, p_idx);
        used[p_idx] = true;
    }
    Some(map)
}

pub fn remap_single_action(
    action_id: u16,
    orig_game: &Game,
    perm_game: &Game,
    mover: PlayerId,
    hand_maps: &[HashMap<usize, usize>; 2],
    field_maps: &[HashMap<usize, usize>; 2],
    trash_maps: &[HashMap<usize, usize>; 2],
) -> Option<u16> {
    if orig_game.current_phase == GamePhase::Mulligan
        || matches!(
            action_id,
            HATCH | MOVE_FROM_BREEDING | PASS | CONCEDE_GAME | PLAY_FIRST | PLAY_SECOND | 99
        )
    {
        return Some(action_id);
    }

    // 1. Owned paged card_choice (actions 30..=57).
    if let (Some(o_sel), Some(p_sel)) = (
        orig_game.pending_selection.as_ref(),
        perm_game.pending_selection.as_ref(),
    ) {
        if let (Some(o_cc), Some(p_cc)) = (o_sel.card_choice.as_ref(), p_sel.card_choice.as_ref())
        {
            if (30..=57).contains(&action_id) {
                let o_view = o_cc.view(orig_game, mover);
                let p_view = p_cc.view(perm_game, mover);
                if let Some(o_row) = o_view.rows.iter().find(|r| r.action_id == action_id) {
                    let owner = o_row.owner.unwrap_or(mover) as usize;
                    let mapped_carrier = o_row.carrier_slot.and_then(|cs| {
                        if cs == BREEDING_TARGET as u8 {
                            Some(cs)
                        } else {
                            field_maps[owner].get(&(cs as usize)).map(|&m| m as u8)
                        }
                    });
                    let orig_pos = o_row
                        .position
                        .as_deref()
                        .and_then(|s| s.parse::<usize>().ok());
                    let mapped_pos = match (o_row.zone.as_ref(), orig_pos) {
                        (Some(CardChoiceZone::Hand), Some(idx)) => {
                            hand_maps[owner].get(&idx).copied()
                        }
                        (Some(CardChoiceZone::Trash), Some(idx)) => {
                            trash_maps[owner].get(&idx).copied()
                        }
                        (Some(CardChoiceZone::BattleTop), Some(idx)) => {
                            field_maps[owner].get(&idx).copied()
                        }
                        (_, p) => p,
                    };
                    for p_row in &p_view.rows {
                        if !matches!(p_row.operation, CardChoiceOperation::CommitCard) {
                            continue;
                        }
                        let same_zone = match (&p_row.zone, &o_row.zone) {
                            (Some(a), Some(b)) => {
                                std::mem::discriminant(a) == std::mem::discriminant(b)
                            }
                            (None, None) => true,
                            _ => false,
                        };
                        if !same_zone
                            || p_row.owner != o_row.owner
                            || p_row.card_id != o_row.card_id
                        {
                            continue;
                        }
                        if mapped_carrier.is_some() && p_row.carrier_slot != mapped_carrier {
                            continue;
                        }
                        let p_pos = p_row
                            .position
                            .as_deref()
                            .and_then(|s| s.parse::<usize>().ok());
                        if mapped_pos.is_some() && p_pos != mapped_pos {
                            continue;
                        }
                        return Some(p_row.action_id);
                    }
                }
            }
            if matches!(action_id, 58 | 59) {
                return Some(action_id);
            }
        }

        if matches!(o_sel.kind, SelectionKind::Replacement) && action_id == REPLACEMENT_ACCEPT {
            return Some(REPLACEMENT_ACCEPT);
        }
    }

    let phase = orig_game.current_phase;
    let opp = (1 - mover) as usize;
    let me = mover as usize;

    // 2. Selection / interrupt phases.
    if orig_game.pending_selection.is_some()
        || !matches!(
            phase,
            GamePhase::Main | GamePhase::Breeding | GamePhase::Mulligan | GamePhase::EndOfTurnAction
        )
    {
        if matches!(phase, GamePhase::BlockTiming | GamePhase::AllianceTiming) {
            if (100..114).contains(&action_id) {
                let slot = (action_id - 100) as usize;
                return field_maps[me].get(&slot).map(|&ns| 100 + ns as u16);
            }
            return Some(action_id);
        }
        if phase == GamePhase::CounterTiming {
            if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
                let (h, f) = decode_digivolve(action_id);
                let nh = *hand_maps[me].get(&(h as usize))? as u16;
                let nf = if f == BREEDING_TARGET {
                    BREEDING_TARGET
                } else {
                    *field_maps[me].get(&(f as usize))? as u16
                };
                return Some(encode_digivolve(nh, nf));
            }
            return Some(action_id);
        }
        if (SOURCE_SELECT_START..SOURCE_SELECT_END).contains(&action_id) {
            let (f, s) = decode_source_select(action_id);
            let owner = orig_game
                .pending_selection
                .as_ref()
                .and_then(|sel| sel.zone_owner)
                .unwrap_or(mover) as usize;
            let nf = *field_maps[owner].get(&(f as usize))? as u16;
            return encode_source_select(nf, s);
        }
        if (BREEDING_SOURCE_SELECT_START..BREEDING_SOURCE_SELECT_END).contains(&action_id) {
            return Some(action_id);
        }

        if let Some(sel) = orig_game.pending_selection.as_ref() {
            let zone_owner = sel.zone_owner.unwrap_or(mover) as usize;
            match &sel.kind {
                SelectionKind::Material | SelectionKind::Source if action_id < 14 => {
                    return field_maps[zone_owner]
                        .get(&(action_id as usize))
                        .map(|&nf| nf as u16);
                }
                SelectionKind::Hand | SelectionKind::UnionZone { .. }
                    if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) =>
                {
                    return hand_maps[zone_owner]
                        .get(&(action_id as usize))
                        .map(|&nh| nh as u16);
                }
                SelectionKind::Hand if (HAND_EFFECT_START..HAND_EFFECT_END).contains(&action_id) => {
                    let h = (action_id - HAND_EFFECT_START) as usize;
                    return hand_maps[zone_owner]
                        .get(&h)
                        .map(|&nh| HAND_EFFECT_START + nh as u16);
                }
                SelectionKind::Trash | SelectionKind::UnionZone { .. }
                    if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action_id) =>
                {
                    let t = (action_id - TRASH_EFFECT_START) as usize;
                    return trash_maps[zone_owner]
                        .get(&t)
                        .map(|&nt| TRASH_EFFECT_START + nt as u16);
                }
                SelectionKind::Trash if (130..180).contains(&action_id) => {
                    let t = (action_id - 130) as usize;
                    return trash_maps[zone_owner].get(&t).map(|&nt| 130 + nt as u16);
                }
                SelectionKind::OwnField if (100..114).contains(&action_id) => {
                    let s = (action_id - 100) as usize;
                    return field_maps[me].get(&s).map(|&ns| 100 + ns as u16);
                }
                SelectionKind::OppField if (100..114).contains(&action_id) => {
                    let s = (action_id - 100) as usize;
                    return field_maps[opp].get(&s).map(|&ns| 100 + ns as u16);
                }
                SelectionKind::AnyField if (100..130).contains(&action_id) => {
                    let rel = action_id - 100;
                    let p = (rel / TARGETS_PER_ATTACKER) as usize;
                    let s = (rel % TARGETS_PER_ATTACKER) as usize;
                    if p < 2 && s < 14 {
                        return field_maps[p]
                            .get(&s)
                            .map(|&ns| 100 + (p as u16) * TARGETS_PER_ATTACKER + ns as u16);
                    }
                }
                _ => {}
            }
        }
        return Some(action_id);
    }

    // 3. Main / Breeding / EndOfTurnAction root ranges.
    if (PLAY_HAND_START..PLAY_HAND_END).contains(&action_id) {
        return hand_maps[me]
            .get(&(action_id as usize))
            .map(|&nh| nh as u16);
    }
    if (HAND_EFFECT_START..HAND_EFFECT_END).contains(&action_id) {
        let h = (action_id - HAND_EFFECT_START) as usize;
        return hand_maps[me]
            .get(&h)
            .map(|&nh| HAND_EFFECT_START + nh as u16);
    }
    if (DNA_DIGIVOLVE_START..DNA_DIGIVOLVE_END).contains(&action_id) {
        let h = (action_id - DNA_DIGIVOLVE_START) as usize;
        return hand_maps[me]
            .get(&h)
            .map(|&nh| DNA_DIGIVOLVE_START + nh as u16);
    }
    if (ATTACK_START..ATTACK_END).contains(&action_id) {
        let (atk, tgt) = decode_attack(action_id);
        let natk = *field_maps[me].get(&(atk as usize))? as u16;
        let ntgt = if tgt == SECURITY_TARGET {
            SECURITY_TARGET
        } else {
            *field_maps[opp].get(&(tgt as usize))? as u16
        };
        return Some(encode_attack(natk, ntgt));
    }
    if (DIGIVOLVE_START..DIGIVOLVE_END).contains(&action_id) {
        let (h, slot) = decode_digivolve(action_id);
        let nh = *hand_maps[me].get(&(h as usize))? as u16;
        let nslot = if slot == BREEDING_TARGET {
            BREEDING_TARGET
        } else {
            *field_maps[me].get(&(slot as usize))? as u16
        };
        return Some(encode_digivolve(nh, nslot));
    }
    if (FIELD_EFFECT_START..FIELD_EFFECT_END).contains(&action_id) {
        let (perm_idx, eff_idx) = decode_field_effect(action_id);
        let np = if perm_idx == BREEDING_TARGET {
            BREEDING_TARGET
        } else {
            *field_maps[me].get(&(perm_idx as usize))? as u16
        };
        return Some(FIELD_EFFECT_START + np * 10 + eff_idx);
    }
    if (TRASH_EFFECT_START..TRASH_EFFECT_END).contains(&action_id) {
        let t = (action_id - TRASH_EFFECT_START) as usize;
        return trash_maps[me]
            .get(&t)
            .map(|&nt| TRASH_EFFECT_START + nt as u16);
    }

    Some(action_id)
}

fn advance_opponent_interrupts(
    game: &mut Game,
    solver_seat: PlayerId,
    triggers: &[RefutationTrigger],
) -> Result<(), String> {
    for _ in 0..64 {
        if game.game_over || decision_player(game) == solver_seat {
            return Ok(());
        }
        let opp = decision_player(game);
        let mask = build_action_mask(game, opp);
        let legal: Vec<u16> = mask
            .iter()
            .enumerate()
            .filter_map(|(i, &m)| (m > 0.5).then_some(i as u16))
            .collect();
        if legal.is_empty() {
            return Err("opponent has empty legal action mask during line remap".to_string());
        }
        let chosen = if legal.len() == 1 {
            legal[0]
        } else if let Some(trig) = match_refutation_trigger(game, opp, triggers) {
            trig
        } else {
            let g_act = greedy_action(game, &mask);
            if mask.get(g_act as usize).copied().unwrap_or(0.0) > 0.5 {
                g_act
            } else {
                legal[0]
            }
        };
        game.decode_action(chosen, opp);
    }
    Ok(())
}

pub fn remap_line_actions(
    orig_root: &Game,
    perm_root: &Game,
    solver_seat: PlayerId,
    orig_actions: &[u16],
    triggers: &[RefutationTrigger],
) -> Result<Vec<u16>, String> {
    let mut orig = orig_root.clone();
    let mut perm = perm_root.clone();
    let mut hand_maps = [HashMap::new(), HashMap::new()];
    let mut field_maps = [HashMap::new(), HashMap::new()];
    let mut trash_maps = [HashMap::new(), HashMap::new()];
    let mut remapped = Vec::with_capacity(orig_actions.len());

    for (step_idx, &orig_aid) in orig_actions.iter().enumerate() {
        if !triggers.is_empty() || decision_player(&orig) != solver_seat {
            advance_opponent_interrupts(&mut orig, solver_seat, triggers)?;
            advance_opponent_interrupts(&mut perm, solver_seat, triggers)?;
        }
        if orig.game_over || perm.game_over {
            break;
        }
        let o_mover = decision_player(&orig);
        let p_mover = decision_player(&perm);
        if o_mover != p_mover {
            return Err(format!(
                "mover diverged at line step {step_idx}: orig={o_mover}, reached={p_mover}"
            ));
        }

        for p in 0..2 {
            let o_hand: Vec<String> = orig.players[p]
                .hand
                .iter()
                .map(|c| c.card_id(&orig.card_data).to_string())
                .collect();
            let p_hand: Vec<String> = perm.players[p]
                .hand
                .iter()
                .map(|c| c.card_id(&perm.card_data).to_string())
                .collect();
            hand_maps[p] = match_zone_map(&o_hand, &p_hand, &hand_maps[p])
                .ok_or_else(|| format!("hand subset mismatch on seat {p} at step {step_idx}"))?;

            let o_trash: Vec<String> = orig.players[p]
                .trash
                .iter()
                .map(|c| c.card_id(&orig.card_data).to_string())
                .collect();
            let p_trash: Vec<String> = perm.players[p]
                .trash
                .iter()
                .map(|c| c.card_id(&perm.card_data).to_string())
                .collect();
            if let Some(tm) = match_zone_map(&o_trash, &p_trash, &trash_maps[p]) {
                trash_maps[p] = tm;
            }

            field_maps[p] = match_field_map(&orig, &perm, p as u8, &field_maps[p])
                .ok_or_else(|| format!("field bijection lost on seat {p} at step {step_idx}"))?;
        }

        let new_aid = remap_single_action(
            orig_aid,
            &orig,
            &perm,
            o_mover,
            &hand_maps,
            &field_maps,
            &trash_maps,
        )
        .ok_or_else(|| format!("could not remap action {orig_aid} at step {step_idx}"))?;

        let o_mask = build_action_mask(&orig, o_mover);
        let p_mask = build_action_mask(&perm, p_mover);
        if (orig_aid as usize) >= o_mask.len() || o_mask[orig_aid as usize] <= 0.5 {
            return Err(format!(
                "original line action {orig_aid} is illegal at step {step_idx}"
            ));
        }
        if (new_aid as usize) >= p_mask.len() || p_mask[new_aid as usize] <= 0.5 {
            return Err(format!(
                "remapped action {new_aid} (from {orig_aid}) is illegal on play-in board at step {step_idx}"
            ));
        }

        remapped.push(new_aid);
        orig.decode_action(orig_aid, o_mover);
        perm.decode_action(new_aid, p_mover);
    }

    Ok(remapped)
}

pub fn evaluate_goal_assertion(game: &Game, solver_seat: u8, assertion: &Value) -> bool {
    let kind = assertion.get("kind").and_then(Value::as_str).unwrap_or("");
    let to_pid = |raw: Option<u64>| -> usize {
        match raw.unwrap_or((solver_seat as u64) + 1) {
            0 => 0,
            1 => 0,
            2 => 1,
            n => (n.saturating_sub(1) as usize).min(1),
        }
    };
    match kind {
        "winner" | "game_won" => {
            let expected = match assertion.get("player").and_then(Value::as_u64) {
                Some(p) => to_pid(Some(p)) as u8,
                None => solver_seat,
            };
            game.game_over && game.winner == Some(expected)
        }
        "memory_equals" => {
            let Some(val) = assertion.get("value").and_then(Value::as_i64) else {
                return false;
            };
            (game.memory as i64) == val
        }
        "memory_at_least" => {
            let Some(val) = assertion.get("value").and_then(Value::as_i64) else {
                return false;
            };
            (game.memory as i64) >= val
        }
        "zone_count" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let Some(val) = assertion.get("value").and_then(Value::as_u64) else {
                return false;
            };
            let p = game.player(pid as u8);
            let actual = match assertion.get("zone").and_then(Value::as_str).unwrap_or("") {
                "hand" => p.hand.len(),
                "security" => p.security.len(),
                "trash" => p.trash.len(),
                "field" | "battle_area" => p.battle_area.len(),
                "deck" => p.deck.len(),
                _ => return false,
            };
            actual == val as usize
        }
        "stack_top" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let idx = assertion
                .get("field_index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let Some(expected_id) = assertion.get("card_id").and_then(Value::as_str) else {
                return false;
            };
            game.player(pid as u8)
                .battle_area
                .get(idx)
                .is_some_and(|perm| perm.top_card().card_id(&game.card_data) == expected_id)
        }
        "field_contains" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let Some(expected_id) = assertion.get("card_id").and_then(Value::as_str) else {
                return false;
            };
            game.player(pid as u8)
                .battle_area
                .iter()
                .any(|perm| perm.top_card().card_id(&game.card_data) == expected_id)
        }
        "field_not_contains" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let Some(expected_id) = assertion.get("card_id").and_then(Value::as_str) else {
                return false;
            };
            !game.player(pid as u8)
                .battle_area
                .iter()
                .any(|perm| perm.top_card().card_id(&game.card_data) == expected_id)
        }
        "effective_dp" => {
            let pid = to_pid(assertion.get("player").and_then(Value::as_u64));
            let idx = assertion
                .get("field_index")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u8;
            let Some(val) = assertion.get("value").and_then(Value::as_i64) else {
                return false;
            };
            game.effective_dp(PermanentHandle {
                player: pid as u8,
                index: idx,
            }) == Some(val as i32)
        }
        _ => false,
    }
}

pub fn line_satisfies_puzzle_goal(game: &Game, solver_seat: u8, doc: &Value) -> bool {
    if game.game_over {
        return game.winner == Some(solver_seat);
    }
    let rl_goal = doc.get("rl_goal").unwrap_or(&Value::Null);
    let win_assertions = rl_goal
        .get("win_assertions")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .or_else(|| {
            doc.get("assertions")
                .and_then(|a| a.get("engine"))
                .and_then(Value::as_array)
                .filter(|a| !a.is_empty())
        });
    if let Some(assertions) = win_assertions {
        return assertions
            .iter()
            .all(|a| evaluate_goal_assertion(game, solver_seat, a));
    }
    false
}

/// Remap all `lines` + `expectations` onto `reached_game` and verify `verify_puzzle`
/// semantics (target lines satisfy goal, avoid lines fail/refute).
pub fn remap_and_verify_puzzle(
    orig_game: &Game,
    reached_game: &Game,
    solver_seat: PlayerId,
    doc: &Value,
    registry: &CardRegistry,
    triggers: &[RefutationTrigger],
) -> Result<(Vec<RemappedLine>, Vec<Value>), String> {
    let orig_lines = extract_puzzle_lines(doc)?;
    let mut remapped_lines = Vec::with_capacity(orig_lines.len());

    for ln in &orig_lines {
        let actions = if ln.actions.is_empty() {
            Vec::new()
        } else {
            remap_line_actions(orig_game, reached_game, solver_seat, &ln.actions, triggers)?
        };
        remapped_lines.push(RemappedLine {
            id: ln.id.clone(),
            name: ln.name.clone(),
            role: ln.role.clone(),
            actions,
        });
    }

    // Verify `target` and `avoid` lines under adversary interrupts across 2 seeds.
    let mut uni = UniformEvaluator;
    let verify_seeds = 2u64;
    for ln in &remapped_lines {
        if (ln.role != "target" && ln.role != "avoid") || ln.actions.is_empty() {
            continue;
        }
        for s in 0..verify_seeds {
            let params = ScoreLineParams {
                actions: ln.actions.clone(),
                auto_adversary: true,
                adversary_sims: 32,
                adversary_seed: s.wrapping_mul(0x5851f42d4c957f2d),
                refutation_triggers: triggers.to_vec(),
                max_adversary_steps: 64,
            };
            let res = score_line_with_evaluator(
                reached_game,
                registry,
                &mut uni,
                ObservationProfileId::StandardLiteV2,
                &params,
            )
            .map_err(|e| format!("line {} failed verification on play-in board: {e}", ln.id))?;
            let solved = line_satisfies_puzzle_goal(&res.final_game, solver_seat, doc);
            if ln.role == "target" && !solved {
                return Err(format!(
                    "target line {} did not satisfy puzzle goal on play-in board (seed {s})",
                    ln.id
                ));
            }
            if ln.role == "avoid" && solved {
                return Err(format!(
                    "avoid line {} unexpectedly satisfied goal on play-in board (seed {s})",
                    ln.id
                ));
            }
        }
    }

    // Remap expectations' `plan` action sequences from root.
    let mut remapped_expectations = Vec::new();
    if let Some(exp_arr) = doc.get("expectations").and_then(Value::as_array) {
        for exp in exp_arr {
            let mut item = exp.clone();
            if let Ok(orig_plan) = parse_u16_vec(exp.get("plan"), "expectation.plan") {
                if !orig_plan.is_empty() {
                    if let Ok(new_plan) =
                        remap_line_actions(orig_game, reached_game, solver_seat, &orig_plan, triggers)
                    {
                        item["plan"] = json!(new_plan);
                    }
                }
            }
            remapped_expectations.push(item);
        }
    }

    Ok((remapped_lines, remapped_expectations))
}
