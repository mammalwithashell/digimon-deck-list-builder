//! Independent trees batch inference, never speculative simulations in one tree.
use crate::support::{advance_random, load_card_data, run_big_stack, Lcg, ST1_DECK, ST5_DECK};
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::game::Game;
use digimon_engine::runners::HeadlessRunner;
use digimon_engine::search::{
    decision_player, determinized_search, determinized_search_batched, DeterminizationMode,
    DeterminizedConfig, EvalRequest, Evaluation, Mcts, PolicyValueFn, SearchConfig,
};

#[derive(Default)]
struct SignalEvaluator {
    batches: Vec<usize>,
}
impl PolicyValueFn for SignalEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        self.batches.push(requests.len());
        requests
            .iter()
            .map(|r| {
                let signal: f32 = r.obs.iter().step_by(97).map(|x| x * 0.0001).sum();
                let mut policy: Vec<f32> = r
                    .mask
                    .iter()
                    .enumerate()
                    .map(|(i, m)| if *m > 0.5 { 1. + (i % 13) as f32 } else { 0. })
                    .collect();
                let mass: f32 = policy.iter().sum();
                for p in &mut policy {
                    *p /= mass;
                }
                Evaluation {
                    policy,
                    value: signal.tanh(),
                }
            })
            .collect()
    }
}

fn game(
    data: &std::collections::HashMap<String, digimon_engine::card_data::CardData>,
    seed: u64,
    steps: u32,
) -> Game {
    let owned = |d: &[&str]| d.iter().map(|s| s.to_string()).collect();
    let mut game = HeadlessRunner::new(
        owned(ST1_DECK),
        owned(ST5_DECK),
        data,
        false,
        false,
        false,
        Some(seed),
    )
    .unwrap()
    .game;
    advance_random(&mut game, &mut Lcg(seed ^ 0x9E3779B97F4A7C15), steps);
    assert!(!game.game_over);
    game
}

#[test]
fn batched_mcts_matches_nonuniform_serial_search_with_mixed_budgets_and_caps() {
    run_big_stack(|| {
        let data = load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let roots = vec![
            game(&data, 7, 0),
            game(&data, 11, 24),
            game(&data, 21, 10),
            game(&data, 22, 25),
        ];
        for depth in [0, 1, 512] {
            let configs: Vec<_> = [0, 1, 17, 80]
                .into_iter()
                .enumerate()
                .map(|(i, sims)| SearchConfig {
                    simulations: sims,
                    seed: 81 + i as u64,
                    max_decisions: depth,
                    ..Default::default()
                })
                .collect();
            let mcts = Mcts::new(&registry);
            let mut serial = SignalEvaluator::default();
            let expected: Vec<_> = roots
                .iter()
                .zip(&configs)
                .map(|(r, c)| mcts.search(r, &mut serial, c))
                .collect();
            let mut batched = SignalEvaluator::default();
            let actual = mcts.search_batch(&roots, &mut batched, &configs);
            assert_eq!(actual, expected);
            assert_eq!(
                serial.batches.iter().sum::<usize>(),
                batched.batches.iter().sum::<usize>()
            );
            assert!(batched.batches.iter().any(|&n| n > 1));
        }
    });
}

#[test]
fn batched_pimc_preserves_remainders_seeds_and_partial_batches() {
    run_big_stack(|| {
        let data = load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let root = game(&data, 11, 24);
        for (sims, worlds) in [(0, 8), (1, 8), (17, 4), (19, 3), (320, 4)] {
            let mut config = DeterminizedConfig::default();
            config.worlds = worlds;
            config.search.simulations = sims;
            config.search.seed = 481;
            let mut serial = SignalEvaluator::default();
            let expected = determinized_search(
                &registry,
                &root,
                decision_player(&root),
                &mut serial,
                &config,
            );
            for batch in [1, 2, 4, 8] {
                let mut evaluator = SignalEvaluator::default();
                let actual = determinized_search_batched(
                    &registry,
                    &root,
                    decision_player(&root),
                    &mut evaluator,
                    &config,
                    batch,
                );
                assert_eq!(
                    actual, expected,
                    "sims={sims}, worlds={worlds}, batch={batch}"
                );
                assert_eq!(
                    serial.batches.iter().sum::<usize>(),
                    evaluator.batches.iter().sum::<usize>()
                );
                assert!(evaluator.batches.iter().all(|&n| n <= batch));
                if sims == 320 && batch == 4 {
                    assert!(evaluator.batches.contains(&4));
                }
            }
        }
    });
}

#[test]
fn terminal_and_live_roots_share_a_batch_without_evaluating_terminals() {
    run_big_stack(|| {
        let data = load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let live = game(&data, 7, 0);
        let mut terminal = live.clone();
        terminal.game_over = true;
        terminal.winner = Some(0);
        let roots = vec![terminal, live];
        let configs = vec![
            SearchConfig {
                simulations: 17,
                ..Default::default()
            };
            2
        ];
        let mut serial = SignalEvaluator::default();
        let mcts = Mcts::new(&registry);
        let expected: Vec<_> = roots
            .iter()
            .zip(&configs)
            .map(|(r, c)| mcts.search(r, &mut serial, c))
            .collect();
        let mut evaluator = SignalEvaluator::default();
        assert_eq!(
            mcts.search_batch(&roots, &mut evaluator, &configs),
            expected
        );
        assert_eq!(
            evaluator.batches.iter().sum::<usize>(),
            serial.batches.iter().sum::<usize>()
        );
        assert!(evaluator.batches.iter().all(|&n| n == 1));
        assert!(mcts.search_batch(&[], &mut evaluator, &[]).is_empty());
    });
}

#[test]
fn batching_preserves_hidden_permutation_and_leaves_ismcts_serial() {
    run_big_stack(|| {
        let data = load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let root = game(&data, 11, 24);
        let viewer = decision_player(&root);
        let mut permuted = root.clone();
        permuted.players[(1 - viewer) as usize].hand.reverse();
        permuted.players[(1 - viewer) as usize].deck.reverse();
        let mut config = DeterminizedConfig::default();
        config.worlds = 4;
        config.search.simulations = 80;
        let a = determinized_search_batched(
            &registry,
            &root,
            viewer,
            &mut SignalEvaluator::default(),
            &config,
            4,
        );
        let b = determinized_search_batched(
            &registry,
            &permuted,
            viewer,
            &mut SignalEvaluator::default(),
            &config,
            4,
        );
        assert_eq!(a, b);
        config.mode = DeterminizationMode::IsMcts;
        let mut evaluator = SignalEvaluator::default();
        assert_eq!(
            determinized_search_batched(&registry, &root, viewer, &mut evaluator, &config, 4),
            determinized_search(
                &registry,
                &root,
                viewer,
                &mut SignalEvaluator::default(),
                &config
            )
        );
        assert!(evaluator.batches.iter().all(|&n| n == 1));
    });
}
