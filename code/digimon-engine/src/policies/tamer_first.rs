//! Explicit experimental baselines; original policies remain unchanged.
use crate::{action::explain::{explain_action,ActionKind,ActionZone},
    enums::{CardKind,GamePhase},game::Game};
use super::{current_decision_player,greedy_action};

/// Direct, currently legal Main-phase digivolutions over the acting seat's
/// Tamer. This does not infer an effect sequence from a card's printed text.
pub fn legal_tamer_digivolutions(game:&Game,mask:&[f32])->Vec<u16>{
    if game.current_phase!=GamePhase::Main || game.pending_selection.is_some(){return vec![];}
    let player=current_decision_player(game);
    mask.iter().enumerate().filter_map(|(id,legal)|{
        if *legal<=0.5{return None;}
        let action=explain_action(game,player,id as u16);
        if action.kind!=ActionKind::Digivolve || action.target_zone!=Some(ActionZone::Battle){return None;}
        let target=game.players[player as usize].battle_area.get(action.target_index? as usize)?;
        (game.card_data[target.top_card().data_index].card_kind==CardKind::Tamer).then_some(id as u16)
    }).collect()
}

/// Strict priority, even when evolution passes memory or another move wins.
/// Equal eligible routes use lowest engine action ID, without extra simulation.
pub fn greedy_tamer_first_action(game:&Game,mask:&[f32])->u16{
    legal_tamer_digivolutions(game,mask).into_iter().next().unwrap_or_else(||greedy_action(game,mask))
}
