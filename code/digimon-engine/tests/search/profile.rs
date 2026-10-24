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

#[test]
fn exact_input_memoization_preserves_search_results() {
    use digimon_engine::search::{
        decision_player, determinized_search, DeterminizedConfig, MemoizedEvaluator,
        UniformEvaluator,
    };
    run_big_stack(|| {
        let card_data = load_card_data();
        let registry = CardRegistry::from_cards(&card_data);
        let game = fresh_game(&card_data);
        let mut cached = MemoizedEvaluator::new(UniformEvaluator, 128);
        for seed in [7, 19, 41] {
            let config = SearchConfig {
                simulations: 32,
                seed,
                observation_profile: ObservationProfileId::StandardLiteV2,
                ..SearchConfig::default()
            };
            let search = Mcts::new(&registry);
            let expected = search.search(&game, &mut UniformEvaluator, &config);
            assert_eq!(search.search(&game, &mut cached, &config), expected);
            assert_eq!(search.search(&game, &mut cached, &config), expected);
            let config = DeterminizedConfig {
                worlds: 4,
                search: config,
                ..Default::default()
            };
            let expected = determinized_search(
                &registry,
                &game,
                decision_player(&game),
                &mut UniformEvaluator,
                &config,
            );
            assert_eq!(
                determinized_search(
                    &registry,
                    &game,
                    decision_player(&game),
                    &mut cached,
                    &config
                ),
                expected
            );
        }
        assert!(cached.hits() > 0);
    });
}
