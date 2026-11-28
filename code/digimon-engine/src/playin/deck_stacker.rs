//! Target-board goal extraction, equivalence/distance scoring, and
//! deterministic `Game::new` pre-shuffle deck stacking for play-in compilation.

use std::collections::{BTreeMap, HashMap};

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use crate::card_data::CardData;
use crate::enums::{CardKind, GamePhase, Keyword, PlayerId};
use crate::game::Game;
use crate::permanent::Permanent;
use crate::search::decision_player;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPermanent {
    pub stack_ids: Vec<String>,
    pub linked_ids: Vec<String>,
    pub is_suspended: bool,
    pub is_token: bool,
    pub needs_prior_turn: bool,
    pub needs_same_turn: bool,
}

#[derive(Debug, Clone)]
pub struct SeatGoal {
    pub field_stacks: Vec<TargetPermanent>,
    pub breeding_stack: Option<Vec<String>>,
    pub hand_vec: Vec<String>,
    pub hand_multiset: BTreeMap<String, usize>,
    pub trash_vec: Vec<String>,
    pub trash_multiset: BTreeMap<String, usize>,
    pub security_bottom_to_top: Vec<String>,
    pub security_count: usize,
}

#[derive(Debug, Clone)]
pub struct PlayInGoal {
    pub solver_seat: PlayerId,
    pub decision_player: PlayerId,
    pub turn_player: PlayerId,
    pub current_phase: GamePhase,
    pub memory: i16,
    pub seats: [SeatGoal; 2],
}

#[derive(Debug, Clone)]
pub struct StackedSeatPlan {
    /// Chronological draw stack (`[hand_0..5, sec_0..5, draw_0, draw_1, ...]`).
    /// Directly compatible with `dcgo-harness` `ScenarioSeat.stack`.
    pub draw_stack: Vec<String>,
    /// `player.deck` post-shuffle order (tail = next pop).
    pub post_main: Vec<String>,
    /// `player.digitama_deck` post-shuffle order (tail = next hatch).
    pub post_egg: Vec<String>,
}

pub fn perm_stack_ids(game: &Game, perm: &Permanent) -> Vec<String> {
    perm.card_sources
        .iter()
        .map(|cs| cs.card_id(&game.card_data).to_string())
        .collect()
}

pub fn perm_linked_ids(game: &Game, perm: &Permanent) -> Vec<String> {
    perm.linked_cards
        .iter()
        .map(|cs| cs.card_id(&game.card_data).to_string())
        .collect()
}

fn card_has_delay_or_digimon_sickness(cd: &CardData) -> bool {
    match cd.card_kind {
        CardKind::Digimon | CardKind::Token | CardKind::Dual => !cd.keywords.contains(&Keyword::Rush),
        CardKind::Option => {
            cd.effect_text.contains("Delay") || cd.effect_text.contains("＜Delay＞") || cd.effect_text.contains("<Delay>")
        }
        CardKind::Tamer | CardKind::DigiEgg => false,
    }
}

