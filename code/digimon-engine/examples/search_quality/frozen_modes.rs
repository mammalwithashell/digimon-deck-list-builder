//! Information-regime boundary for the NEW frozen two-model evaluator.
//! No deck-index, archetype, true-hand scoring or actor-label input is accepted.
//! This module is preparation only until adapter qualification is complete.
use digimon_engine::{
    card_registry::CardRegistry,
    determinize::{
        beliefs::{BeliefEvidence, WeightedDeckPrior},
        closed::materialize_closed_with_diagnostics,
    },
    game::Game,
    search::{
        decision_player, determinized_search_batched, determinized_search_sampled,
        DeterminizedConfig, PolicyValueFn, SearchResult,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, time::Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InformationRegime {
    KnownList,
    ClosedList,
}

/// The supplied evaluator is always the root actor's frozen model, matching
/// the historical harness's rollout routing. Knowledge comes from that seat's
/// engine-maintained ledger, enabled before any real action by the driver.
pub fn search(
    registry: &CardRegistry,
    game: &Game,
    evaluator: &mut dyn PolicyValueFn,
    config: &DeterminizedConfig,
    regime: InformationRegime,
    prior: Option<&WeightedDeckPrior>,
) -> Result<(SearchResult, Value), String> {
    if config.search.simulations != 320
        || config.worlds != 4
        || config.search.dirichlet_epsilon != 0.0
    {
        return Err(
            "frozen comparison requires 320 simulations, four worlds and zero root noise".into(),
        );
    }
    let viewer = decision_player(game);
    let started = Instant::now();
    let (result, diagnostics) = match regime {
        InformationRegime::KnownList => (
            determinized_search_batched(registry, game, viewer, evaluator, config, 4),
            json!({"regime":"known-list","belief_diagnostics":null}),
        ),
        InformationRegime::ClosedList => {
            let prior = prior.ok_or(
                "closed-list actor requires a frozen prior; exact-list fallback forbidden",
            )?;
            let memory = game.belief_knowledge(viewer)?;
            let evidence = BeliefEvidence::extract(game, viewer, memory)?;
            let evidence_bytes = serde_json::to_vec(&evidence).map_err(|e| e.to_string())?;
            let evidence_hash = format!("{:x}", Sha256::digest(evidence_bytes));
            let posterior = prior.posterior(&evidence)?;
            let mut fallback_worlds = 0;
            let mut sampled_worlds = 0;
            let mut sampling_seconds = 0.;
            let mut sampled_hand_counts = BTreeMap::<String, usize>::new();
            let mut world_seeds = Vec::new();
            let result = determinized_search_sampled(
                registry,
                game,
                viewer,
                evaluator,
                config,
                4,
                &mut |seed| {
                    let sample_start = Instant::now();
                    let (world, fallback) =
                        materialize_closed_with_diagnostics(game, viewer, memory, prior, seed)?;
                    sampling_seconds += sample_start.elapsed().as_secs_f64();
                    sampled_worlds += 1;
                    fallback_worlds += usize::from(fallback);
                    world_seeds.push(seed);
                    // Predictions are sampled hypotheses, never the real hidden hand.
                    for card in &world.players[(1 - viewer) as usize].hand {
                        *sampled_hand_counts
                            .entry(card.card_id(&world.card_data).into())
                            .or_default() += 1;
                    }
                    Ok(world)
                },
            )?;
            let squared_mass = posterior.candidates.iter().map(|(_, p)| p * p).sum::<f64>()
                + posterior.fallback_mass * posterior.fallback_mass;
            (
                result,
                json!({"regime":"closed-list","prior_snapshot_hash":prior.snapshot_hash(),
                "evidence_sha256":evidence_hash,"compatible_candidates":posterior.candidates.len(),
                "fallback_mass":posterior.fallback_mass,"mixture_component_ess":1.0/squared_mass,
                "ess_scope":"Candidate-list components plus one aggregate fallback component, not fallback deck count",
                "sampled_worlds":sampled_worlds,"fallback_worlds":fallback_worlds,
                "world_seeds":world_seeds,"sampling_seconds":sampling_seconds,
                "sampled_opponent_hand_counts":sampled_hand_counts}),
            )
        }
    };
    if result.simulations_run != 320 {
        return Err(format!(
            "actual simulation budget {}, expected320",
            result.simulations_run
        ));
    }
    Ok((
        result,
        json!({"viewer":viewer,"search_seed":config.search.seed,
        "simulations":320,"worlds":4,"batch":4,"seconds":started.elapsed().as_secs_f64(),
        "information":diagnostics}),
    ))
}
