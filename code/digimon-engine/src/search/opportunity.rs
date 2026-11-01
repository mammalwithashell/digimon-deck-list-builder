//! Observer-only complete-plan telemetry. Never drives legality or selection.
use crate::action::explain::{explain_action, ActionKind, ActionZone};
use crate::action::build_action_mask;
use crate::enums::{CardKind, PlayerId};
use crate::game::Game;
use super::{decision_player, SearchResult};
use serde_json::{json, Value};

/// Classify the final commitment, following navigation on a private clone.
/// Effect-mediated evolution opportunities are deliberately not inferred.
pub fn describe(game: &Game, actor: PlayerId, result: &SearchResult, selected: &[u16],
    sampling: &str) -> Result<Value, String> {
    let total: u64 = result.root_plans.iter().map(|(_, n, _)| *n as u64).sum();
    if !result.root_plans.iter().any(|(p, _, _)| p == selected) {
        return Err("selected plan absent from search result".into());
    }
    if !matches!(sampling, "visit_sample_tau1" | "argmax_tau0" | "uniform_zero_visits") {
        return Err("unknown search selection mode".into());
    }
    let best = result.best_plan();
    let mut plans = Vec::new();
    for (route, visits, q) in &result.root_plans {
        if route.is_empty() { return Err("empty root plan".into()); }
        let mut state = game.clone();
        for (index, action) in route.iter().enumerate() {
            let mask = build_action_mask(&state, actor);
            if decision_player(&state) != actor || mask.get(*action as usize).copied().unwrap_or(0.) <= 0.5 {
                return Err("telemetry plan violates live navigation mask".into());
            }
            if index + 1 < route.len() { state.decode_action(*action, actor); }
        }
        let explanation = explain_action(&state, actor, *route.last().unwrap());
        let tamer = explanation.target_zone == Some(ActionZone::Battle)
            && explanation.target_index.and_then(|i| state.players[actor as usize].battle_area.get(i as usize))
                .is_some_and(|p| state.card_data[p.top_card().data_index].card_kind == CardKind::Tamer);
        let category = match explanation.kind {
            ActionKind::Digivolve if tamer => "tamer_digivolve",
            ActionKind::Digivolve if explanation.target_zone == Some(ActionZone::Breeding) => "raising_digivolve",
            ActionKind::Digivolve => "battle_digivolve",
            ActionKind::DnaDigivolve => "dna_digivolve",
            ActionKind::Move => "promotion",
            ActionKind::Play => "hard_play",
            _ => "other",
        };
        let prior = result.root_priors.iter().find(|(p, _, _)| p == route);
        let probability = match sampling {
            "uniform_zero_visits" => 1. / result.root_plans.len() as f64,
            "visit_sample_tau1" if total > 0 => *visits as f64 / total as f64,
            "argmax_tau0" => if best == Some(route.as_slice()) { 1. } else { 0. },
            _ => return Err("visit sampling has no visits".into()),
        };
        plans.push(json!({"plan":route,"category":category,"action":explanation,
            "prior_before_noise":prior.map(|p|p.1),"prior_after_noise":prior.map(|p|p.2),
            "visits":visits,"q":if *visits>0 {Some(*q)} else {None},
            "selection_probability":probability,"selected":route==selected}));
    }
    Ok(json!({"schema":"search-opportunities-v1","actor":actor,"turn":game.turn_count,
        "phase":format!("{:?}",game.current_phase),"sampling":sampling,
        "simulations_run":result.simulations_run,"total_visits":total,"plans":plans,
        "selected_plan":selected,"executed_actions":[],"commitment_executed":false,
        "effect_sequence_opportunities":"unmeasured"}))
}

/// Record execution explicitly; a cap may stop navigation before commitment.
pub fn record_execution(record: &mut Value, executed: &[u16]) -> Result<(), String> {
    let selected: Vec<u16> = serde_json::from_value(record["selected_plan"].clone()).map_err(|e|e.to_string())?;
    if !selected.starts_with(executed) { return Err("execution is not selected plan prefix".into()); }
    record["executed_actions"] = json!(executed);
    record["commitment_executed"] = json!(selected == executed);
    Ok(())
}