pub fn extract_goal(target_game: &Game, solver_seat: PlayerId) -> PlayInGoal {
    let build_seat = |p: usize| -> SeatGoal {
        let player = &target_game.players[p];
        let mut field_stacks = Vec::with_capacity(player.battle_area.len());
        for perm in &player.battle_area {
            let stack_ids = perm_stack_ids(target_game, perm);
            let linked_ids = perm_linked_ids(target_game, perm);
            let top = perm.top_card();
            let top_cd = &target_game.card_data[top.data_index];
            let was_prior_turn = target_game.turn_count > perm.turn_played;
            let timing_sensitive = card_has_delay_or_digimon_sickness(top_cd);
            field_stacks.push(TargetPermanent {
                stack_ids,
                linked_ids,
                is_suspended: perm.is_suspended,
                is_token: top.is_token,
                needs_prior_turn: was_prior_turn && timing_sensitive,
                needs_same_turn: !was_prior_turn && timing_sensitive,
            });
        }

        let breeding_stack = player
            .breeding_area
            .as_ref()
            .map(|perm| perm_stack_ids(target_game, perm));

        let hand_vec: Vec<String> = player
            .hand
            .iter()
            .map(|c| c.card_id(&target_game.card_data).to_string())
            .collect();
        let mut hand_multiset = BTreeMap::new();
        for id in &hand_vec {
            *hand_multiset.entry(id.clone()).or_insert(0) += 1;
        }

        let trash_vec: Vec<String> = player
            .trash
            .iter()
            .map(|c| c.card_id(&target_game.card_data).to_string())
            .collect();
        let mut trash_multiset = BTreeMap::new();
        for id in &trash_vec {
            *trash_multiset.entry(id.clone()).or_insert(0) += 1;
        }

        let security_bottom_to_top: Vec<String> = player
            .security
            .iter()
            .map(|c| c.card_id(&target_game.card_data).to_string())
            .collect();
        let security_count = security_bottom_to_top.len();

        SeatGoal {
            field_stacks,
            breeding_stack,
            hand_vec,
            hand_multiset,
            trash_vec,
            trash_multiset,
            security_bottom_to_top,
            security_count,
        }
    };

    PlayInGoal {
        solver_seat,
        decision_player: decision_player(target_game),
        turn_player: target_game.turn_player(),
        current_phase: target_game.current_phase,
        memory: target_game.memory,
        seats: [build_seat(0), build_seat(1)],
    }
}

fn perm_matches_target(game: &Game, perm: &Permanent, target: &TargetPermanent) -> bool {
    let top = perm.top_card();
    if top.is_token != target.is_token {
        return false;
    }
    if perm.is_suspended != target.is_suspended {
        return false;
    }
    if perm_stack_ids(game, perm) != target.stack_ids {
        return false;
    }
    if perm_linked_ids(game, perm) != target.linked_ids {
        return false;
    }
    if target.needs_prior_turn && game.turn_count <= perm.turn_played {
        return false;
    }
    if target.needs_same_turn && game.turn_count != perm.turn_played {
        return false;
    }
    true
}

/// Find a bijective assignment from `target_stacks` indices `0..N` to `live_ba` indices `0..N`.
pub fn match_field_bijection(
    game: &Game,
    live_ba: &[Permanent],
    target_stacks: &[TargetPermanent],
) -> Option<Vec<usize>> {
    if live_ba.len() != target_stacks.len() {
        return None;
    }
    let n = target_stacks.len();
    if n == 0 {
        return Some(Vec::new());
    }
    // Fast path: identity slot mapping.
    if (0..n).all(|i| perm_matches_target(game, &live_ba[i], &target_stacks[i])) {
        return Some((0..n).collect());
    }
    let mut used = vec![false; n];
    let mut mapping = vec![0usize; n];
    fn backtrack(
        idx: usize,
        game: &Game,
        live_ba: &[Permanent],
        target_stacks: &[TargetPermanent],
        used: &mut [bool],
        mapping: &mut [usize],
    ) -> bool {
        if idx == target_stacks.len() {
            return true;
        }
        for cand in 0..live_ba.len() {
            if !used[cand] && perm_matches_target(game, &live_ba[cand], &target_stacks[idx]) {
                used[cand] = true;
                mapping[idx] = cand;
                if backtrack(idx + 1, game, live_ba, target_stacks, used, mapping) {
                    return true;
                }
                used[cand] = false;
            }
        }
        false
    }
    if backtrack(0, game, live_ba, target_stacks, &mut used, &mut mapping) {
        Some(mapping)
    } else {
        None
    }
}

