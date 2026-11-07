//! Shared two-game execution and final publication boundary.
//! Scored startup validation remains in frozen_checkpoint_modes.
//! Diagnostic callers use a distinct schema; this is not an admission API.
use super::{modes, telemetry};
use digimon_engine::{
    action::build_action_mask, card_registry::CardRegistry,
    determinize::beliefs::WeightedDeckPrior, game::Game,
    observation::ObservationProfileId,
    search::{decision_player, DeterminizedConfig, PolicyValueFn},
};
use serde_json::{json, Value};
use std::{path::Path, time::Instant};

pub fn write(path: &Path, value: &Value) {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    std::fs::rename(tmp, path).unwrap();
}
pub fn num(v: &Value, key: &str) -> u64 {
    v[key].as_u64().unwrap_or_else(|| panic!("missing {key}"))
}
#[allow(clippy::too_many_arguments)]
pub fn run_pair(
    out: &Path, request: &Value, plan: &Value, runtime_hash: &str,
    prior: Option<&WeightedDeckPrior>, prior_snapshot_hash: &str,
    registry: &CardRegistry,
    candidate: &mut dyn PolicyValueFn, opponent: &mut dyn PolicyValueFn,
    candidate_mode: modes::InformationRegime, opponent_mode: modes::InformationRegime,
    cap: u64, schema: &str, mut new_game: impl FnMut(u8) -> Game,
) {
    let mut games = Vec::new();
    for candidate_seat in 0..2u8 {
        let mut game = new_game(candidate_seat);
        // Both seats track the same real event history from before any action;
        // each actor queries only its own independent ledger.
        game.enable_belief_knowledge().unwrap();
        game.turn_count = 1;
        let first_player = game.turn_player();
        let initial_ui = telemetry::snapshot(&game);
        game.drain_events();
        let started = Instant::now();
        let mut actions = Vec::new();
        let mut steps = Vec::new();
        let mut searches = Vec::new();
        while !game.game_over && (actions.len() as u64) < cap {
            let pid = decision_player(&game);
            let mask = build_action_mask(&game, pid);
            let valid: Vec<u16> = mask
                .iter()
                .enumerate()
                .filter_map(|(a, m)| (*m > 0.5).then_some(a as u16))
                .collect();
            assert!(!valid.is_empty());
            let t = Instant::now();
            let route = if valid.len() == 1 {
                vec![valid[0]]
            } else {
                let mode = if pid == candidate_seat {
                    candidate_mode
                } else {
                    opponent_mode
                };
                let mut config = DeterminizedConfig::default();
                config.worlds = 4;
                config.search.simulations = 320;
                config.search.dirichlet_epsilon = 0.;
                config.search.observation_profile = ObservationProfileId::StandardLiteV2;
                config.search.seed = num(&request, "search_seed")
                    .wrapping_add((actions.len() as u64).wrapping_mul(0x9e3779b97f4a7c15))
                    .wrapping_add(pid as u64);
                write(
                    &out.with_extension("active.json"),
                    &json!({"request":request,"candidate_seat":candidate_seat,"decision":actions.len(),"viewer":pid,"search_seed":config.search.seed,"executed_actions":actions}),
                );
                let eval: &mut dyn PolicyValueFn = if pid == candidate_seat {
                    &mut *candidate
                } else {
                    &mut *opponent
                };
                let (result, info) = modes::search(
                    &registry,
                    &game,
                    eval,
                    &config,
                    mode,
                    prior,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "request {} seat {candidate_seat} decision {} viewer {pid}: {error}",
                        request["id"],
                        actions.len()
                    )
                });
                let selected = result.best_plan().unwrap().to_vec();
                let opportunity = digimon_engine::search::opportunity::describe(&game,pid,&result,&selected,"argmax_tau0").unwrap();
                searches.push(json!({"decision":actions.len(),"selected_plan":selected,"root_plans":result.root_plans,"diagnostics":info,"opportunity":opportunity}));
                selected
            };
            let selection_seconds = t.elapsed().as_secs_f64();
            let route_start = actions.len();
            for (ix, &action) in route.iter().enumerate() {
                assert!(
                    !game.game_over && actions.len() < (cap as usize),
                    "route exhausted cap before commitment"
                );
                assert_eq!(
                    decision_player(&game),
                    pid,
                    "route changed actor before commitment"
                );
                let next_mask = build_action_mask(&game, pid);
                assert!(next_mask[action as usize] > 0.5, "route no longer legal");
                let phase = format!("{:?}", game.current_phase);
                let legal: Vec<_> = next_mask
                    .iter()
                    .enumerate()
                    .filter_map(|(a, m)| (*m > 0.5).then_some(a))
                    .collect();
                game.decode_action(action, pid);
                actions.push(action);
                steps.push(json!({"mover":pid+1,"phase":phase,"legal_actions":legal,"action":action,
                    "selection_seconds":if ix==0 {selection_seconds} else {0.},"after_ui":telemetry::snapshot(&game),
                    "turn_count":game.turn_count,"events":game.drain_events()}));
            }
            if let Some(search) = searches.last_mut().filter(|s|s["decision"].as_u64()==Some(route_start as u64)) {
                digimon_engine::search::opportunity::record_execution(&mut search["opportunity"],&actions[route_start..]).unwrap();
            }
            if actions.len() % 20 == 0 {
                write(
                    &out.with_extension("progress.json"),
                    &json!({"candidate_seat":candidate_seat,"committed_decisions":actions.len(),"seconds":started.elapsed().as_secs_f64()}),
                );
            }
        }
        let truncated = !game.game_over
            || matches!(
                game.terminal_outcome_reason,
                Some(
                    digimon_engine::game::TerminalOutcomeReason::StepLimit
                        | digimon_engine::game::TerminalOutcomeReason::Crash
                )
            );
        if truncated {
            write(
                &out.with_extension("failure.json"),
                &json!({"request":request,"candidate_seat":candidate_seat,"error":"incomplete game","actions":actions,"terminal_reason":format!("{:?}",game.terminal_outcome_reason)}),
            );
            panic!("incomplete game; no completed score");
        }
        let score = match game.winner {
            None => 0.5,
            Some(w) if w == candidate_seat => 1.,
            _ => 0.,
        };
        games.push(json!({"candidate_seat":candidate_seat,"first_player":first_player,"winner":game.winner,"truncated":false,"score":score,
            "decisions":actions.len(),"searched":searches.len(),"actions":actions,"search_diagnostics":searches,"seconds":started.elapsed().as_secs_f64(),
            "terminal_reason":format!("{:?}",game.terminal_outcome_reason),"card_metrics":{"schema":"native-card-eval-v1","event_version":1,"initial_ui":initial_ui,"steps":steps}}));
    }
    write(
        out,
        &json!({"schema":schema,"request":request,"runtime_sha256":runtime_hash,"prior_snapshot_hash":prior_snapshot_hash,
        "model_sha256":plan["model_sha256"],"opponent_model_sha256":plan["opponent_model_sha256"],"games":games}),
    );
}
