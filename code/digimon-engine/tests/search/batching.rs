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

#[test]
fn macro_substeps_preserves_serial_and_batched_bit_exactness() {
    run_big_stack(|| {
        let data = load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let roots = vec![
            game(&data, 7, 12),
            game(&data, 11, 24),
            game(&data, 21, 18),
            game(&data, 22, 25),
        ];
        for substeps in [1, 4, 16] {
            let configs: Vec<_> = [8, 24, 48, 64]
                .into_iter()
                .enumerate()
                .map(|(i, sims)| SearchConfig {
                    simulations: sims,
                    seed: 901 + i as u64,
                    macro_substeps: substeps,
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
            assert_eq!(actual, expected, "substeps={substeps}");
            assert_eq!(
                serial.batches.iter().sum::<usize>(),
                batched.batches.iter().sum::<usize>(),
                "substeps={substeps}"
            );
        }

        // Also check determinized_search_batched across batch=1,2,4 with macro_substeps=16.
        let mut det_cfg = DeterminizedConfig::default();
        det_cfg.worlds = 4;
        det_cfg.search.simulations = 64;
        det_cfg.search.seed = 2026;
        det_cfg.search.macro_substeps = 16;
        let expected_det = determinized_search_batched(
            &registry,
            &roots[1],
            decision_player(&roots[1]),
            &mut SignalEvaluator::default(),
            &det_cfg,
            1,
        );
        for batch in [2, 4] {
            let actual_det = determinized_search_batched(
                &registry,
                &roots[1],
                decision_player(&roots[1]),
                &mut SignalEvaluator::default(),
                &det_cfg,
                batch,
            );
            assert_eq!(actual_det, expected_det, "det batch={batch}");
        }
    });
}

/// Deterministic mock recurrent evaluator whose policy/value depend on the
/// incoming `RecurrentState`, proving that serial and batched MCTS thread
/// per-player hidden states identically down every tree path.
#[derive(Default)]
struct MockRecurrentEvaluator {
    root: [Option<digimon_engine::search::RecurrentState>; 2],
    batches: Vec<usize>,
    non_none_inputs: usize,
}

impl PolicyValueFn for MockRecurrentEvaluator {
    fn evaluate_batch(&mut self, requests: &[EvalRequest]) -> Vec<Evaluation> {
        let states = vec![None; requests.len()];
        self.evaluate_batch_recurrent(requests, &states).0
    }

    fn evaluate_batch_recurrent(
        &mut self,
        requests: &[EvalRequest],
        states_in: &[Option<digimon_engine::search::RecurrentState>],
    ) -> (
        Vec<Evaluation>,
        Vec<Option<digimon_engine::search::RecurrentState>>,
    ) {
        self.batches.push(requests.len());
        let mut evals = Vec::with_capacity(requests.len());
        let mut nexts = Vec::with_capacity(requests.len());
        for (req, st) in requests.iter().zip(states_in) {
            if st.is_some() {
                self.non_none_inputs += 1;
            }
            let h_in = st.as_ref().map(|s| s.h_pi[0]).unwrap_or(0.0);
            let legal: Vec<usize> = req
                .mask
                .iter()
                .enumerate()
                .filter(|(_, &m)| m > 0.5)
                .map(|(i, _)| i)
                .collect();
            let mut weights = Vec::with_capacity(legal.len());
            let mut sum = 0.0f32;
            for (rank, &a) in legal.iter().enumerate() {
                let w = 1.0 + ((a % 7) as f32) * 0.1 + (h_in * (rank as f32 + 1.0)).sin().abs() * 0.25;
                weights.push((a, w));
                sum += w;
            }
            let mut policy = vec![0.0f32; req.mask.len()];
            for (a, w) in weights {
                policy[a] = w / sum;
            }
            let obs_sum: f32 = req.obs.iter().take(16).sum();
            let next_h = (h_in * 0.7 + obs_sum * 0.05 + 0.13).tanh();
            let value = (next_h * 0.5).clamp(-0.95, 0.95);
            evals.push(Evaluation { policy, value });
            nexts.push(Some(digimon_engine::search::RecurrentState {
                h_pi: vec![next_h; 4],
                c_pi: vec![next_h * 0.5; 4],
                h_vf: vec![next_h * -0.5; 4],
                c_vf: vec![next_h * 0.25; 4],
            }));
        }
        (evals, nexts)
    }

    fn is_recurrent(&self) -> bool {
        true
    }

    fn root_recurrent_state(
        &self,
        player: digimon_engine::enums::PlayerId,
    ) -> Option<digimon_engine::search::RecurrentState> {
        self.root.get(player as usize).and_then(|s| s.clone())
    }
}

#[test]
fn recurrent_evaluator_threads_state_with_serial_and_batched_parity() {
    run_big_stack(|| {
        let data = load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let roots = vec![game(&data, 19, 12), game(&data, 23, 24)];
        let configs: Vec<SearchConfig> = (0..roots.len())
            .map(|i| SearchConfig {
                simulations: 48,
                seed: 1200 + i as u64,
                macro_substeps: 8,
                macro_substep_min_prior: 0.85,
                ..Default::default()
            })
            .collect();
        let seed_state = digimon_engine::search::RecurrentState {
            h_pi: vec![0.42; 4],
            c_pi: vec![0.11; 4],
            h_vf: vec![-0.25; 4],
            c_vf: vec![0.07; 4],
        };
        let mcts = Mcts::new(&registry);
        let mut serial = MockRecurrentEvaluator {
            root: [Some(seed_state.clone()), Some(seed_state.clone())],
            ..Default::default()
        };
        let expected: Vec<_> = roots
            .iter()
            .zip(&configs)
            .map(|(r, c)| mcts.search(r, &mut serial, c))
            .collect();
        let mut batched = MockRecurrentEvaluator {
            root: [Some(seed_state.clone()), Some(seed_state)],
            ..Default::default()
        };
        let actual = mcts.search_batch(&roots, &mut batched, &configs);
        assert_eq!(actual, expected);
        assert!(batched.non_none_inputs > roots.len());
        assert_eq!(
            serial.batches.iter().sum::<usize>(),
            batched.batches.iter().sum::<usize>()
        );
    });
}

#[test]
fn macro_substep_min_prior_gates_and_preserves_parity() {
    run_big_stack(|| {
        let data = load_card_data();
        let registry = CardRegistry::from_cards(&data);
        let root = game(&data, 17, 18);
        let mcts = Mcts::new(&registry);
        let cfg_zero = SearchConfig {
            simulations: 40,
            seed: 555,
            macro_substeps: 0,
            macro_substep_min_prior: 0.0,
            ..Default::default()
        };
        let cfg_strict = SearchConfig {
            simulations: 40,
            seed: 555,
            macro_substeps: 16,
            macro_substep_min_prior: 1.0,
            ..Default::default()
        };
        // With min_prior=1.0 on a non-degenerate evaluator, no multi-option
        // pending selection has top prior >= 1.0, so search matches macro_substeps=0.
        let r0 = mcts.search(&root, &mut SignalEvaluator::default(), &cfg_zero);
        let r_strict = mcts.search(&root, &mut SignalEvaluator::default(), &cfg_strict);
        assert_eq!(r0, r_strict);
    });
}