pub fn is_equivalent(game: &Game, goal: &PlayInGoal, strict_zones: bool) -> bool {
    if game.game_over {
        return false;
    }
    if game.current_phase != goal.current_phase
        || decision_player(game) != goal.decision_player
        || game.turn_player() != goal.turn_player
        || game.memory != goal.memory
    {
        return false;
    }

    for p in 0..2 {
        let player = &game.players[p];
        let sg = &goal.seats[p];

        if player.security.len() != sg.security_count {
            return false;
        }

        let live_breeding = player
            .breeding_area
            .as_ref()
            .map(|perm| perm_stack_ids(game, perm));
        if live_breeding != sg.breeding_stack {
            return false;
        }

        if match_field_bijection(game, &player.battle_area, &sg.field_stacks).is_none() {
            return false;
        }

        let mut live_hand: BTreeMap<String, usize> = BTreeMap::new();
        for c in &player.hand {
            *live_hand.entry(c.card_id(&game.card_data).to_string()).or_insert(0) += 1;
        }
        let mut live_trash: BTreeMap<String, usize> = BTreeMap::new();
        for c in &player.trash {
            *live_trash.entry(c.card_id(&game.card_data).to_string()).or_insert(0) += 1;
        }

        if strict_zones {
            if live_hand != sg.hand_multiset || live_trash != sg.trash_multiset {
                return false;
            }
        } else {
            for (cid, &need) in &sg.hand_multiset {
                if live_hand.get(cid).copied().unwrap_or(0) < need {
                    return false;
                }
            }
            for (cid, &need) in &sg.trash_multiset {
                if live_trash.get(cid).copied().unwrap_or(0) < need {
                    return false;
                }
            }
        }
    }

    true
}

fn stack_prefix_match_cost(live_ids: &[String], target_ids: &[String]) -> f64 {
    let mut matched = 0usize;
    for (a, b) in live_ids.iter().zip(target_ids.iter()) {
        if a == b {
            matched += 1;
        } else {
            break;
        }
    }
    let missing = target_ids.len().saturating_sub(matched) as f64 * 10.0;
    let wrong_extra = live_ids.len().saturating_sub(matched) as f64 * 16.0;
    missing + wrong_extra
}

