//! Explicit experimental baselines; original policies remain unchanged.
use crate::{
    action::{
        explain::{explain_action, ActionKind, ActionZone},
        space::{PLAY_HAND_END, PLAY_HAND_START},
    },
    enums::{CardKind, GamePhase, PlayerId},
    game::Game,
    game_actions::CostReductionKind,
};
use super::{current_decision_player, greedy::greedy_action_with_options, greedy_action};

/// Threshold for opposing memory handed over by hard-playing an unreduced Lv.5/Lv.6
/// Digimon onto an empty opponent battle area.
pub const EMPTY_BOARD_HIGH_LEVEL_DUMP_MEMORY_THRESHOLD: i32 = 6;

fn relative_memory(game: &Game, player: PlayerId) -> i32 {
    if game.turn_player() == player {
        i32::from(game.memory)
    } else {
        -i32::from(game.memory)
    }
}

/// Effective play cost of a hand card after declarable passive/self cost reducers.
pub fn effective_hand_play_cost(game: &Game, player: PlayerId, hand_idx: usize) -> Option<i32> {
    let card = game.player(player).hand.get(hand_idx)?;
    let base = i32::from(card.play_cost(&game.card_data));
    let reduction = game.declarable_cost_reduction(player, card.handle(), CostReductionKind::Play);
    Some((base - reduction).max(0))
}

/// Returns `true` when `action` hard-plays a Lv.5 or Lv.6 Digimon from hand onto
/// a completely empty opponent battle area while handing the opponent `>= 6` memory
/// (after declarable cost reductions).
pub fn is_empty_board_high_level_dump(game: &Game, player: PlayerId, action: u16) -> bool {
    if game.current_phase != GamePhase::Main || game.pending_selection.is_some() {
        return false;
    }
    if !(PLAY_HAND_START..PLAY_HAND_END).contains(&action) {
        return false;
    }
    if !game.player(1 - player).battle_area.is_empty() {
        return false;
    }
    let hand_idx = (action - PLAY_HAND_START) as usize;
    let Some(card) = game.player(player).hand.get(hand_idx) else {
        return false;
    };
    let data = &game.card_data[card.data_index];
    if data.card_kind != CardKind::Digimon || !matches!(data.level, Some(5) | Some(6)) {
        return false;
    }
    let Some(eff_cost) = effective_hand_play_cost(game, player, hand_idx) else {
        return false;
    };
    let opp_memory_given = eff_cost - relative_memory(game, player);
    opp_memory_given >= EMPTY_BOARD_HIGH_LEVEL_DUMP_MEMORY_THRESHOLD
}

/// Direct, currently legal Main-phase digivolutions over the acting seat's
/// Tamer. This does not infer an effect sequence from a card's printed text.
pub fn legal_tamer_digivolutions(game: &Game, mask: &[f32]) -> Vec<u16> {
    if game.current_phase != GamePhase::Main || game.pending_selection.is_some() {
        return vec![];
    }
    let player = current_decision_player(game);
    mask.iter()
        .enumerate()
        .filter_map(|(id, legal)| {
            if *legal <= 0.5 {
                return None;
            }
            let action = explain_action(game, player, id as u16);
            if action.kind != ActionKind::Digivolve || action.target_zone != Some(ActionZone::Battle) {
                return None;
            }
            let target = game.players[player as usize]
                .battle_area
                .get(action.target_index? as usize)?;
            (game.card_data[target.top_card().data_index].card_kind == CardKind::Tamer)
                .then_some(id as u16)
        })
        .collect()
}

/// Legal Main-phase plays of Tamer cards from hand that do not memory-dump (`<= 5` opp memory).
/// Sorted to prefer Tamers with inherited effects (Spirit/Hybrid tamers), then lower effective
/// cost, then lowest hand index.
pub fn legal_tamer_plays(game: &Game, mask: &[f32]) -> Vec<u16> {
    if game.current_phase != GamePhase::Main || game.pending_selection.is_some() {
        return vec![];
    }
    let player = current_decision_player(game);
    let rel_mem = relative_memory(game, player);
    let mut scored: Vec<((i32, i32, i32), u16)> = mask
        .iter()
        .enumerate()
        .filter_map(|(id, legal)| {
            if *legal <= 0.5 {
                return None;
            }
            let a = id as u16;
            if !(PLAY_HAND_START..PLAY_HAND_END).contains(&a) {
                return None;
            }
            let hand_idx = (a - PLAY_HAND_START) as usize;
            let card = game.player(player).hand.get(hand_idx)?;
            let data = &game.card_data[card.data_index];
            if data.card_kind != CardKind::Tamer {
                return None;
            }
            let eff_cost = effective_hand_play_cost(game, player, hand_idx)?;
            if eff_cost - rel_mem > 5 {
                return None;
            }
            let has_inh = i32::from(!data.inherited_text.trim().is_empty());
            Some(((has_inh, -eff_cost, -(hand_idx as i32)), a))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, a)| a).collect()
}

