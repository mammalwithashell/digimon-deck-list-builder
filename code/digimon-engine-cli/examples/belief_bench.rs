//! Restricted-input prior microbenchmark; no live opponent state is supplied.
use digimon_engine::determinize::beliefs::{BeliefEvidence, PriorSnapshot, WeightedDeckPrior};
use std::{collections::BTreeMap, time::Instant};
fn main() {
    let path = std::env::args().nth(1).expect("prior.json argument");
    let prior = WeightedDeckPrior::new(
        serde_json::from_slice::<PriorSnapshot>(&std::fs::read(path).unwrap()).unwrap(),
    )
    .unwrap();
    let repetitions = 1000;
    let initial = BeliefEvidence::from_counts(0, 1, 5, 40, 5, 4, BTreeMap::new()).unwrap();
    let unseen = prior
        .snapshot()
        .population
        .iter()
        .find(|id| {
            !prior
                .snapshot()
                .candidates
                .iter()
                .any(|d| d.cards.contains(id))
                && digimon_engine::deck_tools::card_database()
                    .get(*id)
                    .is_some_and(|c| c.card_kind != digimon_engine::enums::CardKind::DigiEgg as u8)
                && digimon_engine::deck_tools::card_legality_for_descriptor(
                    id,
                    digimon_engine::format::descriptor("standard").unwrap(),
                )
                .max_copies
                    > 0
        })
        .expect("unfamiliar legal main-deck ID");
    let mut unfamiliar = initial.clone();
    unfamiliar.accounted.insert(unseen.clone(), 1);
    unfamiliar.deck_count -= 1;
    let mut rows = Vec::new();
    for (name, evidence) in [("initial", initial), ("unfamiliar", unfamiliar)] {
        let started = Instant::now();
        for _ in 0..repetitions {
            std::hint::black_box(prior.posterior(&evidence).unwrap());
        }
        let conditioning_seconds = started.elapsed().as_secs_f64();
        let started = Instant::now();
        let mut fallback = 0;
        let mut failed = 0;
        for seed in 0..repetitions {
            match prior.sample(&evidence, seed) {
                Ok(sample) => fallback += usize::from(sample.used_fallback),
                Err(_) => failed += 1,
            }
        }
        rows.push(serde_json::json!({"case":name,"repetitions":repetitions,"compatible_candidates":prior.posterior(&evidence).unwrap().candidates.len(),
            "conditioning_seconds":conditioning_seconds,"sampling_seconds":started.elapsed().as_secs_f64(),
            "fallback_successes":fallback,"sample_failures":failed,"fallback_attempt_limit":prior.snapshot().fallback_attempts}));
    }
    println!(
        "{}",
        serde_json::json!({"prior_hash":prior.snapshot_hash(),"candidates":prior.candidate_count(),
        "population":prior.snapshot().population.len(),"scope":"request success, not per-proposal acceptance; legality is not plausibility","cases":rows})
    );
}