/// Smooth heuristic distance (`0.0` iff `is_equivalent` is `true` in identity slot order).
pub fn distance_score(game: &Game, goal: &PlayInGoal, strict_zones: bool) -> f64 {
    if game.game_over {
        return 1_000.0;
    }
    if is_equivalent(game, goal, strict_zones) {
        let exact_order = (0..2).all(|p| {
            let ba = &game.players[p].battle_area;
            let tf = &goal.seats[p].field_stacks;
            ba.len() == tf.len()
                && (0..ba.len()).all(|i| perm_matches_target(game, &ba[i], &tf[i]))
        });
        return if exact_order { 0.0 } else { 0.001 };
    }

    let mut score = 0.0f64;

    for p in 0..2 {
        let player = &game.players[p];
        let sg = &goal.seats[p];
        let opp_sg = &goal.seats[1 - p];
        let opp_sec_live = game.players[1 - p].security.len();

        // Security alignment.
        let sec_live = player.security.len();
        if sec_live > sg.security_count {
            score += (sec_live - sg.security_count) as f64 * 6.5;
        } else if sec_live < sg.security_count {
            score += (sg.security_count - sec_live) as f64 * 30.0;
        }

        // Field & Breeding stack matching.
        let mut live_candidates: Vec<(Vec<String>, bool, bool, bool)> = player
            .battle_area
            .iter()
            .map(|perm| {
                (
                    perm_stack_ids(game, perm),
                    perm.is_suspended,
                    game.turn_count > perm.turn_played,
                    false, // in_breeding
                )
            })
            .collect();

        let live_breeding_ids = player
            .breeding_area
            .as_ref()
            .map(|perm| perm_stack_ids(game, perm));

        let mut breeding_used_for_field = false;
        if sg.breeding_stack.is_none() {
            if let Some(ref b_ids) = live_breeding_ids {
                // Allow an in-progress breeding stack to match an egg-rooted target field stack!
                if sg
                    .field_stacks
                    .iter()
                    .any(|tf| !tf.stack_ids.is_empty() && tf.stack_ids[0] == b_ids[0])
                {
                    live_candidates.push((b_ids.clone(), false, true, true));
                    breeding_used_for_field = true;
                }
            }
        }

        let mut used_cand = vec![false; live_candidates.len()];
        for tf in &sg.field_stacks {
            let mut best_cost = tf.stack_ids.len() as f64 * 11.0 + 4.0;
            let mut best_idx = None;
            for (ci, (c_ids, c_susp, c_prior, c_in_breed)) in live_candidates.iter().enumerate() {
                if used_cand[ci] {
                    continue;
                }
                if c_ids.first() != tf.stack_ids.first() {
                    continue;
                }
                let mut c_cost = stack_prefix_target_cost(c_ids, c_susp, *c_prior, *c_in_breed, tf);
                if ci != best_idx.unwrap_or(ci) {
                    c_cost += 0.01;
                }
                if c_cost < best_cost {
                    best_cost = c_cost;
                    best_idx = Some(ci);
                }
            }
            if let Some(chosen) = best_idx {
                used_cand[chosen] = true;
            }
            score += best_cost;
        }

        // Penalize extra battle-area permanents (mild if still needed to chip opponent security).
        for (ci, &u) in used_cand.iter().enumerate() {
            if !u && !live_candidates[ci].3 {
                if opp_sec_live > opp_sg.security_count {
                    score += 1.5;
                } else {
                    score += 14.0;
                }
            }
        }

        // Breeding stack alignment.
        match (&sg.breeding_stack, &live_breeding_ids) {
            (Some(want_b), Some(have_b)) => {
                score += stack_prefix_match_cost(have_b, want_b) * 0.85;
            }
            (Some(want_b), None) => {
                score += want_b.len() as f64 * 9.0;
            }
            (None, Some(_)) if !breeding_used_for_field => {
                score += 9.0;
            }
            _ => {}
        }

        // Hand & Trash multiset deficits.
        let mut live_hand: BTreeMap<String, usize> = BTreeMap::new();
        for c in &player.hand {
            *live_hand.entry(c.card_id(&game.card_data).to_string()).or_insert(0) += 1;
        }
        let mut live_trash: BTreeMap<String, usize> = BTreeMap::new();
        for c in &player.trash {
            *live_trash.entry(c.card_id(&game.card_data).to_string()).or_insert(0) += 1;
        }

        for (cid, &need) in &sg.hand_multiset {
            let have = live_hand.get(cid).copied().unwrap_or(0);
            if have < need {
                score += (need - have) as f64 * 4.5;
            } else if strict_zones && have > need {
                score += (have - need) as f64 * 5.0;
            }
        }
        if strict_zones {
            for (cid, &have) in &live_hand {
                if !sg.hand_multiset.contains_key(cid) {
                    score += have as f64 * 5.0;
                }
            }
        }

        for (cid, &need) in &sg.trash_multiset {
            let have = live_trash.get(cid).copied().unwrap_or(0);
            if have < need {
                score += (need - have) as f64 * 5.0;
            } else if strict_zones && have > need {
                score += (have - need) as f64 * 6.0;
            }
        }
        if strict_zones {
            for (cid, &have) in &live_trash {
                if !sg.trash_multiset.contains_key(cid) {
                    score += have as f64 * 6.0;
                }
            }
        }
    }

    if game.turn_player() != goal.turn_player {
        score += 3.5;
        score += (-(game.memory) - goal.memory).abs() as f64 * 0.6;
    } else {
        score += (game.memory - goal.memory).abs() as f64 * 1.4;
    }
    if decision_player(game) != goal.decision_player {
        score += 2.0;
    }
    if game.current_phase != goal.current_phase {
        score += 1.8;
    }
    if game.pending_selection.is_some() && goal.current_phase == GamePhase::Main {
        score += 1.2;
    }

    score.max(0.01)
}

fn stack_prefix_target_cost(
    c_ids: &[String],
    c_susp: &bool,
    c_prior: bool,
    c_in_breed: bool,
    tf: &TargetPermanent,
) -> f64 {
    let mut cost = stack_prefix_match_cost(c_ids, &tf.stack_ids);
    if c_in_breed {
        cost += 4.0;
    }
    if *c_susp != tf.is_suspended {
        cost += 2.5;
    }
    if tf.needs_prior_turn && !c_prior {
        cost += 4.5;
    }
    if tf.needs_same_turn && c_prior {
        cost += 8.0;
    }
    cost
}

fn take_one(pool: &mut Vec<String>, card_id: &str) -> bool {
    if let Some(pos) = pool.iter().position(|c| c == card_id) {
        pool.remove(pos);
        true
    } else {
        false
    }
}