/// Returns `true` if the card is a Hybrid/Ten Warriors support Option (e.g. BT18-097
/// *Dark to Light, Thunder to Gunfire*, BT21-091 *Spirit Evolution!*).
pub fn is_hybrid_support_option(data: &crate::card_data::CardData) -> bool {
    if data.card_kind != CardKind::Option {
        return false;
    }
    if matches!(data.card_id.as_str(), "BT18-097" | "BT21-091") {
        return true;
    }
    data.traits.iter().any(|t| {
        let tl = t.to_ascii_lowercase();
        tl == "hybrid" || tl == "ten warriors" || tl == "tenwarriors"
    })
}

/// Legal Main-phase plays of Hybrid support Options (`<= 5` opp memory given).
pub fn legal_hybrid_support_option_plays(game: &Game, mask: &[f32]) -> Vec<u16> {
    if game.current_phase != GamePhase::Main || game.pending_selection.is_some() {
        return vec![];
    }
    let player = current_decision_player(game);
    let rel_mem = relative_memory(game, player);
    mask.iter()
        .enumerate()
        .filter_map(|(id, legal)| {
            if *legal <= 0.5 {
                return None;
            }
            let a = id as u16;
            if !(PLAY_HAND_START..PLAY_HAND_END).contains(&a) {
                return None;
            }
            let hand_idx = (a - PLAY_HAND_START) as usize;
            let card = game.player(player).hand.get(hand_idx)?;
            let data = &game.card_data[card.data_index];
            if !is_hybrid_support_option(data) {
                return None;
            }
            let eff_cost = effective_hand_play_cost(game, player, hand_idx)?;
            (eff_cost - rel_mem <= 5).then_some(a)
        })
        .collect()
}

/// Count of Tamers currently in `player`'s battle area.
pub fn own_tamer_count(game: &Game, player: PlayerId) -> usize {
    game.player(player)
        .battle_area
        .iter()
        .filter(|p| game.card_data[p.top_card().data_index].card_kind == CardKind::Tamer)
        .count()
}

/// Strict priority, even when evolution passes memory or another move wins.
/// Equal eligible routes use lowest engine action ID, without extra simulation.
pub fn greedy_tamer_first_action(game: &Game, mask: &[f32]) -> u16 {
    legal_tamer_digivolutions(game, mask)
        .into_iter()
        .next()
        .unwrap_or_else(|| greedy_action(game, mask))
}

/// Greedy v2: identical to `greedy_action` except it refuses to hard-play an unreduced
/// Lv.5 or Lv.6 Digimon onto an empty opponent battle area for `>= 6` opponent memory.
pub fn greedy_v2_action(game: &Game, mask: &[f32]) -> u16 {
    greedy_action_with_options(game, mask, true)
}

/// Hybrid-pilot greedy v2:
/// 1. Digivolves over an on-board Tamer when legal,
/// 2. Plays Tamers from hand (preferring inherited-effect Tamers) when `< 2` Tamers are on board,
/// 3. Plays Hybrid support Options (e.g. `BT18-097`, `BT21-091`) when affordable,
/// 4. Falls back to `greedy_v2_action` (with the empty-board Lv.5/Lv.6 memory-dump guard).
pub fn greedy_hybrid_v2_action(game: &Game, mask: &[f32]) -> u16 {
    if let Some(a) = legal_tamer_digivolutions(game, mask).into_iter().next() {
        return a;
    }
    let player = current_decision_player(game);
    if own_tamer_count(game, player) < 2 {
        if let Some(a) = legal_tamer_plays(game, mask).into_iter().next() {
            return a;
        }
    }
    if let Some(a) = legal_hybrid_support_option_plays(game, mask).into_iter().next() {
        return a;
    }
    greedy_v2_action(game, mask)
}
