//! Observation-profile plumbing: the search must feed the evaluator tensors
//! in the profile the config names (an exported model's `.meta.json` records
//! its `observation_profile`; a mismatch would silently feed a net garbage).

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::observation::{observation_layout, ObservationProfileId};
use digimon_engine::runners::HeadlessRunner;
use digimon_engine::search::{EvalRequest, Evaluation, Mcts, PolicyValueFn, SearchConfig};

use crate::support::{load_card_data, run_big_stack, ST1_DECK, ST5_DECK};

/// Uniform-policy evaluator that records the obs width of every request.
struct WidthRecordingEvaluator {
    widths: Vec<usize>,
}

impl PolicyValueFn for WidthRecordingEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        requests
            .iter()
            .map(|r| {
                self.widths.push(r.obs.len());
                let legal: Vec<usize> = r
                    .mask
                    .iter()
                    .enumerate()
                    .filter(|(_, &m)| m > 0.5)
                    .map(|(i, _)| i)
                    .collect();
                let mut policy = vec![0.0f32; r.mask.len()];
                let p = 1.0 / legal.len().max(1) as f32;
                for i in legal {
                    policy[i] = p;
                }
                Evaluation { policy, value: 0.0 }
            })
            .collect()
    }
}

fn fresh_game(card_data: &HashMap<String, CardData>) -> digimon_engine::game::Game {
    let owned = |d: &[&str]| -> Vec<String> { d.iter().map(|s| s.to_string()).collect() };
    HeadlessRunner::new(
        owned(ST1_DECK),
        owned(ST5_DECK),
        card_data,
        false,
        false,
        false,
        Some(7),
    )
    .expect("build")
    .game
}

#[test]
fn search_builds_observations_in_the_configured_profile() {
    run_big_stack(|| {
        let card_data = load_card_data();
        let registry = CardRegistry::from_cards(&card_data);
        let game = fresh_game(&card_data);

        for profile in [
            ObservationProfileId::StandardLiteDeckV2, // the default
            ObservationProfileId::StandardCompactV1,  // a differently-sized profile
        ] {
            let expected = observation_layout(profile).tensor_size;
            let mut evaluator = WidthRecordingEvaluator { widths: Vec::new() };
            let config = SearchConfig {
                simulations: 12,
                observation_profile: profile,
                ..SearchConfig::default()
            };
            let result = Mcts::new(&registry).search(&game, &mut evaluator, &config);
            assert!(result.simulations_run > 0);
            assert!(
                !evaluator.widths.is_empty(),
                "search never called the evaluator"
            );
            for w in &evaluator.widths {
                assert_eq!(
                    *w, expected,
                    "profile {:?}: evaluator received obs width {} != layout size {}",
                    profile, w, expected
                );
            }
        }
    });
}