/// Invert `Game::new`'s `StdRng::seed_from_u64(seed)` main + digitama shuffles
/// so passing `[out[0], out[1]]` to `Game::new(..., Some(seed))` produces the
/// exact desired `post_main` and `post_egg` orders.
pub fn invert_game_new_shuffle(
    post_main: &[Vec<String>; 2],
    post_egg: &[Vec<String>; 2],
    seed: u64,
) -> [Vec<String>; 2] {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut out = [Vec::new(), Vec::new()];
    for p in 0..2 {
        let m_len = post_main[p].len();
        let mut perm_main: Vec<usize> = (0..m_len).collect();
        perm_main.shuffle(&mut rng);
        let mut pre_main = vec![String::new(); m_len];
        for (post_idx, &pre_idx) in perm_main.iter().enumerate() {
            pre_main[pre_idx] = post_main[p][post_idx].clone();
        }

        let e_len = post_egg[p].len();
        let mut perm_egg: Vec<usize> = (0..e_len).collect();
        perm_egg.shuffle(&mut rng);
        let mut pre_egg = vec![String::new(); e_len];
        for (post_idx, &pre_idx) in perm_egg.iter().enumerate() {
            pre_egg[pre_idx] = post_egg[p][post_idx].clone();
        }

        let mut combined = Vec::with_capacity(e_len + m_len);
        combined.extend(pre_egg);
        combined.extend(pre_main);
        out[p] = combined;
    }
    out
}

/// Solve chronological `draw_stack` + `post_egg` per seat and invert `Game::new` shuffle for `seed`.
pub fn solve_stacked_decks(
    raw_decks: &[Vec<String>; 2],
    target_game: &Game,
    goal: &PlayInGoal,
    card_data: &HashMap<String, CardData>,
    seed: u64,
) -> Result<([StackedSeatPlan; 2], [Vec<String>; 2]), String> {
    let is_egg = |id: &str| -> bool {
        card_data
            .get(id)
            .is_some_and(|cd| cd.card_kind == CardKind::DigiEgg)
    };
    let is_token = |id: &str| -> bool {
        card_data
            .get(id)
            .is_none_or(|cd| cd.card_kind == CardKind::Token)
    };

    let build_plan = |p: usize| -> Result<StackedSeatPlan, String> {
        let sg = &goal.seats[p];
        let opp_sg = &goal.seats[1 - p];

        // Reconcile raw deck pool with any staged cards on `target_game`.
        let mut egg_pool: Vec<String> = Vec::new();
        let mut main_pool: Vec<String> = Vec::new();
        for id in &raw_decks[p] {
            if is_token(id) {
                continue;
            }
            if is_egg(id) {
                egg_pool.push(id.clone());
            } else {
                main_pool.push(id.clone());
            }
        }

        // Ensure every non-token card present in `target_game.players[p]` exists in the pool.
        let mut stage_counts: BTreeMap<String, usize> = BTreeMap::new();
        let tp = &target_game.players[p];
        for cs in tp
            .hand
            .iter()
            .chain(tp.trash.iter())
            .chain(tp.security.iter())
            .chain(tp.deck.iter())
            .chain(tp.digitama_deck.iter())
        {
            if !cs.is_token {
                *stage_counts
                    .entry(cs.card_id(&target_game.card_data).to_string())
                    .or_insert(0) += 1;
            }
        }
        for perm in tp.battle_area.iter().chain(tp.breeding_area.iter()) {
            for cs in perm.card_sources.iter().chain(perm.linked_cards.iter()) {
                if !cs.is_token {
                    *stage_counts
                        .entry(cs.card_id(&target_game.card_data).to_string())
                        .or_insert(0) += 1;
                }
            }
        }
        for (cid, &need) in &stage_counts {
            if is_token(cid) {
                continue;
            }
            let target_vec = if is_egg(cid) {
                &mut egg_pool
            } else {
                &mut main_pool
            };
            let have = target_vec.iter().filter(|x| *x == cid).count();
            for _ in have..need {
                target_vec.push(cid.clone());
            }
        }
        while main_pool.len() < 20 {
            main_pool.push("ST1-03".to_string());
        }

        // 1. Order Digi-Egg deck: hatched eggs first (placed at end of `post_egg`).
        let mut hatch_order: Vec<String> = Vec::new();
        for tf in &sg.field_stacks {
            if let Some(bottom) = tf.stack_ids.first() {
                if is_egg(bottom) && take_one(&mut egg_pool, bottom) {
                    hatch_order.push(bottom.clone());
                }
            }
        }
        if let Some(ref b_stack) = sg.breeding_stack {
            if let Some(bottom) = b_stack.first() {
                if is_egg(bottom) && take_one(&mut egg_pool, bottom) {
                    hatch_order.push(bottom.clone());
                }
            }
        }
        hatch_order.extend(egg_pool);
        let post_egg: Vec<String> = hatch_order.into_iter().rev().collect();

        // 2. Reserve target surviving security `0..sec_len` (bottom-to-top).
        let sec_keep_len = sg.security_count.min(5);
        let mut sec_bottom_to_top: Vec<String> = Vec::with_capacity(5);
        for cid in sg.security_bottom_to_top.iter().take(sec_keep_len) {
            if take_one(&mut main_pool, cid) {
                sec_bottom_to_top.push(cid.clone());
            }
        }

        // Reserve required active cards first so we don't accidentally put them into security!
        let mut ordered_active: Vec<String> = Vec::new();
        // (a) Prior-turn field stacks, then same-turn field stacks (bottom-to-top, non-egg, non-token).
        let mut field_refs: Vec<&TargetPermanent> = sg.field_stacks.iter().collect();
        field_refs.sort_by_key(|tf| (!tf.needs_prior_turn, tf.is_token));
        for tf in field_refs {
            if tf.is_token {
                continue;
            }
            for cid in &tf.stack_ids {
                if !is_egg(cid) && !is_token(cid) && take_one(&mut main_pool, cid) {
                    ordered_active.push(cid.clone());
                }
            }
            for cid in &tf.linked_ids {
                if !is_egg(cid) && !is_token(cid) && take_one(&mut main_pool, cid) {
                    ordered_active.push(cid.clone());
                }
            }
        }

        // (b) Breeding stack non-egg cards.
        if let Some(ref b_stack) = sg.breeding_stack {
            for cid in b_stack {
                if !is_egg(cid) && !is_token(cid) && take_one(&mut main_pool, cid) {
                    ordered_active.push(cid.clone());
                }
            }
        }

        // (c) Trash cards: safe zero-effect / low-DP cards can ride the checked top security
        // (`sec_keep_len..5`) when security will be chipped! Others go to `ordered_active`.
        let checks_to_take = 5usize.saturating_sub(sec_bottom_to_top.len());
        let mut checked_sec: Vec<String> = Vec::new();
        let mut remaining_trash: Vec<String> = Vec::new();
        for cid in &sg.trash_vec {
            if is_egg(cid) || is_token(cid) {
                continue;
            }
            if !take_one(&mut main_pool, cid) {
                continue;
            }
            let safe_sec_trash = card_data.get(cid).is_some_and(|cd| {
                cd.security_text.trim().is_empty()
                    && (matches!(cd.card_kind, CardKind::Option | CardKind::Tamer)
                        || cd.dp.unwrap_or(0) <= 2000)
            });
            if checked_sec.len() < checks_to_take && safe_sec_trash {
                checked_sec.push(cid.clone());
            } else {
                remaining_trash.push(cid.clone());
            }
        }
        ordered_active.extend(remaining_trash);

        // (d) If opponent needs security chipped AND this seat has no field Digimon in goal,
        // pull one low-cost Lv.3 rookie forward as a sacrificial security attacker.
        let has_field_digimon = sg.field_stacks.iter().any(|tf| {
            tf.stack_ids
                .last()
                .and_then(|id| card_data.get(id))
                .is_some_and(|cd| cd.card_kind == CardKind::Digimon)
        });
        if opp_sg.security_count < 5 && !has_field_digimon {
            if let Some(pos) = main_pool.iter().position(|id| {
                card_data.get(id).is_some_and(|cd| {
                    cd.card_kind == CardKind::Digimon
                        && cd.level == Some(3)
                        && cd.play_cost <= 4
                })
            }) {
                ordered_active.insert(0, main_pool.remove(pos));
            }
        }

        // (e) Required final hand cards!
        let mut target_hand_cards: Vec<String> = Vec::new();
        for cid in &sg.hand_vec {
            if !is_egg(cid) && !is_token(cid) && take_one(&mut main_pool, cid) {
                target_hand_cards.push(cid.clone());
            }
        }
        ordered_active.extend(target_hand_cards);

        // Fill remaining security (`sec_bottom_to_top` + `checked_sec` up to 5) with safe
        // empty-security-text low-DP cards from `main_pool`.
        while sec_bottom_to_top.len() + checked_sec.len() < 5 && !main_pool.is_empty() {
            let best_pos = main_pool
                .iter()
                .enumerate()
                .min_by_key(|(_, id)| {
                    card_data.get(*id).map_or((2, 99_999), |cd| {
                        let has_sec = !cd.security_text.trim().is_empty();
                        let dp_score = match cd.card_kind {
                            CardKind::Option | CardKind::Tamer => 0,
                            _ => cd.dp.unwrap_or(5000).max(0),
                        };
                        (usize::from(has_sec), dp_score)
                    })
                })
                .map(|(idx, _)| idx)
                .unwrap_or(0);
            checked_sec.push(main_pool.remove(best_pos));
        }
        // If the opponent used a sacrificial Lv.3 rookie (empty field in goal) and this seat
        // had security chipped, place one high-DP Digimon at `sec_bottom_to_top[keep_len]`
        // (checked LAST) if available so the sacrificial rookie dies on the final check!
        if sg.security_count < 5 && opp_sg.field_stacks.is_empty() && !checked_sec.is_empty() {
            if let Some(hi_pos) = main_pool.iter().position(|id| {
                card_data.get(id).is_some_and(|cd| {
                    cd.card_kind == CardKind::Digimon
                        && cd.security_text.trim().is_empty()
                        && cd.dp.unwrap_or(0) >= 5000
                })
            }) {
                let hi_card = main_pool.remove(hi_pos);
                let old = std::mem::replace(&mut checked_sec[0], hi_card);
                main_pool.push(old);
            }
        }
        sec_bottom_to_top.extend(checked_sec);

        // Sort remaining `main_pool` so diverse costs / vanilla cards appear early for
        // flexible memory calibration without firing unwanted mandatory effects.
        let mut seen_costs = std::collections::BTreeSet::new();
        let mut diverse_front = Vec::new();
        let mut tail_rest = Vec::new();
        for cid in main_pool {
            let cost = card_data.get(&cid).map(|c| c.play_cost).unwrap_or(3);
            let is_vanilla = card_data
                .get(&cid)
                .is_some_and(|c| c.effect_text.trim().is_empty());
            if seen_costs.insert((cost, !is_vanilla)) && diverse_front.len() < 8 {
                diverse_front.push(cid);
            } else {
                tail_rest.push(cid);
            }
        }
        ordered_active.extend(diverse_front);
        ordered_active.extend(tail_rest);

        // Assemble chronological `draw_stack`:
        // [0..5] = opening hand, [5..10] = security bottom-to-top, [10..] = turn/evo draws!
        let mut draw_stack = Vec::with_capacity(ordered_active.len() + sec_bottom_to_top.len());
        let hand_split = ordered_active.len().min(5);
        draw_stack.extend_from_slice(&ordered_active[..hand_split]);
        draw_stack.extend(sec_bottom_to_top);
        draw_stack.extend_from_slice(&ordered_active[hand_split..]);

        let post_main: Vec<String> = draw_stack.iter().rev().cloned().collect();

        Ok(StackedSeatPlan {
            draw_stack,
            post_main,
            post_egg,
        })
    };

    let plan0 = build_plan(0)?;
    let plan1 = build_plan(1)?;
    let post_mains = [plan0.post_main.clone(), plan1.post_main.clone()];
    let post_eggs = [plan0.post_egg.clone(), plan1.post_egg.clone()];
    let pre_decks = invert_game_new_shuffle(&post_mains, &post_eggs, seed);

    Ok(([plan0, plan1], pre_decks))
}
